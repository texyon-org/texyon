use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    io::Cursor,
};

use quick_xml::{
    events::{BytesStart, Event},
    Reader, Writer,
};

use crate::{FrameId, PageSize, PageSvgPayload, ServerEvent, SourceRevision};

/// Sanitized retained state for one persistent viewer page slot.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewerPage {
    /// Current page dimensions.
    pub size: PageSize,
    /// Payload installed for a page inside the prefetch range.
    pub displayed_svg: Option<String>,
    /// Newest dirty payload retained while the page is off screen.
    pub pending_svg: Option<String>,
}

/// Atomic SVG-only viewer reducer state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewerState {
    last_frame_id: Option<FrameId>,
    source_revision: Option<SourceRevision>,
    pages: Vec<ViewerPage>,
    visible: Option<(usize, usize)>,
    pending_acks: VecDeque<FrameId>,
}

impl ViewerState {
    /// Last coherently retained frame identity.
    pub fn last_frame_id(&self) -> Option<FrameId> {
        self.last_frame_id
    }

    /// Last coherently retained source revision.
    pub fn source_revision(&self) -> Option<SourceRevision> {
        self.source_revision
    }

    /// Persistent page slots in document order.
    pub fn pages(&self) -> &[ViewerPage] {
        &self.pages
    }

    /// True visible range, distinct from the prefetch band.
    pub fn visible(&self) -> Option<(usize, usize)> {
        self.visible
    }

    /// Update true visibility and install pending SVGs within ±1 prefetch.
    pub fn set_visible(&mut self, first: usize, last: usize) -> Result<Vec<usize>, ViewerError> {
        if first > last {
            return Err(ViewerError::InvalidPatch(
                "viewport begins after it ends".into(),
            ));
        }
        self.visible = Some((first, last));
        let mut installed = Vec::new();
        let prefetch_first = first.saturating_sub(1);
        let prefetch_last = last.saturating_add(1);
        for (page_index, page) in self.pages.iter_mut().enumerate() {
            if (prefetch_first..=prefetch_last).contains(&page_index) {
                if let Some(svg) = page.pending_svg.take() {
                    page.displayed_svg = Some(svg);
                    installed.push(page_index);
                }
            }
        }
        Ok(installed)
    }

    /// Atomically validate and retain one update event.
    ///
    /// Non-update events are rejected. No field is committed unless every
    /// counter, shape, payload, and SVG security check succeeds.
    pub fn apply(&mut self, event: &ServerEvent) -> Result<(), ViewerError> {
        let ServerEvent::Update {
            frame_id,
            source_revision,
            page_count,
            changed,
            removed,
            pages_svg,
            page_sizes,
            ..
        } = event
        else {
            return Err(ViewerError::NotAnUpdate);
        };

        if self.last_frame_id.is_some_and(|last| *frame_id <= last) {
            return Err(ViewerError::NonIncreasingFrame {
                previous: self.last_frame_id.unwrap(),
                received: *frame_id,
            });
        }
        if self
            .source_revision
            .is_some_and(|previous| *source_revision < previous)
        {
            return Err(ViewerError::DecreasingSourceRevision {
                previous: self.source_revision.unwrap(),
                received: *source_revision,
            });
        }
        if *page_count != page_sizes.len() {
            return Err(ViewerError::InvalidPatch(format!(
                "page_count {page_count} disagrees with {} page sizes",
                page_sizes.len()
            )));
        }
        for (page_index, size) in page_sizes.iter().enumerate() {
            if !size.width_bp.is_finite()
                || !size.height_bp.is_finite()
                || size.width_bp <= 0.0
                || size.height_bp <= 0.0
            {
                return Err(ViewerError::InvalidPatch(format!(
                    "page {page_index} has invalid dimensions"
                )));
            }
        }
        validate_sorted_indices(changed, *page_count, "changed")?;
        validate_sorted_indices(removed, self.pages.len(), "removed")?;
        if removed.iter().any(|&index| index < *page_count) {
            return Err(ViewerError::InvalidPatch(
                "removed list contains a retained page".into(),
            ));
        }
        let expected_removed = (*page_count..self.pages.len()).collect::<Vec<_>>();
        if *removed != expected_removed {
            return Err(ViewerError::InvalidPatch(format!(
                "expected removed pages {expected_removed:?}, received {removed:?}"
            )));
        }

        let payloads = index_payloads(pages_svg, *page_count)?;
        let supplied = payloads.keys().copied().collect::<Vec<_>>();
        if supplied != *changed {
            return Err(ViewerError::InvalidPatch(format!(
                "changed pages {changed:?} disagree with SVG pages {supplied:?}"
            )));
        }

        let mut candidate = self.pages.clone();
        candidate.resize_with(*page_count, || ViewerPage {
            size: PageSize {
                width_bp: 1.0,
                height_bp: 1.0,
            },
            displayed_svg: None,
            pending_svg: None,
        });
        candidate.truncate(*page_count);
        let prefetch = self
            .visible
            .map(|(first, last)| (first.saturating_sub(1), last.saturating_add(1)));
        for (page_index, page) in candidate.iter_mut().enumerate() {
            page.size = page_sizes[page_index];
            if let Some(svg) = payloads.get(&page_index) {
                let namespace = format!("f{}_p{}", frame_id.get(), page_index);
                let sanitized = sanitize_and_namespace_svg(svg, &namespace)?;
                let prefetched =
                    prefetch.is_some_and(|(first, last)| (first..=last).contains(&page_index));
                if prefetched {
                    page.displayed_svg = Some(sanitized);
                    page.pending_svg = None;
                } else {
                    page.pending_svg = Some(sanitized);
                }
            }
        }

        self.pages = candidate;
        self.last_frame_id = Some(*frame_id);
        self.source_revision = Some(*source_revision);
        self.pending_acks.push_back(*frame_id);
        Ok(())
    }

    /// Take the exactly-once retained-frame acknowledgement.
    pub fn take_ack(&mut self) -> Option<FrameId> {
        self.pending_acks.pop_front()
    }
}

/// Viewer reduction or SVG security failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ViewerError {
    /// Reducer was passed a non-update server event.
    NotAnUpdate,
    /// Frame identity did not strictly increase.
    NonIncreasingFrame {
        /// Previously retained frame.
        previous: FrameId,
        /// Rejected frame.
        received: FrameId,
    },
    /// Source revision decreased.
    DecreasingSourceRevision {
        /// Previously retained source revision.
        previous: SourceRevision,
        /// Rejected source revision.
        received: SourceRevision,
    },
    /// Differential patch shape is inconsistent.
    InvalidPatch(String),
    /// SVG is malformed or contains active/external content.
    UnsafeSvg(String),
}

impl fmt::Display for ViewerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAnUpdate => write!(f, "viewer reducer requires an update event"),
            Self::NonIncreasingFrame { previous, received } => write!(
                f,
                "frame {received} does not increase retained frame {previous}"
            ),
            Self::DecreasingSourceRevision { previous, received } => write!(
                f,
                "source revision {received} precedes retained revision {previous}"
            ),
            Self::InvalidPatch(message) => write!(f, "invalid viewer patch: {message}"),
            Self::UnsafeSvg(message) => write!(f, "unsafe SVG: {message}"),
        }
    }
}

impl std::error::Error for ViewerError {}

/// Reject active/external SVG content and namespace every document-local ID.
pub fn sanitize_and_namespace_svg(svg: &str, namespace: &str) -> Result<String, ViewerError> {
    if namespace.is_empty()
        || !namespace
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(ViewerError::UnsafeSvg("invalid namespace".into()));
    }
    let prefix = format!("{namespace}__");
    let mut reader = Reader::from_str(svg);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::with_capacity(svg.len() + 64)));
    let mut depth = 0usize;
    let mut saw_root = false;

    loop {
        let event = reader
            .read_event()
            .map_err(|error| ViewerError::UnsafeSvg(error.to_string()))?;
        match event {
            Event::Start(start) => {
                let rewritten = sanitize_start(&reader, &start, &prefix, depth == 0)?;
                if depth == 0 {
                    saw_root = true;
                }
                depth += 1;
                writer
                    .write_event(Event::Start(rewritten))
                    .map_err(|error| ViewerError::UnsafeSvg(error.to_string()))?;
            }
            Event::Empty(start) => {
                let rewritten = sanitize_start(&reader, &start, &prefix, depth == 0)?;
                if depth == 0 {
                    saw_root = true;
                }
                writer
                    .write_event(Event::Empty(rewritten))
                    .map_err(|error| ViewerError::UnsafeSvg(error.to_string()))?;
            }
            Event::End(end) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| ViewerError::UnsafeSvg("unbalanced closing tag".into()))?;
                writer
                    .write_event(Event::End(end.into_owned()))
                    .map_err(|error| ViewerError::UnsafeSvg(error.to_string()))?;
            }
            Event::Text(text) => writer
                .write_event(Event::Text(text.into_owned()))
                .map_err(|error| ViewerError::UnsafeSvg(error.to_string()))?,
            Event::CData(text) => writer
                .write_event(Event::CData(text.into_owned()))
                .map_err(|error| ViewerError::UnsafeSvg(error.to_string()))?,
            Event::Comment(comment) => writer
                .write_event(Event::Comment(comment.into_owned()))
                .map_err(|error| ViewerError::UnsafeSvg(error.to_string()))?,
            Event::Decl(decl) => writer
                .write_event(Event::Decl(decl.into_owned()))
                .map_err(|error| ViewerError::UnsafeSvg(error.to_string()))?,
            Event::DocType(_) | Event::PI(_) => {
                return Err(ViewerError::UnsafeSvg(
                    "doctype, processing instructions, and entities are forbidden".into(),
                ));
            }
            Event::Eof => break,
        }
    }
    if !saw_root || depth != 0 {
        return Err(ViewerError::UnsafeSvg(
            "missing or unbalanced SVG root".into(),
        ));
    }
    String::from_utf8(writer.into_inner().into_inner())
        .map_err(|error| ViewerError::UnsafeSvg(error.to_string()))
}

fn sanitize_start(
    reader: &Reader<&[u8]>,
    start: &BytesStart<'_>,
    prefix: &str,
    is_root: bool,
) -> Result<BytesStart<'static>, ViewerError> {
    let element_name = start.name();
    let qualified = std::str::from_utf8(element_name.as_ref())
        .map_err(|_| ViewerError::UnsafeSvg("non-UTF-8 element name".into()))?;
    let local = qualified.rsplit(':').next().unwrap_or(qualified);
    let lower = local.to_ascii_lowercase();
    if is_root && lower != "svg" {
        return Err(ViewerError::UnsafeSvg("root element is not svg".into()));
    }
    if matches!(
        lower.as_str(),
        "script"
            | "style"
            | "foreignobject"
            | "iframe"
            | "object"
            | "embed"
            | "audio"
            | "video"
            | "a"
            | "animate"
            | "animatemotion"
            | "animatetransform"
            | "discard"
            | "mpath"
            | "set"
    ) {
        return Err(ViewerError::UnsafeSvg(format!(
            "element {qualified} is forbidden"
        )));
    }

    let mut rewritten = BytesStart::new(qualified.to_owned());
    for attribute in start.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| ViewerError::UnsafeSvg(error.to_string()))?;
        let key = std::str::from_utf8(attribute.key.as_ref())
            .map_err(|_| ViewerError::UnsafeSvg("non-UTF-8 attribute name".into()))?;
        let key_lower = key.to_ascii_lowercase();
        if key_lower.starts_with("on") {
            return Err(ViewerError::UnsafeSvg(format!(
                "event attribute {key} is forbidden"
            )));
        }
        let value = attribute
            .decode_and_unescape_value(reader.decoder())
            .map_err(|error| ViewerError::UnsafeSvg(error.to_string()))?
            .into_owned();
        let value = if key_lower == "id" {
            if value.is_empty() {
                return Err(ViewerError::UnsafeSvg("empty id".into()));
            }
            format!("{prefix}{value}")
        } else if key_lower == "href" || key_lower.ends_with(":href") {
            rewrite_href(&value, prefix)?
        } else {
            rewrite_urls(&value, prefix)?
        };
        rewritten.push_attribute((key, value.as_str()));
    }
    Ok(rewritten)
}

fn rewrite_href(value: &str, prefix: &str) -> Result<String, ViewerError> {
    if let Some(fragment) = value.strip_prefix('#') {
        if fragment.is_empty() {
            return Err(ViewerError::UnsafeSvg("empty href fragment".into()));
        }
        return Ok(format!("#{prefix}{fragment}"));
    }
    if value.starts_with("data:image/png;base64,")
        || value.starts_with("data:image/jpeg;base64,")
        || value.starts_with("data:image/jpg;base64,")
    {
        return Ok(value.into());
    }
    Err(ViewerError::UnsafeSvg(format!(
        "external href {value:?} is forbidden"
    )))
}

fn rewrite_urls(value: &str, prefix: &str) -> Result<String, ViewerError> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("javascript:") || lower.contains("data:text/html") {
        return Err(ViewerError::UnsafeSvg("active attribute URL".into()));
    }
    let mut rewritten = String::with_capacity(value.len() + prefix.len());
    let mut rest = value;
    while let Some(position) = rest.to_ascii_lowercase().find("url(") {
        rewritten.push_str(&rest[..position]);
        let after = &rest[position + 4..];
        let close = after
            .find(')')
            .ok_or_else(|| ViewerError::UnsafeSvg("unterminated url()".into()))?;
        let target = after[..close].trim().trim_matches(['\'', '"']);
        let fragment = target
            .strip_prefix('#')
            .ok_or_else(|| ViewerError::UnsafeSvg("external url() is forbidden".into()))?;
        rewritten.push_str("url(#");
        rewritten.push_str(prefix);
        rewritten.push_str(fragment);
        rewritten.push(')');
        rest = &after[close + 1..];
    }
    rewritten.push_str(rest);
    Ok(rewritten)
}

fn validate_sorted_indices(indices: &[usize], bound: usize, name: &str) -> Result<(), ViewerError> {
    if indices.iter().any(|&index| index >= bound) {
        return Err(ViewerError::InvalidPatch(format!(
            "{name} index is outside page bound {bound}"
        )));
    }
    if indices.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ViewerError::InvalidPatch(format!(
            "{name} indices are not strictly increasing"
        )));
    }
    Ok(())
}

fn index_payloads(
    payloads: &[PageSvgPayload],
    page_count: usize,
) -> Result<BTreeMap<usize, &str>, ViewerError> {
    let mut indexed = BTreeMap::new();
    for payload in payloads {
        if payload.page_index >= page_count {
            return Err(ViewerError::InvalidPatch(format!(
                "SVG page {} is outside page count {page_count}",
                payload.page_index
            )));
        }
        if indexed
            .insert(payload.page_index, payload.svg.as_str())
            .is_some()
        {
            return Err(ViewerError::InvalidPatch(format!(
                "SVG page {} appears more than once",
                payload.page_index
            )));
        }
    }
    Ok(indexed)
}

#[cfg(test)]
mod tests {
    use crate::{FramePhase, OperationMetrics, RequestSeq, ServerEvent, UpdateOutcome};

    use super::*;

    fn update(
        frame: u64,
        revision: u64,
        page_count: usize,
        changed: Vec<usize>,
        removed: Vec<usize>,
        payloads: Vec<(usize, &str)>,
    ) -> ServerEvent {
        ServerEvent::Update {
            frame_id: FrameId::new(frame),
            source_revision: SourceRevision::new(revision),
            applied_request_seq: RequestSeq::new(revision),
            phase: FramePhase::Foreground,
            outcome: UpdateOutcome::Complete,
            page_count,
            changed,
            removed,
            pages_svg: payloads
                .into_iter()
                .map(|(page_index, svg)| PageSvgPayload {
                    page_index,
                    svg: svg.into(),
                })
                .collect(),
            page_sizes: (0..page_count)
                .map(|_| PageSize {
                    width_bp: 612.0,
                    height_bp: 792.0,
                })
                .collect(),
            unsupported_pages: vec![],
            metrics: OperationMetrics::default(),
            tail_pending: false,
        }
    }

    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><defs><clipPath id="c"><path d="M0 0"/></clipPath></defs><g clip-path="url(#c)"/></svg>"#;

    #[test]
    fn offscreen_payload_is_pending_then_installed_in_prefetch() {
        let mut state = ViewerState::default();
        state.set_visible(0, 0).unwrap();
        state
            .apply(&update(
                1,
                0,
                3,
                vec![0, 1, 2],
                vec![],
                vec![(0, SVG), (1, SVG), (2, SVG)],
            ))
            .unwrap();

        assert!(state.pages()[0].displayed_svg.is_some());
        assert!(state.pages()[1].displayed_svg.is_some());
        assert!(state.pages()[2].pending_svg.is_some());
        assert_eq!(state.set_visible(2, 2).unwrap(), vec![2]);
        assert!(state.pages()[2].displayed_svg.is_some());
    }

    #[test]
    fn pending_payload_overwrites_older_offscreen_payload() {
        let mut state = ViewerState::default();
        state.set_visible(0, 0).unwrap();
        state
            .apply(&update(
                1,
                0,
                3,
                vec![0, 1, 2],
                vec![],
                vec![(0, SVG), (1, SVG), (2, SVG)],
            ))
            .unwrap();
        let newer = "<svg xmlns=\"http://www.w3.org/2000/svg\"><path id=\"new\"/></svg>";
        state
            .apply(&update(2, 1, 3, vec![2], vec![], vec![(2, newer)]))
            .unwrap();

        assert!(state.pages()[2]
            .pending_svg
            .as_ref()
            .unwrap()
            .contains("f2_p2__new"));
    }

    #[test]
    fn metadata_only_and_same_revision_frames_advance_atomically() {
        let mut state = ViewerState::default();
        state
            .apply(&update(1, 7, 1, vec![0], vec![], vec![(0, SVG)]))
            .unwrap();
        state
            .apply(&update(2, 7, 1, vec![], vec![], vec![]))
            .unwrap();

        assert_eq!(state.last_frame_id(), Some(FrameId::new(2)));
        assert_eq!(state.take_ack(), Some(FrameId::new(1)));
        assert_eq!(state.take_ack(), Some(FrameId::new(2)));
        assert_eq!(state.take_ack(), None);
    }

    #[test]
    fn stale_or_malformed_patch_leaves_state_unchanged() {
        let mut state = ViewerState::default();
        state
            .apply(&update(2, 2, 1, vec![0], vec![], vec![(0, SVG)]))
            .unwrap();
        let before = state.clone();

        assert!(matches!(
            state.apply(&update(2, 2, 1, vec![], vec![], vec![])),
            Err(ViewerError::NonIncreasingFrame { .. })
        ));
        assert_eq!(state, before);
        assert!(matches!(
            state.apply(&update(3, 1, 1, vec![], vec![], vec![])),
            Err(ViewerError::DecreasingSourceRevision { .. })
        ));
        assert_eq!(state, before);
        assert!(state
            .apply(&update(3, 3, 1, vec![0], vec![], vec![]))
            .is_err());
        assert_eq!(state, before);
    }

    #[test]
    fn grow_shrink_and_namespaces_are_consistent() {
        let mut state = ViewerState::default();
        state
            .apply(&update(
                1,
                0,
                2,
                vec![0, 1],
                vec![],
                vec![(0, SVG), (1, SVG)],
            ))
            .unwrap();
        assert!(state.pages()[0]
            .pending_svg
            .as_ref()
            .unwrap()
            .contains("f1_p0__c"));
        assert!(state.pages()[1]
            .pending_svg
            .as_ref()
            .unwrap()
            .contains("f1_p1__c"));
        state
            .apply(&update(2, 1, 1, vec![], vec![1], vec![]))
            .unwrap();
        assert_eq!(state.pages().len(), 1);
    }

    #[test]
    fn rejects_active_and_external_svg_content() {
        let hostile = [
            "<svg><script>alert(1)</script></svg>",
            "<svg onload=\"alert(1)\"/>",
            "<svg><foreignObject/></svg>",
            "<svg><animate attributeName=\"opacity\"/></svg>",
            "<svg><image href=\"https://example.test/x.png\"/></svg>",
            "<!DOCTYPE svg><svg/>",
            "<svg><path style=\"fill:url(https://example.test/x)\"/></svg>",
        ];
        for svg in hostile {
            assert!(
                sanitize_and_namespace_svg(svg, "page").is_err(),
                "accepted {svg}"
            );
        }
    }

    #[test]
    fn permits_embedded_raster_data_and_rewrites_fragment_references() {
        let svg = "<svg><defs><clipPath id=\"x\"/></defs><image href=\"data:image/png;base64,AA==\"/><use href=\"#x\"/></svg>";
        let sanitized = sanitize_and_namespace_svg(svg, "p0").unwrap();
        assert!(sanitized.contains("id=\"p0__x\""));
        assert!(sanitized.contains("href=\"#p0__x\""));
        assert!(sanitized.contains("data:image/png;base64,AA=="));
    }
}
