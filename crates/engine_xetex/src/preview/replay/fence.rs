//! Strong source constraints carried by resumable command fences.

use std::mem::size_of;

use super::adapter::{self, MemoryInput};
use super::source::{SourceRevision, SourceSnapshot, first_difference};
use crate::files::input_file::{InputFile, InputFileCheckpoint};
use crate::scanning::input::InputSlot;
use crate::state::EngineState;

#[derive(Clone)]
enum InputSlotCheckpoint {
    Terminal {
        slot: usize,
    },
    Open {
        slot: usize,
        file: InputFileCheckpoint,
    },
}

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
                    let file = file.checkpoint()?;
                    if !adapter::checkpoint_input_is_open(&inputs.inputs, file.id) {
                        return None;
                    }
                    slots.push(InputSlotCheckpoint::Open { slot, file });
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
            match checkpoint {
                InputSlotCheckpoint::Terminal { slot } => {
                    state.input.input_file[*slot] = InputSlot::Terminal;
                }
                InputSlotCheckpoint::Open { slot, file } => {
                    state.input.input_file[*slot] =
                        InputSlot::Open(Box::new(InputFile::restore(file)));
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

#[derive(Clone)]
struct ReplayFenceInputs {
    revision: SourceRevision,
    inputs: Vec<MemoryInput>,
}

impl ReplayFenceInputs {
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
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;
    use crate::base::units::BufPos;
    use crate::ffi::bridge::FileFormat;
    use crate::files::input_file::{Decoder, Encoding, InputSource};
    use crate::files::io::input_line;
    use crate::scanning::input::InputBuffer;

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

    fn reader(handle: usize) -> InputFile {
        let mut file = InputFile::new(InputSource::from_handle(handle).unwrap());
        file.decoder = Decoder::Plain(Encoding::Utf8);
        file
    }

    fn line_state(slots: usize) -> EngineState {
        let mut state = EngineState::default();
        state.input.input_file.resize_with(slots, Default::default);
        state.input.buffer = InputBuffer(vec![0; 64]);
        state.input.buf_size = BufPos::from_raw(63);
        state
    }

    fn line(state: &EngineState) -> Vec<i32> {
        state.input.buffer.0[state.input.first.index()..state.input.last.index()].to_vec()
    }

    fn chars(text: &str) -> Vec<i32> {
        text.bytes().map(i32::from).collect()
    }

    #[test]
    fn complete_fence_recreates_open_input_state() {
        let original = snapshot(1, b"abcdef", b"child");
        let mut written = BTreeSet::new();
        let fence = {
            let _guard = adapter::install(original.clone(), &mut written);
            let mut file = reader(adapter::open_primary().unwrap());
            assert_eq!(file.getc(), i32::from(b'a'));
            assert_eq!(file.getc(), i32::from(b'b'));
            file.pending_lf = true;
            file.sync();
            let mut state = EngineState::default();
            state.input.input_file.resize_with(2, Default::default);
            state.input.input_file[1] = InputSlot::Open(Box::new(file));
            ReplayFence::capture(&state, &original, true).unwrap()
        };
        let edited = snapshot(2, b"abcdEF", b"child");
        assert!(fence.is_eligible(&edited));
        let _guard = adapter::install(edited, &mut written);
        let mut state = EngineState::default();
        state.input.input_file.resize_with(2, Default::default);
        assert!(fence.restore_inputs(&mut state));
        let InputSlot::Open(file) = &mut state.input.input_file[1] else {
            panic!("input slot was not restored")
        };
        assert!(file.pending_lf);
        assert_eq!(file.getc(), i32::from(b'c'));
        assert!(fence.resume_pending_command());
    }

    #[test]
    fn complete_fence_recreates_a_deep_open_input_stack() {
        let original = nested_snapshot(1, b"EFgh");
        let mut written = BTreeSet::new();
        let fence = {
            let _guard = adapter::install(original.clone(), &mut written);
            let mut main = reader(adapter::open_primary().unwrap());
            let mut chapter = reader(adapter::open_named("chapter", FileFormat::Tex).unwrap());
            let mut leaf = reader(adapter::open_named("parts/leaf", FileFormat::Tex).unwrap());
            assert_eq!(main.getc(), i32::from(b'A'));
            assert_eq!(chapter.getc(), i32::from(b'C'));
            assert_eq!(leaf.getc(), i32::from(b'E'));
            assert_eq!(leaf.getc(), i32::from(b'F'));

            let mut state = EngineState::default();
            state.input.input_file.resize_with(4, Default::default);
            for (slot, mut file) in [main, chapter, leaf].into_iter().enumerate() {
                file.sync();
                state.input.input_file[slot + 1] = InputSlot::Open(Box::new(file));
            }
            ReplayFence::capture(&state, &original, true).unwrap()
        };

        let edited = nested_snapshot(2, b"EFGH");
        assert!(fence.is_eligible(&edited));
        let _guard = adapter::install(edited, &mut written);
        let mut restored = EngineState::default();
        restored.input.input_file.resize_with(4, Default::default);
        assert!(fence.restore_inputs(&mut restored));

        let next = restored
            .input
            .input_file
            .iter_mut()
            .skip(1)
            .map(|slot| match slot {
                InputSlot::Open(file) => file.getc(),
                _ => panic!("nested input slot was not restored"),
            })
            .collect::<Vec<_>>();
        assert_eq!(next, chars("BDG"));
    }

    #[test]
    fn a_line_read_fences_exactly_the_line_and_its_terminator() {
        let original = snapshot(1, b"ab\r\ncd\n", b"child");
        let mut written = BTreeSet::new();
        let fence = {
            let _guard = adapter::install(original.clone(), &mut written);
            let mut state = line_state(2);
            let mut file = reader(adapter::open_primary().unwrap());
            assert!(input_line(&mut state, &mut file).unwrap());
            assert_eq!(line(&state), chars("ab"));
            state.input.input_file[1] = InputSlot::Open(Box::new(file));
            ReplayFence::capture(&state, &original, true).unwrap()
        };

        // The CR ended the line, the LF after it is still unread.
        assert!(!fence.is_eligible(&snapshot(2, b"aX\r\ncd\n", b"child")));
        assert!(fence.is_eligible(&snapshot(2, b"ab\rXcd\n", b"child")));

        let _guard = adapter::install(snapshot(2, b"ab\rXcd\n", b"child"), &mut written);
        let mut state = line_state(2);
        assert!(fence.restore_inputs(&mut state));
        let InputSlot::Open(mut file) = std::mem::take(&mut state.input.input_file[1]) else {
            panic!("input slot was not restored")
        };
        assert!(input_line(&mut state, &mut file).unwrap());
        assert_eq!(line(&state), chars("Xcd"));
    }
}
