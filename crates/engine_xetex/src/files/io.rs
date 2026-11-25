//! Low-level input and output: encoded line reading, file names, and the
//! log and input-file lifecycle.

use std::cell::RefCell;
use std::{iter, mem, slice};

use texyon_bridge_core::EOF;

use crate::base::cmd::Cmd;
use crate::base::consts::{BIGGEST_USV, CLOSED, EMPTY_STRING, INVALID_HANDLE, JUST_OPEN};
use crate::base::params::IntPar;
use crate::base::token::TokCursor;
use crate::base::units::{BufPos, StrNumber};
use crate::base::utf8;
use crate::base::utf8::{BYTES_FROM_UTF8, OFFSETS_FROM_UTF8, next_utf8_char};
use crate::engine::output_file_extension;
use crate::ffi::bridge::{
    FileFormat, directory_exists, input_open, input_open_primary, last_input_abspath,
    last_input_found, output_close, output_flush, output_open,
};
pub(crate) use crate::ffi::bridge::{input_close, input_seek};
use crate::ffi::teckit_icu::FontMappingHandle;
use crate::files::input_file::{Decoder, Encoding, IcuConverter, InputFile, InputSource};
use crate::print::diagnostic::warn_host;
use crate::print::errors::{Abort, FilePrompt, prompt_file_name};
use crate::print::{
    Selector, TexPrint, bad_utf8_warning, begin_diagnostic, end_diagnostic, log_putc, print,
    print_char, print_file_name, print_int, print_ln, print_nl, show_token_list,
};
use crate::scanning::input::{
    InputName, InputSlot, NEW_LINE, back_input, begin_file_reading, end_file_reading,
};
use crate::scanning::scan::{
    finish_line, get_x_token, scan_four_bit_int, scan_optional_equals, scan_toks,
};
use crate::shipout::synctex::{synctex_start_input, synctex_terminate};
use crate::shipout::{ensure_write_stream_open, finalize_dvi_file};
use crate::state::EngineState;
use crate::tables::string_pool::StringsState;

pub(crate) const AUTO: i32 = 0;
pub(crate) const UTF8: i32 = 1;
pub(crate) const UTF16BE: i32 = 2;
pub(crate) const UTF16LE: i32 = 3;
pub(crate) const RAW: i32 = 4;
pub(crate) const ICUMAPPING: i32 = 5;

#[cfg(target_endian = "little")]
const NATIVE_UTF32: u16 = 6;
#[cfg(target_endian = "big")]
const NATIVE_UTF32: u16 = 5;
const K_FORM_NFC: u16 = 0x0100;
const K_FORM_NFD: u16 = 0x0200;
pub(crate) const K_STATUS_NO_ERROR: libc::c_long = 0;

#[derive(Clone, Copy)]
pub(crate) enum FileSlot {
    Input(usize),
    Read(usize),
}

impl FileSlot {
    fn get(self, st: &mut EngineState) -> &mut InputSlot {
        match self {
            FileSlot::Input(idx) => &mut st.input.input_file[idx],
            FileSlot::Read(idx) => &mut st.input.read_file[idx],
        }
    }
}

/// Reads the next line of the file in `slot` into `buffer[first..last]`.
/// Returns `false` at end of file.
pub(crate) fn input_slot_line(st: &mut EngineState, slot: FileSlot) -> Result<bool, Abort> {
    let mut taken = mem::take(slot.get(st));
    let result = match &mut taken {
        InputSlot::Open(file) => input_line(st, file),
        InputSlot::Empty | InputSlot::Terminal => Err(Abort::Internal(
            "reads from synthetic \"terminal\" file #0 should never happen",
        )),
    };
    *slot.get(st) = taken;
    result
}

/// Switches the file in `slot` to `mode`. On the terminal the switch only reports
/// a converter that fails to open.
pub(crate) fn set_slot_encoding(
    st: &mut EngineState,
    slot: FileSlot,
    mode: i32,
    encoding_data: i32,
) -> Result<(), Abort> {
    let mut taken = mem::take(slot.get(st));
    match &mut taken {
        InputSlot::Empty => {}
        InputSlot::Terminal => {
            decoder_for(st, mode, encoding_data);
        }
        InputSlot::Open(file) => {
            if let Some(decoder) = decoder_for(st, mode, encoding_data) {
                file.decoder = decoder;
            }
        }
    }
    *slot.get(st) = taken;
    Ok(())
}

pub(crate) fn icu_converter_exists(name: &[u8]) -> Result<(), libc::c_int> {
    IcuConverter::open(name).map(drop)
}

fn utf32_bytes(units: &[u32]) -> &[u8] {
    // SAFETY: `u32` has no padding, so the view covers initialised bytes.
    unsafe { slice::from_raw_parts(units.as_ptr().cast(), mem::size_of_val(units)) }
}

fn utf32_bytes_mut<T: Utf32Unit>(units: &mut [T]) -> &mut [u8] {
    // SAFETY: `T` is a 4-byte integer, so every byte pattern is a valid value
    // and the byte view covers exactly the same memory.
    unsafe { slice::from_raw_parts_mut(units.as_mut_ptr().cast(), mem::size_of_val(units)) }
}

trait Utf32Unit {}
impl Utf32Unit for i32 {}
impl Utf32Unit for u32 {}

pub(crate) struct FoundInput {
    /// The path plus the extension the bridge added, as TeX Live prints it.
    pub name: Vec<u8>,
    pub abspath: Vec<u8>,
}

/// Opens input `path` undecoded. `None` means it was not found.
pub(crate) fn open_input_file(
    st: &mut EngineState,
    path: &[u8],
    filefmt: FileFormat,
) -> Option<(InputFile, FoundInput)> {
    let handle = if filefmt == FileFormat::TexyonPrimary {
        input_open_primary()
    } else if path.first() == Some(&b'|') {
        // TeX Live without shell escape takes the name as a file, which is not found.
        warn_host(st, "TeXyoN cannot read from shell commands");
        return None;
    } else {
        input_open(path, filefmt, false)
    };
    let source = InputSource::from_handle(handle)?;

    let name = if filefmt == FileFormat::TexyonPrimary {
        path.to_vec()
    } else {
        last_found_name(path)
    };
    let abspath = last_input_abspath();
    Some((InputFile::new(source), FoundInput { name, abspath }))
}

/// The name the last input asked as `path` was found under, as kpathsea returns it.
pub(crate) fn last_found_name(path: &[u8]) -> Vec<u8> {
    last_input_found().name(path)
}

pub(crate) fn u_open_in(
    st: &mut EngineState,
    path: &[u8],
    filefmt: FileFormat,
    mut mode: i32,
    encoding_data: i32,
) -> Result<Option<(Box<InputFile>, FoundInput)>, Abort> {
    let Some((mut f, found)) = open_input_file(st, path, filefmt) else {
        return Ok(None);
    };

    if mode == AUTO {
        let b1 = f.getc();
        let b2 = f.getc();

        if b1 == 0xfe && b2 == 0xff {
            mode = UTF16BE;
        } else if b2 == 0xfe && b1 == 0xff {
            mode = UTF16LE;
        } else if b1 == 0 && b2 != 0 {
            mode = UTF16BE;
            f.rewind();
        } else if b2 == 0 && b1 != 0 {
            mode = UTF16LE;
            f.rewind();
        } else if b1 == 0xEF && b2 == 0xBB && f.getc() == 0xBF {
            mode = UTF8;
        }

        if mode == AUTO {
            f.rewind();
            mode = UTF8;
        }
        f.sync();
    }

    if let Some(decoder) = decoder_for(st, mode, encoding_data) {
        f.decoder = decoder;
    }
    Ok(Some((Box::new(f), found)))
}

/// The decoder for input mode `mode`, `None` for a mode that names none. A
/// converter that fails to open is reported and reads raw bytes.
fn decoder_for(st: &mut EngineState, mode: i32, encoding_data: i32) -> Option<Decoder> {
    let encoding = match mode {
        UTF8 => Encoding::Utf8,
        UTF16BE => Encoding::Utf16Be,
        UTF16LE => Encoding::Utf16Le,
        RAW => Encoding::Raw,
        ICUMAPPING => {
            let name = st
                .strings
                .texstring_bytes(StrNumber::from_raw(encoding_data));
            match IcuConverter::open(&name) {
                Ok(cnv) => return Some(Decoder::Icu(cnv)),
                Err(err) => {
                    begin_diagnostic(st);
                    print_nl(st, b'E');
                    print(st, "rror ");
                    print_int(st, err);
                    print(st, " creating Unicode converter for `");
                    print(st, &name[..]);
                    print(st, "'; reading as raw bytes");
                    end_diagnostic(st, true);
                    Encoding::Raw
                }
            }
        }
        _ => return None,
    };
    Some(Decoder::Plain(encoding))
}

#[derive(Default)]
struct LineScratch {
    bytes: Vec<u8>,
    utf32: Vec<u32>,
    normalizers: [Option<FontMappingHandle>; 2],
}

thread_local! {
    static LINE_SCRATCH: RefCell<LineScratch> = Default::default();
}

fn is_line_end(c: i32) -> bool {
    c == EOF || c == i32::from(b'\n') || c == i32::from(b'\r')
}

fn line_too_long() -> Abort {
    Abort::Internal("unable to read an entire line (buffer too small)")
}

fn conversion_error(st: &mut EngineState, errcode: i32) {
    begin_diagnostic(st);
    print_nl(st, b'U');
    print(st, "nicode conversion failed (ICU error code = ");
    print_int(st, errcode);
    print(st, ") discarding any remaining text");
    end_diagnostic(st, true);
}

/// Normalizes the UTF-32 `units` into `buffer[first..]` (NFC for `norm` 1,
/// NFD for 2) and sets `last`.
fn apply_normalization(
    st: &mut EngineState,
    normalizers: &mut [Option<FontMappingHandle>; 2],
    units: &[u32],
    norm: i32,
) -> Result<(), Abort> {
    let converter = match &mut normalizers[(norm - 1) as usize] {
        Some(converter) => converter,
        slot => {
            let target_form = NATIVE_UTF32 | if norm == 1 { K_FORM_NFC } else { K_FORM_NFD };
            match FontMappingHandle::new(None, true, NATIVE_UTF32, target_form) {
                (Some(handle), K_STATUS_NO_ERROR) => slot.insert(handle),
                _ => return Err(Abort::Internal("failed to create normalizer")),
            }
        }
    };

    let first = st.input.first.index();
    let out = utf32_bytes_mut(&mut st.input.buffer.0[first..st.input.buf_size.index()]);
    let (status, out_used) = converter.convert(utf32_bytes(units), out);
    if status != K_STATUS_NO_ERROR {
        return Err(line_too_long());
    }
    st.input.last = st.input.first + (out_used / 4) as i32;
    Ok(())
}

/// Reads one line of `f` into `buffer[first..last]`, decoding and optionally
/// normalizing it. Returns `false` at end of file.
pub(crate) fn input_line(st: &mut EngineState, f: &mut InputFile) -> Result<bool, Abort> {
    let more = LINE_SCRATCH.with(|scratch| input_line_with(st, f, &mut scratch.borrow_mut()));
    f.sync();
    more
}

fn input_line_with(
    st: &mut EngineState,
    f: &mut InputFile,
    scratch: &mut LineScratch,
) -> Result<bool, Abort> {
    let norm = if st.eqtb.ints.values().is_empty() {
        0
    } else {
        st.eqtb.int_par(IntPar::XetexInputNormalization)
    };

    st.input.last = st.input.first;
    let buf_size = st.input.buf_size.index();
    if norm == 1 || norm == 2 {
        scratch.utf32.resize(buf_size, 0);
    }

    let mut i;
    if matches!(f.decoder, Decoder::Icu(_)) {
        scratch.bytes.resize(buf_size + 1, 0);
        let mut bytes_read = 0;

        i = f.getc();
        if mem::take(&mut f.pending_lf) && i == i32::from(b'\n') {
            i = f.getc();
        }

        if !is_line_end(i) {
            scratch.bytes[bytes_read] = i as u8;
            bytes_read += 1;
            while bytes_read < buf_size {
                i = f.getc();
                if is_line_end(i) {
                    break;
                }
                scratch.bytes[bytes_read] = i as u8;
                bytes_read += 1;
            }
        }

        if i == EOF && bytes_read == 0 {
            return Ok(false);
        }
        if !is_line_end(i) {
            return Err(line_too_long());
        }

        let Decoder::Icu(cnv) = &mut f.decoder else {
            unreachable!("checked above");
        };
        let source = &scratch.bytes[..bytes_read];
        if norm == 1 || norm == 2 {
            let (len, error_code) = cnv.decode(utf32_bytes_mut(&mut scratch.utf32), source);
            if error_code != 0 {
                conversion_error(st, error_code);
                return Ok(false);
            }
            let units = &scratch.utf32[..(len / 4) as usize];
            apply_normalization(st, &mut scratch.normalizers, units, norm)?;
        } else {
            let first = st.input.first.index();
            let out = utf32_bytes_mut(&mut st.input.buffer.0[first..buf_size]);
            let (len, error_code) = cnv.decode(out, source);
            if error_code != 0 {
                conversion_error(st, error_code);
                return Ok(false);
            }
            st.input.last = st.input.first + len / 4;
        }
    } else {
        i = get_uni_c(st, f);
        if mem::take(&mut f.pending_lf) && i == i32::from(b'\n') {
            i = get_uni_c(st, f);
        }

        if norm == 1 || norm == 2 {
            let mut len = 0;
            if !is_line_end(i) {
                scratch.utf32[len] = i as u32;
                len += 1;
                while len < buf_size {
                    i = get_uni_c(st, f);
                    if is_line_end(i) {
                        break;
                    }
                    scratch.utf32[len] = i as u32;
                    len += 1;
                }
            }

            if i == EOF && len == 0 {
                return Ok(false);
            }
            if !is_line_end(i) {
                return Err(line_too_long());
            }
            apply_normalization(st, &mut scratch.normalizers, &scratch.utf32[..len], norm)?;
        } else {
            if st.input.last < st.input.buf_size && !is_line_end(i) {
                st.input.buffer[st.input.last] = i;
                st.input.last += 1;
                while st.input.last < st.input.buf_size {
                    i = get_uni_c(st, f);
                    if is_line_end(i) {
                        break;
                    }
                    st.input.buffer[st.input.last] = i;
                    st.input.last += 1;
                }
            }

            if i == EOF && st.input.last == st.input.first {
                return Ok(false);
            }
            if !is_line_end(i) {
                return Err(line_too_long());
            }
        }
    }

    if i == i32::from(b'\r') {
        f.pending_lf = true;
    }

    st.input.buffer[st.input.last] = i32::from(b' ');
    if st.input.last >= st.input.max_buf_stack {
        st.input.max_buf_stack = st.input.last;
    }

    while st.input.last > st.input.first {
        let c = st.input.buffer[st.input.last - 1];
        if c != i32::from(b' ') && c != i32::from(b'\r') && c != i32::from(b'\n') {
            break;
        }
        st.input.last -= 1;
    }

    Ok(true)
}

/// The next character of `f` in one of the encodings the engine decodes itself.
fn get_uni_c(st: &mut EngineState, f: &mut InputFile) -> i32 {
    if let Some(saved) = f.saved.take() {
        return saved;
    }

    let Decoder::Plain(encoding) = f.decoder else {
        unreachable!("ICU input is read as bytes");
    };
    match encoding {
        Encoding::Utf8 => {
            let mut rval = f.getc();
            if rval != EOF {
                let extra_bytes = BYTES_FROM_UTF8[rval as usize] as usize;
                if extra_bytes >= 4 {
                    bad_utf8_warning(st);
                    return 0xFFFD;
                }
                for _ in 0..extra_bytes {
                    let c = f.getc();
                    if !(0x80..0xC0).contains(&c) {
                        if c != EOF {
                            f.unget();
                        }
                        bad_utf8_warning(st);
                        return 0xFFFD;
                    }
                    rval = (rval << 6) + c;
                }
                let offset = OFFSETS_FROM_UTF8[extra_bytes];
                rval = (rval as u32).wrapping_sub(offset) as i32;
                if !(0..=0x10ffff).contains(&rval) {
                    bad_utf8_warning(st);
                    return 0xFFFD;
                }
            }
            rval
        }

        Encoding::Utf16Be | Encoding::Utf16Le => {
            let unit = |f: &mut InputFile, first: i32| {
                let second = f.getc();
                if encoding == Encoding::Utf16Be {
                    (first << 8) + second
                } else {
                    first + (second << 8)
                }
            };
            let mut rval = f.getc();
            if rval != EOF {
                rval = unit(f, rval);
                if (0xd800..=0xdbff).contains(&rval) {
                    let first = f.getc();
                    let lo = unit(f, first);
                    if (0xdc00..=0xdfff).contains(&lo) {
                        rval = 0x10000 + (rval - 0xd800) * 0x400 + (lo - 0xdc00);
                    } else {
                        rval = 0xfffd;
                        f.saved = Some(lo);
                    }
                } else if (0xdc00..=0xdfff).contains(&rval) {
                    rval = 0xfffd;
                }
            }
            rval
        }

        Encoding::Raw => f.getc(),
    }
}

/// Closes an input slot. Returns `true` on a serious internal close error.
#[must_use]
pub fn u_close(slot: InputSlot) -> bool {
    match slot {
        InputSlot::Open(f) => f.close(),
        InputSlot::Empty | InputSlot::Terminal => false,
    }
}

/// Runs `path` back through the name scanner, giving the name of the file actually opened.
fn rescan_opened_name(st: &mut EngineState, path: &[u8]) -> Result<FileName, Abort> {
    scan_whole_name(st, utf16_units(path).into_iter().map(u32::from))
}

fn utf16_units(mut path: &[u8]) -> Vec<u16> {
    let mut units = Vec::with_capacity(path.len());
    while !path.is_empty() {
        let (c, n) = utf8::utf16_units(next_utf8_char(&mut path));
        units.extend_from_slice(&c[..n]);
    }
    units
}

pub(crate) fn open_or_close(st: &mut EngineState, open: bool) -> Result<(), Abort> {
    let n = usize::try_from(scan_four_bit_int(st)?).unwrap_or(0);

    if st.input.read_open[n] != CLOSED {
        let slot = mem::take(&mut st.input.read_file[n]);
        if u_close(slot) {
            return Err(Abort::InputClose);
        }
        st.input.read_open[n] = CLOSED;
    }

    if !open {
        return Ok(());
    }

    scan_optional_equals(st)?;
    let path = scan_name_path(st)?;

    let mode = st.eqtb.int_par(IntPar::XetexDefaultInputMode);
    let encoding = st.eqtb.int_par(IntPar::XetexDefaultInputEncoding);
    if let Some((file, _)) = u_open_in(st, &path, FileFormat::Tex, mode, encoding)? {
        st.input.read_file[n] = InputSlot::Open(file);
        rescan_opened_name(st, &path)?;
        st.input.read_open[n] = JUST_OPEN;
    }

    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileName {
    pub name: StrNumber,
    pub area: StrNumber,
    pub ext: StrNumber,
}

impl FileName {
    /// The name as one path: area, name, then extension. Like TeX Live's C
    /// string of the joined name, it ends at the first NUL.
    pub(crate) fn path(self, strings: &StringsState) -> Vec<u8> {
        let mut path: Vec<u8> = [self.area, self.name, self.ext]
            .into_iter()
            .flat_map(|part| strings.texstring_utf8(part))
            .collect();
        if let Some(nul) = path.iter().position(|&b| b == 0) {
            path.truncate(nul);
        }
        path
    }
}

impl TexPrint for FileName {
    fn print_to(&self, st: &mut EngineState) {
        print_file_name(st, self.name, self.area, self.ext);
    }
}

/// Whether TeX may write `\openout` file `path`: kpathsea's default `openout_any = p` refuses
/// absolute names, `..` steps and names starting with a dot, and the directory must exist.
pub(crate) fn may_write(path: &[u8]) -> bool {
    if !stays_in_output_dir(path) {
        return false;
    }
    match path.iter().rposition(|&b| b == b'/') {
        Some(slash) => directory_exists(&path[..slash]),
        None => true,
    }
}

/// The name part of [`may_write`]. A backslash counts as a separator on every
/// platform, since Windows resolves it as one.
fn stays_in_output_dir(path: &[u8]) -> bool {
    let is_sep = |b: &u8| *b == b'/' || *b == b'\\';
    let components: Vec<&[u8]> = path.split(is_sep).collect();
    let absolute = path.first().is_some_and(is_sep) || path.get(1) == Some(&b':');
    // Only a `./` step may start with a dot.
    let last = components.len() - 1;
    let refused = components
        .iter()
        .enumerate()
        .any(|(i, c)| c.first() == Some(&b'.') && !(*c == b"." && i < last));
    !absolute && !refused
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct ScannedName {
    pub file: FileName,
    /// Whether a quote appeared, which makes a font name try installed fonts first.
    pub quoted: bool,
    pub open_quote: Option<u8>,
}

struct NameScan {
    area_delimiter: i32,
    ext_delimiter: i32,
    quoted: bool,
    quote: Option<u8>,
    stop_at_space: bool,
}

impl NameScan {
    fn begin_name(stop_at_space: bool) -> Self {
        Self {
            area_delimiter: 0,
            ext_delimiter: 0,
            quoted: false,
            quote: None,
            stop_at_space,
        }
    }

    fn more_name(&mut self, st: &mut EngineState, c: u32) -> Result<bool, Abort> {
        if self.stop_at_space {
            match self.quote {
                None if c == u32::from(b' ') => return Ok(false),
                Some(q) if c == u32::from(q) => {
                    self.quote = None;
                    return Ok(true);
                }
                None => {
                    if let Some(q) = b"\"'".iter().copied().find(|&q| c == u32::from(q)) {
                        self.quote = Some(q);
                        self.quoted = true;
                        return Ok(true);
                    }
                }
                Some(_) => {}
            }
        }
        let (units, n) = utf8::utf16_units(c);
        if st.strings.pool_ptr.index() + n > st.strings.pool_size.index() {
            return Err(st.strings.pool_overflow());
        }
        st.strings.append_units(&units[..n]);
        if c == u32::from(b'/') {
            self.area_delimiter = st.strings.cur_length();
            self.ext_delimiter = 0;
        } else if c == u32::from(b'.') {
            self.ext_delimiter = st.strings.cur_length();
        }
        Ok(true)
    }

    fn end_name(self, st: &mut EngineState) -> Result<ScannedName, Abort> {
        if st.strings.str_ptr.offset(3) > st.strings.max_strings {
            return Err(st.strings.string_overflow());
        }

        let area_delimiter = self.area_delimiter;
        let mut area = EMPTY_STRING;
        if area_delimiter != 0 {
            let strings = &mut st.strings;
            area = strings.str_ptr;
            let start = strings.cur_string_start();
            strings.str_start[strings.str_ptr.next().slot()] = start + area_delimiter;
            strings.str_ptr = strings.str_ptr.next();
            let temp_str = strings.search_string(area);
            if temp_str > StrNumber::NONE {
                area = temp_str;
                let base = strings.str_start[strings.str_ptr.slot()];
                strings.str_ptr = strings.str_ptr.prev();
                for j in base.until(strings.pool_ptr) {
                    strings.str_pool[(j - area_delimiter).index()] = strings.str_pool[j.index()];
                }
                strings.pool_ptr -= area_delimiter;
            }
        }

        let (name, ext) = if self.ext_delimiter == 0 {
            (st.strings.slow_make_string()?, EMPTY_STRING)
        } else {
            let strings = &mut st.strings;
            let shift = self.ext_delimiter - area_delimiter - 1;
            let mut name = strings.str_ptr;
            let start = strings.cur_string_start();
            strings.str_start[strings.str_ptr.next().slot()] = start + shift;
            strings.str_ptr = strings.str_ptr.next();

            strings.make_string()?;
            strings.str_ptr = strings.str_ptr.prev();
            let temp_str = strings.search_string(name);
            if temp_str > StrNumber::NONE {
                name = temp_str;
                let base = strings.str_start[strings.str_ptr.slot()];
                strings.str_ptr = strings.str_ptr.prev();
                for j in base.until(strings.pool_ptr) {
                    strings.str_pool[(j - shift).index()] = strings.str_pool[j.index()];
                }
                strings.pool_ptr -= shift;
            }
            (name, strings.slow_make_string()?)
        };
        Ok(ScannedName {
            file: FileName { name, area, ext },
            quoted: self.quoted,
            open_quote: self.quote,
        })
    }
}

fn scan_whole_name(
    st: &mut EngineState,
    chars: impl IntoIterator<Item = u32>,
) -> Result<FileName, Abort> {
    let mut scan = NameScan::begin_name(false);
    for c in chars {
        scan.more_name(st, c)?;
    }
    Ok(scan.end_name(st)?.file)
}

/// `path` as a pool string, also run through the name scanner as WEB does.
pub(crate) fn make_name_string(st: &mut EngineState, path: &[u8]) -> Result<StrNumber, Abort> {
    if st.strings.pool_ptr + path.len() as i32 > st.strings.pool_size
        || st.strings.str_ptr == st.strings.max_strings
        || st.strings.cur_length() > 0
    {
        return Ok(StrNumber::single_char(i32::from(b'?')));
    }

    let name16 = utf16_units(path);
    st.strings.append_units(&name16);

    let result = st.strings.make_string()?;
    scan_whole_name(st, name16.into_iter().map(u32::from))?;
    Ok(result)
}

/// The job name with extension `s`. The extension becomes a pool string, as in WEB.
pub(crate) fn pack_job_name(st: &mut EngineState, s: &str) -> Result<Vec<u8>, Abort> {
    let ext = st.strings.make_tex_string(s)?;
    let file = FileName {
        name: st.strings.job_name,
        area: EMPTY_STRING,
        ext,
    };
    Ok(file.path(&st.strings))
}

pub(crate) fn open_log_file(st: &mut EngineState) -> Result<(), Abort> {
    let old_setting = st.shipout.selector;
    if st.strings.job_name == StrNumber::NONE {
        st.strings.job_name = st.strings.make_tex_string("texput")?;
    }

    let log_path = pack_job_name(st, ".log")?;

    let handle = output_open(&log_path, false);
    if handle == INVALID_HANDLE {
        return Err(Abort::Internal("cannot open log file output"));
    }
    st.shipout.log_file = handle;
    st.shipout.log_bytes = 0;
    st.shipout.log_resumed_at = None;

    st.strings.texmf_log_name = make_name_string(st, &log_path)?;
    st.shipout.selector = Selector::LogOnly;
    st.errors.log_opened = true;

    st.input.input_stack.park(st.input.cur_input);

    print_nl(st, "**");
    let mut l = st.input.input_stack[0].limit;
    if st.input.buffer[l] == st.eqtb.int_par(IntPar::EndLineChar) {
        l -= 1;
    }
    for k in BufPos::from_raw(1).until(l + 1) {
        print(st, StrNumber::single_char(st.input.buffer[k]));
    }

    print_ln(st);
    st.shipout.selector = old_setting.with_log();
    Ok(())
}

/// Recreates bridge-owned output handles after restoring a resident checkpoint.
///
/// Checkpoints keep the logical output state, but bridge handles and
/// diagnostic pointers are valid only for the operation that created them.
pub(crate) fn reopen_resident_outputs(st: &mut EngineState) -> Result<(), Abort> {
    st.diagnostics.resume_after(st.shipout.diagnostics_issued);
    st.shipout.write_file = [INVALID_HANDLE; 16];
    st.shipout.log_file = INVALID_HANDLE;
    st.shipout.dvi.file = INVALID_HANDLE;

    if st.errors.log_opened {
        let log_path = pack_job_name(st, ".log")?;
        let handle = output_open(&log_path, false);
        if handle == INVALID_HANDLE {
            return Err(Abort::Internal("cannot reopen log file output"));
        }
        st.shipout.log_file = handle;
        st.shipout.log_resumed_at = Some(st.shipout.log_bytes);
    }

    if st.shipout.output_file_name != StrNumber::NONE {
        let dvi_path = pack_job_name(st, output_file_extension(st))?;
        let handle = output_open(&dvi_path, false);
        if handle == INVALID_HANDLE {
            return Err(Abort::Internal("cannot reopen XDV file output"));
        }
        st.shipout.dvi.file = handle;
    }

    Ok(())
}

pub(crate) fn close_files_and_terminate(st: &mut EngineState) -> Result<(), Abort> {
    for k in 0usize..=15 {
        if st.shipout.write_open[k] {
            ensure_write_stream_open(st, k)?;
            output_close(st.shipout.write_file[k]);
        }
    }

    // Routed through the journaling setter so a rewind restores `\newlinechar`.
    st.eqtb.set_int_par(IntPar::NewLineChar, -1);

    finalize_dvi_file(st)?;
    synctex_terminate(st);

    if st.errors.log_opened {
        log_putc(st, b'\n');
        output_close(st.shipout.log_file);
        st.shipout.selector = st.shipout.selector.without_log();
        if st.shipout.selector == Selector::TermOnly {
            print_nl(st, "Transcript written on ");
            print(st, st.strings.texmf_log_name);
            print_char(st, b'.');
        }
    }

    print_ln(st);
    Ok(())
}

pub fn init_io(st: &mut EngineState) {
    let first = st.input.first;
    st.input.input_file[0] = InputSlot::Terminal;
    st.input.buffer[first] = 0;
    st.input.last = first;
    st.input.cur_input.loc = first;
    st.input.cur_input.limit = first;
    st.input.first = first + 1;
}

pub(crate) fn scan_name_path(st: &mut EngineState) -> Result<Vec<u8>, Abort> {
    Ok(scan_file_name(st)?.file.path(&st.strings))
}

fn scan_file_name_braced(st: &mut EngineState) -> Result<NameScan, Abort> {
    let save_scanner_status = st.scanner.scanner_status;
    let save_def_ref = st.scanner.def_ref;
    let save_cur_cs = st.scanner.cur_cs;

    st.scanner.cur_cs = st.scanner.warning_index;
    scan_toks(st, false, true)?;

    st.shipout.old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    let arg = st.mem.tokens.cursor(st.scanner.def_ref);
    let l = st.strings.pool_size - st.strings.pool_ptr;
    show_token_list(st, arg, TokCursor::END, l);
    st.shipout.selector = st.shipout.old_setting;

    let s = st.strings.make_string()?;
    st.mem.tokens.delete_ref(st.scanner.def_ref);
    st.scanner.def_ref = save_def_ref;
    st.scanner.cur_cs = save_cur_cs;
    st.scanner.scanner_status = save_scanner_status;

    let mut scan = NameScan::begin_name(false);
    for i in st.strings.str_range(s) {
        let c = st.strings.str_pool[i];
        scan.more_name(st, u32::from(c))?;
    }
    Ok(scan)
}

pub(crate) fn scan_file_name(st: &mut EngineState) -> Result<ScannedName, Abort> {
    let save_warning_index = st.scanner.warning_index;
    st.scanner.warning_index = st.scanner.cur_cs;

    loop {
        get_x_token(st)?;
        if st.scanner.cur.cmd() != Cmd::Spacer && st.scanner.cur.cmd() != Cmd::Relax {
            break;
        }
    }

    back_input(st)?;

    let scan = if st.scanner.cur.cmd() == Cmd::LeftBrace {
        scan_file_name_braced(st)?
    } else {
        st.scanner.name_in_progress = true;
        let mut scan = NameScan::begin_name(true);

        loop {
            get_x_token(st)?;
            if st.scanner.cur.cmd() != Cmd::Spacer {
                break;
            }
        }

        loop {
            let (cmd, chr) = st.scanner.cur.encode();
            let c = match u32::try_from(chr) {
                Ok(c) if cmd <= Cmd::OtherChar && chr <= BIGGEST_USV => c,
                _ => {
                    back_input(st)?;
                    break;
                }
            };

            if !scan.more_name(st, c)? {
                break;
            }

            get_x_token(st)?;
        }
        scan
    };

    let scanned = scan.end_name(st)?;
    st.scanner.name_in_progress = false;
    st.scanner.warning_index = save_warning_index;
    Ok(scanned)
}

pub(crate) fn start_input(
    st: &mut EngineState,
    primary_input_name: Option<&str>,
) -> Result<(), Abort> {
    let format = if primary_input_name.is_none() {
        FileFormat::Tex
    } else {
        FileFormat::TexyonPrimary
    };

    let scanned = if let Some(name) = primary_input_name {
        let mut name = name.as_bytes();
        if st.strings.pool_ptr + name.len() as i32 * 2 >= st.strings.pool_size {
            return Err(Abort::Internal("string pool overflow"));
        }
        let chars = iter::from_fn(|| (!name.is_empty()).then(|| next_utf8_char(&mut name)));
        scan_whole_name(st, chars)?
    } else {
        scan_file_name(st)?.file
    };

    if primary_input_name.is_some() {
        // TeX Live scans this name from the terminal line, which leaves none of it to read.
        st.input.cur_input.loc = st.input.cur_input.limit + 1;
    }
    let path = scanned.path(&st.strings);
    begin_file_reading(st)?;

    let idx = i32::from(st.input.cur_input.index);
    let mode = st.eqtb.int_par(IntPar::XetexDefaultInputMode);
    let encoding = st.eqtb.int_par(IntPar::XetexDefaultInputEncoding);
    let Some((file, found)) = u_open_in(st, &path, format, mode, encoding)? else {
        end_file_reading(st)?;
        return Err(prompt_file_name(st, FilePrompt::Input, scanned, &path));
    };
    st.input.input_file[idx as usize] = InputSlot::Open(file);

    let opened = rescan_opened_name(st, &path)?;

    let name = make_name_string(st, &path)?;
    st.input.cur_input.name = InputName::File(name);
    let in_open = st.input.in_open;
    st.input.open_levels[in_open].source_name = name;
    let full_name = st.strings.make_tex_string(&found.name)?;
    st.input.open_levels[in_open].full_source_name = full_name;

    if name == st.strings.str_ptr.prev() {
        let temp_str = st.strings.search_string(name);
        if temp_str > StrNumber::NONE {
            st.input.cur_input.name = InputName::File(temp_str);
            st.strings.flush_string();
        }
    }

    if st.strings.job_name == StrNumber::NONE {
        st.strings.job_name = opened.name;
        open_log_file(st)?;
    }

    if st.shipout.term_offset + st.strings.length(full_name) > st.config.max_print_line - 2 {
        print_ln(st);
    } else if st.shipout.term_offset > 0 || st.shipout.file_offset > 0 {
        print_char(st, b' ');
    }
    print_char(st, b'(');
    st.input.open_parens += 1;
    print(st, full_name);
    output_flush(st.shipout.rust_stdout);

    let tracing_stack_levels = st.eqtb.int_par(IntPar::TracingStackLevels);
    if tracing_stack_levels > 0 {
        begin_diagnostic(st);
        print_ln(st);
        print_char(st, b'~');
        let v = st.input.input_depth() - 1;
        if v < tracing_stack_levels {
            for _ in 0..v {
                print_char(st, b'.');
            }
        } else {
            print_char(st, b'~');
        }
        print(st, "INPUT ");
        print(st, (opened.name, opened.ext));
        print_ln(st);
        end_diagnostic(st, false);
    }

    st.input.cur_input.state = NEW_LINE;
    let job_name = st.strings.texstring_bytes(st.strings.job_name);
    synctex_start_input(st, &job_name, &found.abspath);
    st.mem.line = 1;
    input_slot_line(st, FileSlot::Input(idx as usize))?;
    st.input.cur_input.limit = st.input.last;
    finish_line(st);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::stays_in_output_dir;

    #[test]
    fn output_names_stay_in_output_dir() {
        for ok in ["a.tex", "./a.tex", "sub/a.tex", "sub\\a.tex", "a.b.tex"] {
            assert!(stays_in_output_dir(ok.as_bytes()), "{ok}");
        }
        for bad in [
            "/etc/x",
            "C:x",
            "../x",
            "sub/../../x",
            ".hidden",
            "\\x",
            "\\\\server\\share\\x",
            "sub\\..\\..\\x",
            "sub\\.hidden",
        ] {
            assert!(!stays_in_output_dir(bad.as_bytes()), "{bad}");
        }
    }
}
