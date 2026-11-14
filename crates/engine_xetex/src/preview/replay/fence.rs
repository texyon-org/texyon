//! Strong source constraints carried by resumable command fences.

use super::adapter::{self, MemoryInput};
use super::source::{SourceRevision, SourceSnapshot, first_difference};
use crate::files::io::UFile;
use crate::scanning::input::InputSlot;
use crate::state::EngineState;
use std::mem::size_of;

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
        state: &EngineState,
        snapshot: &SourceSnapshot,
        resume_pending_command: bool,
    ) -> Option<Self> {
        if state
            .input
            .read_file
            .iter()
            .any(|slot| matches!(slot, InputSlot::Open(_)))
        {
            return None;
        }
        let shipout = &state.shipout;
        if shipout.write_open.iter().enumerate().any(|(stream, open)| {
            *open && shipout.write_name.get(stream).is_none_or(Option::is_none)
        }) {
            return None;
        }
        let inputs = ReplayFenceInputs::capture(snapshot)?;
        let mut slots = Vec::new();
        for (slot, input) in state.input.input_file.iter().enumerate() {
            match input {
                InputSlot::Empty => {}
                InputSlot::Terminal => slots.push(InputSlotCheckpoint::Terminal { slot }),
                InputSlot::Open(file) => {
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
            splice_at: shipout.dvi.bytes_written().max(0) as usize,
            resume_pending_command,
        })
    }

    pub(crate) fn is_eligible(&self, snapshot: &SourceSnapshot) -> bool {
        self.inputs.revision <= snapshot.revision() && self.inputs.is_eligible(snapshot)
    }

    pub(crate) fn restore_inputs(&self, state: &mut EngineState) -> bool {
        if !adapter::restore_inputs(&self.inputs.inputs) {
            return false;
        }
        state.input.input_file.fill_with(|| InputSlot::Empty);
        for checkpoint in &self.slots {
            match *checkpoint {
                InputSlotCheckpoint::Terminal { slot } => {
                    state.input.input_file[slot] = InputSlot::Terminal;
                }
                InputSlotCheckpoint::Open {
                    slot,
                    handle,
                    saved_char,
                    skip_next_lf,
                    encoding_mode,
                } => {
                    state.input.input_file[slot] = InputSlot::Open(Box::new(UFile {
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
        self.inputs.inputs.capacity() * size_of::<MemoryInput>()
            + self.slots.capacity() * size_of::<InputSlotCheckpoint>()
    }
}

/// Source identity and byte-channel positions needed to validate a fence.
#[derive(Clone)]
struct ReplayFenceInputs {
    revision: SourceRevision,
    inputs: Vec<MemoryInput>,
}

impl ReplayFenceInputs {
    /// Capture every editor input ever opened during the active operation.
    fn capture(snapshot: &SourceSnapshot) -> Option<Self> {
        Some(Self {
            revision: snapshot.revision(),
            inputs: adapter::checkpoint_inputs()?,
        })
    }

    /// Whether all consumed source constraints remain sound in `snapshot`.
    ///
    /// A source may change at or beyond its `seen` watermark, which the engine
    /// has not looked at yet. A channel whose watermark reached end of file
    /// acted on the file ending there, so any change, appending included,
    /// invalidates it. Gaining or losing all bytes always invalidates.
    fn is_eligible(&self, snapshot: &SourceSnapshot) -> bool {
        self.inputs.iter().all(|input| {
            let Some((bytes, identity)) = snapshot.replay_entry(&input.path) else {
                return false;
            };
            if identity == input.identity {
                return true;
            }
            let Some(offset) = first_difference(&input.bytes, &bytes) else {
                return true;
            };
            if bytes.is_empty() != input.bytes.is_empty() {
                return false;
            }
            if input.seen >= input.bytes.len() {
                return false;
            }
            offset >= input.seen
        })
    }
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

    fn checkpoint(path: &str, bytes: &[u8], position: usize, closed: bool) -> MemoryInput {
        let source = snapshot(1, bytes, b"child");
        let normalized = source.primary().clone();
        let (bytes, identity) = source.replay_entry(&normalized).unwrap();
        assert_eq!(path, "main.tex");
        MemoryInput {
            path: normalized,
            bytes,
            identity,
            position,
            pushback: None,
            seen: position,
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
    fn identical_bytes_under_a_new_identity_keep_the_fence() {
        let original = b"prefix\nlate text\n";
        let fence = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![checkpoint("main.tex", original, original.len(), true)],
        };
        assert!(fence.is_eligible(&snapshot(2, original, b"child")));
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
    fn a_query_that_read_nothing_constrains_nothing() {
        let original = b"same length";
        let fence = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![checkpoint("main.tex", original, 0, true)],
        };
        assert!(fence.is_eligible(&snapshot(2, b"sAME LENGTH", b"child")));
        assert!(fence.is_eligible(&snapshot(2, b"a completely different length", b"child")));
    }

    #[test]
    fn appending_to_a_fully_read_input_invalidates_the_fence() {
        let original = b"prefix\nlate text\n";
        let fence = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![checkpoint("main.tex", original, original.len(), true)],
        };
        assert!(!fence.is_eligible(&snapshot(2, b"prefix\nlate text\nmore\n", b"child")));
        assert!(!fence.is_eligible(&snapshot(2, b"prefix\nlate", b"child")));
    }

    #[test]
    fn emptiness_cannot_survive_any_watermark() {
        let present = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![checkpoint("main.tex", b"present", 0, true)],
        };
        assert!(!present.is_eligible(&snapshot(2, b"", b"child")));

        let absent = ReplayFenceInputs {
            revision: SourceRevision::new(1),
            inputs: vec![checkpoint("main.tex", b"", 0, true)],
        };
        assert!(!absent.is_eligible(&snapshot(2, b"now present", b"child")));
    }

    #[test]
    fn a_rewound_channel_stays_constrained_by_its_watermark() {
        let original = b"prefix late text";
        let mut input = checkpoint("main.tex", original, 0, true);
        input.seen = 6;
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
        let mut written = std::collections::BTreeSet::new();
        let fence = {
            let _guard = adapter::install(original.clone(), &mut written);
            let handle = adapter::open_primary().unwrap();
            assert_eq!(adapter::getc(handle), Some(b'a'.into()));
            assert_eq!(adapter::getc(handle), Some(b'b'.into()));
            let mut state = crate::state::EngineState::default();
            state.input.input_file.resize_with(2, Default::default);
            state.input.input_file[1] =
                crate::scanning::input::InputSlot::Open(Box::new(crate::files::io::UFile {
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
        let _guard = adapter::install(edited, &mut written);
        let mut state = crate::state::EngineState::default();
        state.input.input_file.resize_with(2, Default::default);
        assert!(fence.restore_inputs(&mut state));
        let crate::scanning::input::InputSlot::Open(file) = &state.input.input_file[1] else {
            panic!("input slot was not restored")
        };
        assert_eq!(file.skip_next_lf, 1);
        assert_eq!(adapter::getc(file.handle), Some(b'c'.into()));
        assert!(fence.resume_pending_command());
    }

    #[test]
    fn complete_fence_recreates_a_deep_open_input_stack() {
        let original = nested_snapshot(1, b"EFgh");
        let mut written = std::collections::BTreeSet::new();
        let fence = {
            let _guard = adapter::install(original.clone(), &mut written);
            let main = adapter::open_primary().unwrap();
            let chapter =
                adapter::open_named("chapter", crate::ffi::bridge::FileFormat::Tex).unwrap();
            let leaf =
                adapter::open_named("parts/leaf", crate::ffi::bridge::FileFormat::Tex).unwrap();
            assert_eq!(adapter::getc(main), Some(b'A'.into()));
            assert_eq!(adapter::getc(chapter), Some(b'C'.into()));
            assert_eq!(adapter::getc(leaf), Some(b'E'.into()));
            assert_eq!(adapter::getc(leaf), Some(b'F'.into()));

            let mut state = crate::state::EngineState::default();
            state.input.input_file.resize_with(4, Default::default);
            for (slot, handle) in [main, chapter, leaf].into_iter().enumerate() {
                state.input.input_file[slot + 1] =
                    crate::scanning::input::InputSlot::Open(Box::new(crate::files::io::UFile {
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
        let _guard = adapter::install(edited, &mut written);
        let mut restored = crate::state::EngineState::default();
        restored.input.input_file.resize_with(4, Default::default);
        assert!(fence.restore_inputs(&mut restored));

        let handles = restored
            .input
            .input_file
            .iter()
            .skip(1)
            .map(|slot| match slot {
                crate::scanning::input::InputSlot::Open(file) => file.handle,
                _ => panic!("nested input slot was not restored"),
            })
            .collect::<Vec<_>>();
        assert_eq!(adapter::getc(handles[0]), Some(b'B'.into()));
        assert_eq!(adapter::getc(handles[1]), Some(b'D'.into()));
        assert_eq!(adapter::getc(handles[2]), Some(b'G'.into()));
    }
}
