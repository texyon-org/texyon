//! Extensions: `\openout`, `\write`, `\closeout`, `\special`, pictures and source specials.

use crate::base::codes::ExtensionCode;
use crate::base::command::Command;
use crate::base::params::IntPar;
use crate::base::token::{Token, TokenList};
use crate::base::units::{PoolPos, StrNumber};
use crate::build::hyphenation::norm_min;
use crate::build::main_control::new_graf;
use crate::ffi::clock::get_seconds_and_micros;
use crate::files::io::{AUTO, FileSlot, scan_file_name, scan_name_path, set_slot_encoding};
use crate::fonts::ext::get_encoding_mode_and_info;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::memory::mem::TEX_NULL;
use crate::memory::node::{
    GlyphNode, LanguageNode, Node, NodePtr, OpenNode, SavePosNode, WhatsitKind, WhatsitNode,
    WhatsitSubtype, WriteNode,
};
use crate::print::errors::{Abort, error, not_font_error_common, report_illegal_case};
use crate::scanning::input::{TokenType, back_input, begin_token_list};
use crate::scanning::scan::{
    get_x_token, scan_four_bit_int, scan_int, scan_keyword, scan_optional_equals, scan_toks,
};
use crate::shipout::out_what;
use crate::shipout::pic::load_picture;
use crate::state::EngineState;
use crate::tables::cs::Frozen;

fn new_source_position(st: &mut EngineState) -> Option<StrNumber> {
    let fname = st.input.open_levels[st.input.in_open].source_name;
    (fname > StrNumber::NONE && is_new_source(st, fname, st.mem.line)).then_some(fname)
}

pub(crate) fn append_src_special(st: &mut EngineState) -> Result<(), Abort> {
    let Some(fname) = new_source_position(st) else {
        return Ok(());
    };
    let w = st
        .input
        .cur_list
        .append_whatsit::<WriteNode>(&mut st.mem, WhatsitSubtype::Special)?;
    st.mem[w].stream = 0;
    let b = make_src_special(st, fname, st.mem.line)
        .map_err(|()| Abort::Internal("string pool overflow"))?;
    let d = st.mem.tokens.begin()?;
    st.mem.str_toks_cat(&mut st.strings, b, None, &d)?;
    let d = st.mem.tokens.finish(d)?;
    st.scanner.def_ref = d;
    st.mem[w].tokens = d;
    remember_source_info(st, fname, st.mem.line);
    Ok(())
}

/// `\special{...}` around `str_pool[b..pool_ptr]`, which is dropped from the pool.
fn special_list(st: &mut EngineState, b: PoolPos) -> Result<TokenList, Abort> {
    let out = st.mem.tokens.begin()?;
    st.mem
        .tokens
        .push(&out, Token::from_cs(Frozen::Special.cs()))?;
    st.mem.tokens.push(&out, Token::left_brace(b'{'))?;
    st.mem.str_toks_cat(&mut st.strings, b, None, &out)?;
    st.mem.tokens.push(&out, Token::right_brace(b'}'))?;
    st.mem.tokens.finish(out)
}

pub(crate) fn insert_src_special(st: &mut EngineState) -> Result<(), Abort> {
    let Some(fname) = new_source_position(st) else {
        return Ok(());
    };
    let b = make_src_special(st, fname, st.mem.line)
        .map_err(|()| Abort::Internal("string pool overflow"))?;
    let toklist = special_list(st, b)?;
    begin_token_list(st, toklist, TokenType::Inserted)?;
    remember_source_info(st, fname, st.mem.line);
    Ok(())
}

pub(crate) fn insert_special(st: &mut EngineState, text: &str) -> Result<(), Abort> {
    let start_pool = st.strings.pool_ptr;
    let text = text.as_bytes();
    if st.strings.pool_ptr + text.len() as i32 >= st.strings.pool_size {
        return Err(st.strings.pool_overflow());
    }
    for &b in text {
        st.strings.append_char(u16::from(b));
    }

    let toklist = special_list(st, start_pool)?;
    begin_token_list(st, toklist, TokenType::Inserted)?;
    Ok(())
}

fn append_whatsit<K: WhatsitKind + Node>(
    st: &mut EngineState,
    subtype: WhatsitSubtype,
) -> Result<NodePtr<K>, Abort> {
    st.input.cur_list.append_whatsit(&mut st.mem, subtype)
}

fn scan_write_stream(st: &mut EngineState, open: bool) -> Result<i32, Abort> {
    Ok(if open {
        scan_four_bit_int(st)?
    } else {
        match scan_int(st)? {
            n if n < 0 => 17,
            n if n > 15 && n != 18 => 16,
            n => n,
        }
    })
}

fn io_whatsit(st: &mut EngineState, code: ExtensionCode) -> Result<NodePtr<WhatsitNode>, Abort> {
    Ok(match code {
        ExtensionCode::Open => {
            let w = append_whatsit::<OpenNode>(st, WhatsitSubtype::Open)?;
            st.mem[w].stream = scan_write_stream(st, true)?;
            scan_optional_equals(st)?;
            let file = scan_file_name(st)?.file;
            let open = &mut st.mem[w];
            open.name = file.name;
            open.area = file.area;
            open.ext = file.ext;
            w.whatsit()
        }
        ExtensionCode::Write => {
            let k = st.scanner.cur_cs;
            let w = append_whatsit::<WriteNode>(st, WhatsitSubtype::Write)?;
            st.mem[w].stream = scan_write_stream(st, false)?;
            st.scanner.cur_cs = k;
            scan_toks(st, false, false)?;
            st.mem[w].tokens = st.scanner.def_ref;
            w.whatsit()
        }
        _ => {
            let w = append_whatsit::<WriteNode>(st, WhatsitSubtype::Close)?;
            st.mem[w].stream = scan_write_stream(st, false)?;
            st.mem[w].tokens = TokenList::NULL;
            w.whatsit()
        }
    })
}

pub(crate) fn do_extension(st: &mut EngineState, code: ExtensionCode) -> Result<(), Abort> {
    let mode = st.input.cur_list.mode;
    match code {
        ExtensionCode::Open | ExtensionCode::Write | ExtensionCode::Close => {
            io_whatsit(st, code)?;
        }

        ExtensionCode::Special => {
            let late = scan_keyword(st, "shipout")?;
            let subtype = if late {
                WhatsitSubtype::LateSpecial
            } else {
                WhatsitSubtype::Special
            };
            let w = append_whatsit::<WriteNode>(st, subtype)?;
            st.mem[w].stream = TEX_NULL;
            scan_toks(st, false, !late)?;
            st.mem[w].tokens = st.scanner.def_ref;
        }

        ExtensionCode::Immediate => {
            get_x_token(st)?;
            let io_code = match st.scanner.cur {
                Command::Extension(
                    c @ (ExtensionCode::Open | ExtensionCode::Write | ExtensionCode::Close),
                ) => Some(c),
                _ => None,
            };
            if let Some(io_code) = io_code {
                let p = st.input.cur_list.tail;
                let w = io_whatsit(st, io_code)?;
                out_what(st, w)?;
                st.mem.flush_node_list(Some(st.input.cur_list.tail))?;
                st.input.cur_list.tail = p;
                p.set_link(&mut st.mem, None);
            } else {
                back_input(st)?;
            }
        }

        ExtensionCode::SetLanguage => {
            if !mode.is_horizontal() {
                report_illegal_case(st)?;
            } else {
                let w = append_whatsit::<LanguageNode>(st, WhatsitSubtype::Language)?;
                let lang = match scan_int(st)? {
                    n @ 1..=255 => n,
                    _ => 0,
                };
                st.input.cur_list.set_clang(lang)?;
                let lhm = norm_min(st.eqtb.int_par(IntPar::LeftHyphenMin));
                let rhm = norm_min(st.eqtb.int_par(IntPar::RightHyphenMin));
                let language = &mut st.mem[w];
                language.lang = lang;
                language.lhm = lhm as u16;
                language.rhm = rhm as u16;
            }
        }

        ExtensionCode::PdfSavePos => {
            st.input
                .cur_list
                .append_whatsit::<SavePosNode>(&mut st.mem, WhatsitSubtype::PdfSavePos)?;
        }

        ExtensionCode::ResetTimer => {
            if !st.config.deterministic_mode {
                (st.shipout.epochseconds, st.shipout.microseconds) = get_seconds_and_micros();
            }
        }

        ExtensionCode::SetRandomSeed => {
            let seed = scan_int(st)?.wrapping_abs();
            st.random.reseed(seed);
        }

        ExtensionCode::PicFile | ExtensionCode::PdfFile => {
            if mode.is_math() {
                report_illegal_case(st)?;
            } else {
                load_picture(st, code == ExtensionCode::PdfFile)?;
            }
        }

        ExtensionCode::Glyph => {
            if mode.is_vertical() {
                back_input(st)?;
                new_graf(st, true)?;
            } else if mode.is_math() {
                report_illegal_case(st)?;
            } else {
                let cur_font = st.eqtb.cur_font();
                let area = st.fonts.font_area[cur_font];
                if area != AAT_FONT_FLAG && area != OTGR_FONT_FLAG {
                    return not_font_error_common(
                        st,
                        Command::Extension(ExtensionCode::Glyph),
                        cur_font,
                        "; not a native platform font",
                    );
                }
                let w = append_whatsit::<GlyphNode>(st, WhatsitSubtype::Glyph)?;
                let n = scan_int(st)?;
                let glyph = match u16::try_from(n) {
                    Ok(glyph) => glyph,
                    Err(_) => {
                        error(
                            st,
                            ("Bad glyph number", " (", n, ")"),
                            &[
                                "A glyph number must be between 0 and 65535.",
                                "I changed this one to zero.",
                            ],
                        )?;
                        0
                    }
                };
                let node = &mut st.mem[w];
                node.font = cur_font;
                node.glyph = glyph;
                let use_glyph_metrics = st.eqtb.int_par(IntPar::XetexUseGlyphMetrics) > 0;
                st.fonts
                    .measure_native_glyph(&mut st.mem, w, use_glyph_metrics)?;
            }
        }

        ExtensionCode::InputEncoding => {
            let name = scan_name_path(st)?;
            let (enc_mode, info) = get_encoding_mode_and_info(st, &name)?;
            if enc_mode == AUTO {
                error(
                    st,
                    "Encoding mode `auto' is not valid for \\XeTeXinputencoding",
                    &[
                        r"You can't use `auto' encoding here, only for \XeTeXdefaultencoding.",
                        "I'll ignore this and leave the current encoding unchanged.",
                    ],
                )?;
            } else {
                // `\scantokens` occupies an input level without a file.
                set_slot_encoding(st, FileSlot::Input(st.input.in_open), enc_mode, info)?;
            }
        }

        ExtensionCode::DefaultEncoding => {
            let name = scan_name_path(st)?;
            let (enc_mode, info) = get_encoding_mode_and_info(st, &name)?;
            st.eqtb.set_int_par(IntPar::XetexDefaultInputMode, enc_mode);
            st.eqtb.set_int_par(IntPar::XetexDefaultInputEncoding, info);
        }

        ExtensionCode::LinebreakLocale => {
            let name = scan_file_name(st)?.file.name;
            let locale = if st.strings.length(name) == 0 {
                StrNumber::NONE
            } else {
                name
            };
            st.eqtb
                .set_int_par(IntPar::XetexLinebreakLocale, locale.raw());
        }
    }
    Ok(())
}

/// Writes "src:<lineno> <filename>" into the pool and returns its start, or
/// `Err` if the pool is too small.
fn make_src_special(
    st: &mut EngineState,
    srcfilename: StrNumber,
    lineno: i32,
) -> Result<PoolPos, ()> {
    let old_pool_ptr = st.strings.pool_ptr;
    let filename = st.strings.texstring_bytes(srcfilename);
    let prefix = format!("src:{} ", lineno);

    if st.strings.pool_ptr.index() + prefix.len() + filename.len() >= st.strings.pool_size.index() {
        return Err(());
    }
    st.strings.push_bytes(prefix.as_bytes());
    st.strings.push_bytes(&filename);
    Ok(old_pool_ptr)
}

fn is_new_source(st: &EngineState, srcfilename: StrNumber, lineno: i32) -> bool {
    let name = st.strings.texstring_bytes(srcfilename);
    let last_name = st
        .src_specials
        .last_source_name
        .as_deref()
        .unwrap_or_default();
    name != last_name || lineno != st.src_specials.last_lineno
}

fn remember_source_info(st: &mut EngineState, srcfilename: StrNumber, lineno: i32) {
    st.src_specials.last_source_name = Some(st.strings.texstring_bytes(srcfilename));
    st.src_specials.last_lineno = lineno;
}

#[derive(Default, Clone)]
pub(crate) struct SourceSpecials {
    pub last_source_name: Option<Vec<u8>>,
    pub last_lineno: i32,
}
