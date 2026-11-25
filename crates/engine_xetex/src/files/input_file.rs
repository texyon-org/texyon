//! Open input files: where their bytes come from and how they decode.

use std::ptr::NonNull;
use std::sync::Arc;

use texyon_bridge_core::{EOF, InputId};

use crate::ffi::bridge::{disk_close, disk_read, disk_rewind};
use crate::ffi::teckit_icu::with_c_str;
use crate::preview::replay::adapter::{self, MemoryInputId};

/// Bytes read from disk per refill.
const DISK_CHUNK: usize = 16 * 1024;

/// Where an open input's bytes come from.
#[derive(Clone, Copy)]
pub(crate) enum InputSource {
    Disk(InputId),
    /// A file of the installed source snapshot.
    Memory(MemoryInputId),
}

impl InputSource {
    /// The source behind a bridge or replay handle, `None` for the invalid handle.
    pub(crate) fn from_handle(handle: usize) -> Option<Self> {
        match MemoryInputId::from_handle(handle) {
            Some(id) => Some(InputSource::Memory(id)),
            None => InputId::from_raw(handle).map(InputSource::Disk),
        }
    }
}

/// A fixed-width encoding the engine decodes itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Encoding {
    Utf8,
    Utf16Be,
    Utf16Le,
    /// Each byte is one character.
    Raw,
}

/// How an input's bytes become characters.
pub(crate) enum Decoder {
    Plain(Encoding),
    Icu(IcuConverter),
}

impl Decoder {
    fn plain(&self) -> Option<Encoding> {
        match self {
            Decoder::Plain(encoding) => Some(*encoding),
            Decoder::Icu(_) => None,
        }
    }
}

// ICU checks the incoming UErrorCode, so it must be zeroed before each call.
unsafe extern "C" {
    #[link_name = concat!("ucnv_open", env!("ICU_VERSION_SUFFIX"))]
    fn ucnv_open(converter_name: *const libc::c_char, err: *mut i32) -> *mut libc::c_void;
    #[link_name = concat!("ucnv_close", env!("ICU_VERSION_SUFFIX"))]
    fn ucnv_close(converter: *mut libc::c_void);
    #[link_name = concat!("ucnv_toAlgorithmic", env!("ICU_VERSION_SUFFIX"))]
    fn ucnv_toAlgorithmic(
        algorithmic_type: libc::c_int,
        cnv: *mut libc::c_void,
        target: *mut libc::c_char,
        target_capacity: i32,
        source: *const libc::c_char,
        source_length: i32,
        err: *mut i32,
    ) -> i32;
}

#[cfg(target_endian = "little")]
const UCNV_UTF32_NATIVE_ENDIAN: libc::c_int = 8;
#[cfg(target_endian = "big")]
const UCNV_UTF32_NATIVE_ENDIAN: libc::c_int = 7;

/// An open ICU converter, closed on drop.
pub(crate) struct IcuConverter(NonNull<libc::c_void>);

impl IcuConverter {
    /// Opens the converter called `name`, or returns ICU's error code.
    pub(crate) fn open(name: &[u8]) -> Result<Self, libc::c_int> {
        let mut err: i32 = 0;
        let cnv = with_c_str(name, |name| {
            // SAFETY: `name` is NUL-terminated and `err` is a valid out-parameter.
            unsafe { ucnv_open(name, &mut err) }
        });
        NonNull::new(cnv).map(IcuConverter).ok_or(err)
    }

    /// Converts `source` into native-endian UTF-32 in `target`, returning the
    /// bytes written and ICU's error code.
    pub(crate) fn decode(&mut self, target: &mut [u8], source: &[u8]) -> (i32, libc::c_int) {
        let mut err: i32 = 0;
        let capacity = i32::try_from(target.len()).unwrap_or(i32::MAX);
        let length = i32::try_from(source.len()).unwrap_or(i32::MAX);
        // SAFETY: the converter is open, both buffers are valid for the lengths
        // passed and `err` is a valid out-parameter.
        let len = unsafe {
            ucnv_toAlgorithmic(
                UCNV_UTF32_NATIVE_ENDIAN,
                self.0.as_ptr(),
                target.as_mut_ptr().cast(),
                capacity,
                source.as_ptr().cast(),
                length,
                &mut err,
            )
        };
        (len, err)
    }
}

impl Drop for IcuConverter {
    fn drop(&mut self) {
        // SAFETY: the converter was opened by `open` and is closed only here.
        unsafe { ucnv_close(self.0.as_ptr()) };
    }
}

/// An open input file with its decoding state.
pub(crate) struct InputFile {
    source: InputSource,
    pub(crate) decoder: Decoder,
    /// The last line ended at a CR, so a LF right after it belongs to that line.
    pub(crate) pending_lf: bool,
    /// A UTF-16 unit read past an unpaired high surrogate, returned next.
    pub(crate) saved: Option<i32>,
    /// Bytes read ahead. A memory source holds its whole file here.
    bytes: Arc<[u8]>,
    pos: usize,
    /// Furthest position reached in `bytes`.
    reached: usize,
    /// File offset of `bytes[0]`.
    offset: usize,
    /// Whether a memory source has loaded its file.
    loaded: bool,
}

/// What a replay fence keeps of an open snapshot input.
#[derive(Clone, Debug)]
pub(crate) struct InputFileCheckpoint {
    pub(crate) id: MemoryInputId,
    encoding: Encoding,
    pending_lf: bool,
    saved: Option<i32>,
}

impl InputFile {
    /// An unread file decoded as raw bytes.
    pub(crate) fn new(source: InputSource) -> Self {
        InputFile {
            source,
            decoder: Decoder::Plain(Encoding::Raw),
            pending_lf: false,
            saved: None,
            bytes: Arc::from([]),
            pos: 0,
            reached: 0,
            offset: 0,
            loaded: false,
        }
    }

    /// The next byte, or `EOF`.
    #[inline]
    pub(crate) fn getc(&mut self) -> i32 {
        match self.bytes.get(self.pos) {
            Some(&byte) => {
                self.pos += 1;
                i32::from(byte)
            }
            None => self.getc_after_refill(),
        }
    }

    #[cold]
    fn getc_after_refill(&mut self) -> i32 {
        if self.refill() { self.getc() } else { EOF }
    }

    /// Steps back over the byte the last [`Self::getc`] returned.
    pub(crate) fn unget(&mut self) {
        self.pos -= 1;
    }

    fn refill(&mut self) -> bool {
        self.reached = self.reached.max(self.pos);
        match self.source {
            InputSource::Disk(id) => {
                let mut chunk = vec![0; DISK_CHUNK];
                let n = disk_read(id, &mut chunk);
                chunk.truncate(n);
                self.offset += self.bytes.len();
                self.bytes = Arc::from(chunk);
                self.pos = 0;
                self.reached = 0;
                n > 0
            }
            InputSource::Memory(id) => {
                if self.loaded {
                    return false;
                }
                let (bytes, position) = adapter::contents(id);
                self.bytes = bytes;
                self.pos = position;
                self.reached = position;
                self.loaded = true;
                self.pos < self.bytes.len()
            }
        }
    }

    /// Goes back to the start of the file, as after a seek to 0.
    pub(crate) fn rewind(&mut self) {
        self.reached = self.reached.max(self.pos);
        if self.offset == 0 {
            self.pos = 0;
            return;
        }
        if let InputSource::Disk(id) = self.source {
            disk_rewind(id);
            self.bytes = Arc::from([]);
            self.pos = 0;
            self.reached = 0;
            self.offset = 0;
        }
    }

    /// Hands the read position to the replay sources, so fences see what the
    /// engine has read. Disk sources need nothing.
    pub(crate) fn sync(&mut self) {
        if let InputSource::Memory(id) = self.source
            && self.loaded
        {
            self.reached = self.reached.max(self.pos);
            adapter::sync(id, self.pos, self.reached);
        }
    }

    /// Closes the file. Returns `true` on a serious internal close error.
    pub(crate) fn close(mut self) -> bool {
        self.sync();
        match self.source {
            InputSource::Disk(id) => disk_close(id),
            InputSource::Memory(id) => adapter::close_memory(id),
        }
    }

    /// The fence checkpoint of a snapshot input decoded by the engine itself.
    pub(crate) fn checkpoint(&self) -> Option<InputFileCheckpoint> {
        let InputSource::Memory(id) = self.source else {
            return None;
        };
        Some(InputFileCheckpoint {
            id,
            encoding: self.decoder.plain()?,
            pending_lf: self.pending_lf,
            saved: self.saved,
        })
    }

    /// Reopens a checkpointed input at the position the sources were restored to.
    pub(crate) fn restore(checkpoint: &InputFileCheckpoint) -> Self {
        InputFile {
            decoder: Decoder::Plain(checkpoint.encoding),
            pending_lf: checkpoint.pending_lf,
            saved: checkpoint.saved,
            ..InputFile::new(InputSource::Memory(checkpoint.id))
        }
    }
}
