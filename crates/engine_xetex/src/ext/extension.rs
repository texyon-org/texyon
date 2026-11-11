//! Extensions: `\openout`, `\write`, `\closeout`, `\special`, pictures and source specials.

use crate::cmd::Cmd;
use crate::codes::ExtensionCode;
use crate::command::Command;
use crate::consts::{FROZEN_SPECIAL, HMODE, MMODE, VMODE};
use crate::errors::Abort;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::input::TokenType;
use crate::io::AUTO;
use crate::main_control::new_graf;
use crate::node::{
    GlyphNode, LanguageNode, NodePtr, OpenNode, SavePosNode, WhatsitKind, WhatsitNode,
    WhatsitSubtype, WriteNode,
};
use crate::params::IntPar;
use crate::pic::load_picture;
use crate::scan::{get_x_token, scan_int};
use crate::state::EngineState;
use crate::token::{Token, TokenList};
use crate::units::StrNumber;

use super::get_encoding_mode_and_info;

/// The current source file if the input position moved since the last source
/// special.
fn new_source_position(st: &mut EngineState) -> Option<StrNumber> {
    let fname = st.input.open_levels[st.input.in_open].source_name;
    (fname > StrNumber::NONE && crate::texmfmp::is_new_source(st, fname, st.mem.line))
        .then_some(fname)
}

/// Append a `\special{src:...}` whatsit for the current source line.
pub(crate) fn append_src_special(st: &mut EngineState) -> Result<(), Abort> {
    let Some(fname) = new_source_position(st) else {
        return Ok(());
    };
    let w = st
        .input
        .cur_list
        .append_whatsit::<WriteNode>(&mut st.mem, WhatsitSubtype::Special)?;
    st.mem[w].stream = 0;
    let b = crate::texmfmp::make_src_special(st, fname, st.mem.line)
        .map_err(|()| Abort::Internal("string pool overflow"))?;
    let d = st.mem.tokens.begin()?;
    st.mem.str_toks_cat(&mut st.strings, b, None, &d)?;
    let d = st.mem.tokens.finish(d)?;
    st.scanner.def_ref = d;
    st.mem[w].tokens = d;
    crate::texmfmp::remember_source_info(st, fname, st.mem.line);
    Ok(())
}

/// `\special{...}` around `str_pool[b..pool_ptr]`, which is dropped from the pool.
fn special_list(st: &mut EngineState, b: crate::units::PoolPos) -> Result<TokenList, Abort> {
    let out = st.mem.tokens.begin()?;
    st.mem.tokens.push(&out, Token::from_cs(FROZEN_SPECIAL))?;
    st.mem.tokens.push(&out, Token::left_brace(b'{'))?;
    st.mem.str_toks_cat(&mut st.strings, b, None, &out)?;
    st.mem.tokens.push(&out, Token::right_brace(b'}'))?;
    st.mem.tokens.finish(out)
}

/// Insert `\special{src:...}` for the current source line into the input.
pub(crate) fn insert_src_special(st: &mut EngineState) -> Result<(), Abort> {
    let Some(fname) = new_source_position(st) else {
        return Ok(());
    };
    let b = crate::texmfmp::make_src_special(st, fname, st.mem.line)
        .map_err(|()| Abort::Internal("string pool overflow"))?;
    let toklist = special_list(st, b)?;
    crate::input::begin_token_list(st, toklist, TokenType::Inserted)?;
    crate::texmfmp::remember_source_info(st, fname, st.mem.line);
    Ok(())
}

/// Insert `\special{<ascii_text>}` into the input.
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
    crate::input::begin_token_list(st, toklist, TokenType::Inserted)?;
    Ok(())
}

fn append_whatsit<K: WhatsitKind + crate::node::Node>(
    st: &mut EngineState,
    subtype: WhatsitSubtype,
) -> Result<NodePtr<K>, Abort> {
    st.input.cur_list.append_whatsit(&mut st.mem, subtype)
}

/// The stream number of an `\openout` (`open`), or of a `\write` or `\closeout`.
fn scan_write_stream(st: &mut EngineState, open: bool) -> Result<i32, Abort> {
    Ok(if open {
        crate::scan::scan_four_bit_int(st)?
    } else {
        match scan_int(st)? {
            n if n < 0 => 17,
            n if n > 15 && n != 18 => 16,
            n => n,
        }
    })
}

/// Appends the `\openout`, `\write` or `\closeout` whatsit for `cur_chr`.
fn io_whatsit(st: &mut EngineState, code: ExtensionCode) -> Result<NodePtr<WhatsitNode>, Abort> {
    Ok(match code {
        ExtensionCode::Open => {
            let w = append_whatsit::<OpenNode>(st, WhatsitSubtype::Open)?;
            st.mem[w].stream = scan_write_stream(st, true)?;
            crate::scan::scan_optional_equals(st)?;
            let file = crate::io::scan_file_name(st)?.file;
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
            crate::scan::scan_toks(st, false, false)?;
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
    let mode = st.input.cur_list.mode.abs();
    match code {
        ExtensionCode::Open | ExtensionCode::Write | ExtensionCode::Close => {
            io_whatsit(st, code)?;
        }

        ExtensionCode::Special => {
            let late = crate::scan::scan_keyword(st, "shipout")?;
            let subtype = if late {
                WhatsitSubtype::LateSpecial
            } else {
                WhatsitSubtype::Special
            };
            let w = append_whatsit::<WriteNode>(st, subtype)?;
            st.mem[w].stream = crate::mem::TEX_NULL;
            crate::scan::scan_toks(st, false, !late)?;
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
                crate::shipout::out_what(st, w)?;
                st.mem.flush_node_list(Some(st.input.cur_list.tail))?;
                st.input.cur_list.tail = p;
                p.set_link(&mut st.mem, None);
            } else {
                crate::input::back_input(st)?;
            }
        }

        ExtensionCode::SetLanguage => {
            if mode != HMODE {
                crate::errors::report_illegal_case(st)?;
            } else {
                let w = append_whatsit::<LanguageNode>(st, WhatsitSubtype::Language)?;
                let lang = match scan_int(st)? {
                    n @ 1..=255 => n,
                    _ => 0,
                };
                st.input.cur_list.set_clang(lang);
                let lhm = crate::helpers::norm_min(st.eqtb.int_par(IntPar::LeftHyphenMin));
                let rhm = crate::helpers::norm_min(st.eqtb.int_par(IntPar::RightHyphenMin));
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
                (st.shipout.epochseconds, st.shipout.microseconds) =
                    crate::texmfmp::get_seconds_and_micros();
            }
        }

        ExtensionCode::SetRandomSeed => {
            let seed = scan_int(st)?.wrapping_abs();
            st.random.reseed(seed);
        }

        ExtensionCode::PicFile | ExtensionCode::PdfFile => {
            if mode == MMODE {
                crate::errors::report_illegal_case(st)?;
            } else {
                load_picture(st, code == ExtensionCode::PdfFile)?;
            }
        }

        ExtensionCode::Glyph => {
            if mode == VMODE {
                crate::input::back_input(st)?;
                new_graf(st, true)?;
            } else if mode == MMODE {
                crate::errors::report_illegal_case(st)?;
            } else {
                let cur_font = st.eqtb.cur_font();
                let area = st.fonts.font_area[cur_font];
                if area != AAT_FONT_FLAG && area != OTGR_FONT_FLAG {
                    return crate::errors::not_font_error_common(
                        st,
                        Cmd::Extension,
                        ExtensionCode::Glyph.raw(),
                        cur_font,
                        "; not a native platform font",
                    );
                }
                let w = append_whatsit::<GlyphNode>(st, WhatsitSubtype::Glyph)?;
                let n = scan_int(st)?;
                let glyph = match u16::try_from(n) {
                    Ok(glyph) => glyph,
                    Err(_) => {
                        crate::errors::error(
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
            let name = crate::io::scan_name_path(st)?;
            let (enc_mode, info) = get_encoding_mode_and_info(st, &name)?;
            if enc_mode == AUTO {
                crate::errors::error(
                    st,
                    "Encoding mode `auto' is not valid for \\XeTeXinputencoding",
                    &[
                        r"You can't use `auto' encoding here, only for \XeTeXdefaultencoding.",
                        "I'll ignore this and leave the current encoding unchanged.",
                    ],
                )?;
            } else {
                // `\scantokens` occupies an input level without a UFile.
                let slot = crate::io::FileSlot::Input(st.input.in_open);
                crate::io::with_slot_file(st, slot, |st, f| {
                    crate::io::set_input_file_encoding(st, f, enc_mode, info)
                })
                .transpose()?;
            }
        }

        ExtensionCode::DefaultEncoding => {
            let name = crate::io::scan_name_path(st)?;
            let (enc_mode, info) = get_encoding_mode_and_info(st, &name)?;
            st.eqtb.set_int_par(IntPar::XetexDefaultInputMode, enc_mode);
            st.eqtb.set_int_par(IntPar::XetexDefaultInputEncoding, info);
        }

        ExtensionCode::LinebreakLocale => {
            let name = crate::io::scan_file_name(st)?.file.name;
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
