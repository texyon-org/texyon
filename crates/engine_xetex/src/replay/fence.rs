//! Strong source constraints carried by resumable command fences.

use super::adapter::{self, MemoryInputCheckpoint};
use super::source::{SourceRevision, SourceSnapshot};

#[derive(Clone)]
enum InputSlotCheckpoint {
    Terminal {
        slot: usize,
    },
    Open {
        slot: usize,
        handle: usize,
        saved_char: i64,
        skip_next_lf: i16,
        encoding_mode: i16,
    },
}

/// Complete engine/adapter input reconstruction record for one command fence.
#[derive(Clone)]
pub(crate) struct ReplayFence {
    inputs: ReplayFenceInputs,
    slots: Vec<InputSlotCheckpoint>,
    splice_at: usize,
    resume_pending_command: bool,
}

impl ReplayFence {
    pub(crate) fn capture(
        state: &crate::state::EngineState,
        snapshot: &SourceSnapshot,
        resume_pending_command: bool,
    ) -> Option<Self> {
        if state
            .input
            .read_file
            .iter()
            .any(|slot| matches!(slot, crate::state::InputSlot::Open(_)))
        {
            return None;
        }
        if state
            .shipout
            .write_open
            .iter()
            .enumerate()
            .any(|(stream, open)| {
                *open
                    && (stream >= state.shipout.write_name.len()
                        || state.shipout.write_name[stream].last() != Some(&0))
            })
        {
            return None;
        }
        let inputs = ReplayFenceInputs::capture(snapshot)?;
        let mut slots = Vec::new();
        for (slot, input) in state.input.input_file.iter().enumerate() {
            match input {
                crate::state::InputSlot::Empty => {}
                crate::state::InputSlot::Terminal => {
                    slots.push(InputSlotCheckpoint::Terminal { slot });
                }
                crate::state::InputSlot::Open(file) => {
                    if !file.conversion_data.is_null()
                        || !adapter::checkpoint_handle_is_open(&inputs.inputs, file.handle)
                    {
                        return None;
                    }
                    slots.push(InputSlotCheckpoint::Open {
                        slot,
                        handle: file.handle,
                        saved_char: file.saved_char,
                        skip_next_lf: file.skip_next_lf,
                        encoding_mode: file.encoding_mode,
                    });
                }
            }
        }
        Some(Self {
            inputs,
            slots,
            splice_at: state.shipout.dvi_gone.max(0) as usize,
            resume_pending_command,
        })
    }

    pub(crate) fn is_eligible(&self, snapshot: &SourceSnapshot) -> bool {
        self.inputs.revision() <= snapshot.revision() && self.inputs.is_eligible(snapshot)
    }

    pub(crate) fn restore_inputs(&self, state: &mut crate::state::EngineState) -> bool {
        if !self.inputs.restore_inputs() {
            return false;
        }
        for slot in &mut state.input.input_file {
            *slot = crate::state::InputSlot::Empty;
        }
        for checkpoint in &self.slots {
            match *checkpoint {
                InputSlotCheckpoint::Terminal { slot } => {
                    state.input.input_file[slot] = crate::state::InputSlot::Terminal;
                }
                InputSlotCheckpoint::Open {
                    slot,
                    handle,
                    saved_char,
                    skip_next_lf,
                    encoding_mode,
                } => {
                    state.input.input_file[slot] =
                        crate::state::InputSlot::Open(Box::new(crate::io::UFile {
                            handle,
                            saved_char,
                            skip_next_lf,
                            encoding_mode,
                            conversion_data: std::ptr::null_mut(),
                        }));
                }
            }
        }
        true
    }

    pub(crate) fn splice_at(&self) -> usize {
        self.splice_at
    }

    pub(crate) fn resume_pending_command(&self) -> bool {
        self.resume_pending_command
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.inputs.allocated_bytes()
            + self.slots.capacity() * std::mem::size_of::<InputSlotCheckpoint>()
    }
}

/// Source identity and byte-channel positions needed to validate a fence.
#[derive(Clone)]
pub(crate) struct ReplayFenceInputs {
    revision: SourceRevision,
    inputs: Vec<MemoryInputCheckpoint>,
}

impl ReplayFenceInputs {
    /// Capture every editor input ever opened during the active operation.
    pub(crate) fn capture(snapshot: &SourceSnapshot) -> Option<Self> {
        Some(Self {
            revision: snapshot.revision(),
            inputs: adapter::checkpoint_inputs()?,
        })
    }

    /// Whether all consumed source constraints remain sound in `snapshot`.
    ///
    /// An input may differ only after the source prefix that was observed or
    /// lies before its resumable byte cursor. Size and EOF observations also
    /// constrain length, except for a probe that consumed nothing, which is
    /// held only to the existence and emptiness it actually distinguished.
    pub(crate) fn is_eligible(&self, snapshot: &SourceSnapshot) -> bool {
        self.inputs.iter().all(|input| {
            let Some((bytes, identity)) = snapshot.replay_entry(&input.path) else {
                return false;
            };
            if identity == input.identity {
                return true;
            }
            if bytes.len() < input.minimum_size {
                return false;
            }
            if input.exact_size_observed
                && input.bytes.len() != bytes.len()
                && !probe_survives_resize(input, &bytes)
            {
                return false;
            }
            let constrained_prefix = input.position.max(input.observed_prefix);
            first_difference(&input.bytes, &bytes)
                .is_some_and(|offset| offset >= constrained_prefix)
        })
    }

    /// Restore the captured byte-channel table after the new snapshot is installed.
    pub(crate) fn restore_inputs(&self) -> bool {
        adapter::restore_inputs(&self.inputs)
    }

    pub(crate) fn revision(&self) -> SourceRevision {
        self.revision
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.inputs.capacity() * std::mem::size_of::<MemoryInputCheckpoint>()
    }
}

/// Whether a size observation that consumed no bytes still holds after a
/// length change.
///
/// LaTeX resolves `\input` and graphics paths by asking for a file's size and
/// closing it again without reading it, and branches only on whether the answer
/// is empty. Holding such a probe to an exact length makes every fence planted
/// after it useless the moment the author's next keystroke changes that file's
/// length, which forces each edit inside an included file to replay from before
/// the file was opened. Constraining these probes to the existence and
/// emptiness they discriminate keeps the fences that sit next to the edit.
///
/// A probe that read, sought, or pushed back any byte is not covered: those
/// observations still constrain the exact length.
fn probe_survives_resize(input: &MemoryInputCheckpoint, bytes: &[u8]) -> bool {
    let consumed_nothing = input.position == 0
        && input.observed_prefix == 0
        && input.minimum_size == 0
        && input.pushback.is_none();
    consumed_nothing && input.bytes.is_empty() == bytes.is_empty()
}

fn first_difference(left: &[u8], right: &[u8]) -> Option<usize> {
    let shared = left.len().min(right.len());
    left[..shared]
        .iter()
        .zip(&right[..shared])
        .position(|(left, right)| left != right)
        .or_else(|| (left.len() != right.len()).then_some(shared))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;

    fn snapshot(revision: u64, primary: &[u8], child: &[u8]) -> SourceSnapshot {
        SourceSnapshot::new_with_revision(
            PathBuf::from("/workspace"),
            PathBuf::from("main.tex"),
            vec![
                (PathBuf::from("main.tex"), Arc::from(primary)),
                (PathBuf::from("child.tex"), Arc::from(child)),
            ],
            SourceRevision::new(revision),
        )
        .unwrap()
    }

    fn nested_snapshot(revision: u64, leaf: &[u8]) -> SourceSnapshot {
        SourceSnapshot::new_with_revision(
            PathBuf::from("/workspace"),
            PathBuf::from("main.tex"),
            vec![
                (PathBuf::from("main.tex"), Arc::from(&b"AB"[..])),
                (PathBuf::from("chapter.tex"), Arc::from(&b"CD"[..])),
                (PathBuf::from("parts/leaf.tex"), Arc::from(leaf)),
            ],
            SourceRevision::new(revision),
        )
        .unwrap()
    }

    fn checkpoint(
        path: &str,
        bytes: &[u8],
        position: usize,
        closed: bool,
    ) -> MemoryInputCheckpoint {
        let source = snapshot(1, bytes, b"child");
        let normalized = source.primary().clone();
        let (bytes, identity) = source.replay_entry(&normalized).unwrap();
        assert_eq!(path, "main.tex");
        let exact_size_observed = closed && position == bytes.len();
        MemoryInputCheckpoint {
            path: normalized,
            bytes,
            identity,
            position,
            pushback: None,
            observed_prefix: position,
            minimum_size: position,
            exact_size_observed,
            closed,
        }
    }

    #[test]
    fn unchanged_or_later_edits_preserve_an_open_constraint() {
        let original = b"prefix\nlate text\n";
        let fence = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![checkpoint("main.tex", original, 7, false)],
        };
        assert!(fence.is_eligible(&snapshot(2, original, b"child")));
        assert!(fence.is_eligible(&snapshot(2, b"prefix\nLATE text\n", b"child")));
        assert!(!fence.is_eligible(&snapshot(2, b"Prefix\nlate text\n", b"child")));
    }

    #[test]
    fn any_change_to_a_closed_input_invalidates_the_fence() {
        let original = b"prefix\nlate text\n";
        let fence = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![checkpoint("main.tex", original, original.len(), true)],
        };
        assert!(!fence.is_eligible(&snapshot(2, b"prefix\nlate text!\n", b"child")));
    }

    #[test]
    fn change_to_an_unread_closed_probe_preserves_the_fence() {
        let original = b"prefix\nlate text\n";
        let fence = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![checkpoint("main.tex", original, 0, true)],
        };
        assert!(fence.is_eligible(&snapshot(2, b"changed entirely\n", b"child")));
    }

    #[test]
    fn a_size_probe_that_read_nothing_survives_a_resize() {
        let original = b"same length";
        let mut input = checkpoint("main.tex", original, 0, true);
        input.exact_size_observed = true;
        let fence = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![input],
        };
        assert!(fence.is_eligible(&snapshot(2, b"SAME LENGTH", b"child")));
        assert!(fence.is_eligible(&snapshot(2, b"different length", b"child")));
    }

    #[test]
    fn a_size_probe_still_discriminates_an_empty_file() {
        let original = b"present";
        let mut input = checkpoint("main.tex", original, 0, true);
        input.exact_size_observed = true;
        let fence = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![input],
        };
        assert!(!fence.is_eligible(&snapshot(2, b"", b"child")));

        let mut empty = checkpoint("main.tex", b"", 0, true);
        empty.exact_size_observed = true;
        let from_empty = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![empty],
        };
        assert!(!from_empty.is_eligible(&snapshot(2, b"now present", b"child")));
    }

    #[test]
    fn a_size_observation_after_reading_still_pins_the_length() {
        let original = b"prefix late text";
        let mut input = checkpoint("main.tex", original, 0, true);
        input.exact_size_observed = true;
        input.observed_prefix = 6;
        let fence = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![input],
        };
        assert!(!fence.is_eligible(&snapshot(2, b"prefix late text!", b"child")));
    }

    #[test]
    fn rewound_prefix_probe_constrains_only_observed_bytes() {
        let original = b"prefix late text";
        let mut input = checkpoint("main.tex", original, 0, true);
        input.observed_prefix = 6;
        input.minimum_size = 6;
        let fence = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![input],
        };
        assert!(fence.is_eligible(&snapshot(2, b"prefix LATE TEXT", b"child")));
        assert!(fence.is_eligible(&snapshot(2, b"prefix much longer late text", b"child")));
        assert!(!fence.is_eligible(&snapshot(2, b"PREFIX late text", b"child")));
    }

    #[test]
    fn complete_fence_recreates_open_ufile_state() {
        let original = snapshot(1, b"abcdef", b"child");
        let fence = {
            let _guard = adapter::install(original.clone());
            let handle = adapter::open_primary().unwrap();
            assert_eq!(adapter::getc(handle), Some(b'a'.into()));
            assert_eq!(adapter::getc(handle), Some(b'b'.into()));
            let mut state = crate::state::EngineState::default();
            state.input.input_file.resize_with(2, Default::default);
            state.input.input_file[1] = crate::state::InputSlot::Open(Box::new(crate::io::UFile {
                handle,
                saved_char: -1,
                skip_next_lf: 1,
                encoding_mode: 1,
                conversion_data: std::ptr::null_mut(),
            }));
            ReplayFence::capture(&state, &original, true).unwrap()
        };
        let edited = snapshot(2, b"abcdEF", b"child");
        assert!(fence.is_eligible(&edited));
        let _guard = adapter::install(edited);
        let mut state = crate::state::EngineState::default();
        state.input.input_file.resize_with(2, Default::default);
        assert!(fence.restore_inputs(&mut state));
        let crate::state::InputSlot::Open(file) = &state.input.input_file[1] else {
            panic!("input slot was not restored")
        };
        assert_eq!(file.skip_next_lf, 1);
        assert_eq!(adapter::getc(file.handle), Some(b'c'.into()));
        assert!(fence.resume_pending_command());
    }

    #[test]
    fn complete_fence_recreates_a_deep_open_input_stack() {
        let original = nested_snapshot(1, b"EFgh");
        let fence = {
            let _guard = adapter::install(original.clone());
            let main = adapter::open_primary().unwrap();
            let chapter = unsafe { adapter::open_named(c"chapter".as_ptr(), 26) }.unwrap();
            let leaf = unsafe { adapter::open_named(c"parts/leaf".as_ptr(), 26) }.unwrap();
            assert_eq!(adapter::getc(main), Some(b'A'.into()));
            assert_eq!(adapter::getc(chapter), Some(b'C'.into()));
            assert_eq!(adapter::getc(leaf), Some(b'E'.into()));
            assert_eq!(adapter::getc(leaf), Some(b'F'.into()));

            let mut state = crate::state::EngineState::default();
            state.input.input_file.resize_with(4, Default::default);
            for (slot, handle) in [main, chapter, leaf].into_iter().enumerate() {
                state.input.input_file[slot + 1] =
                    crate::state::InputSlot::Open(Box::new(crate::io::UFile {
                        handle,
                        saved_char: -1,
                        skip_next_lf: slot as i16,
                        encoding_mode: 1,
                        conversion_data: std::ptr::null_mut(),
                    }));
            }
            ReplayFence::capture(&state, &original, true).unwrap()
        };

        let edited = nested_snapshot(2, b"EFGH");
        assert!(fence.is_eligible(&edited));
        let _guard = adapter::install(edited);
        let mut restored = crate::state::EngineState::default();
        restored.input.input_file.resize_with(4, Default::default);
        assert!(fence.restore_inputs(&mut restored));

        let handles = restored
            .input
            .input_file
            .iter()
            .skip(1)
            .map(|slot| match slot {
                crate::state::InputSlot::Open(file) => file.handle,
                _ => panic!("nested input slot was not restored"),
            })
            .collect::<Vec<_>>();
        assert_eq!(adapter::getc(handles[0]), Some(b'B'.into()));
        assert_eq!(adapter::getc(handles[1]), Some(b'D'.into()));
        assert_eq!(adapter::getc(handles[2]), Some(b'G'.into()));
    }
}
