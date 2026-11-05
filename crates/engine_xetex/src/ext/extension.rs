//! Extensions: `\openout`, `\write`, `\closeout`, `\special`, pictures and source specials.

use crate::cmd::Cmd;
use crate::codes::ExtensionCode;
use crate::consts::{
    FROZEN_SPECIAL, GLYPH_NODE_SIZE, HMODE, MMODE, OPEN_NODE_SIZE, SMALL_NODE_SIZE, VMODE,
    WRITE_NODE_SIZE,
};
use crate::errors::Abort;
use crate::fonts::{AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::input::TokenType;
use crate::io::AUTO;
use crate::main_control::new_graf;
use crate::mem::TEMP_HEAD_TOKS;
use crate::node::{NodePtr, WhatsitNode, WhatsitSubtype};
use crate::output::{capture_to_diagnostic, error_here_with_diagnostic};
use crate::params::IntPar;
use crate::pic::load_picture;
use crate::scan::{get_x_token, scan_int};
use crate::state::EngineState;
use crate::token::{Token, TokenList};
use crate::units::{MemPtr, StrNumber};

use super::get_encoding_mode_and_info;

/// The current source file if the input position moved since the last source
/// special.
fn new_source_position(st: &mut EngineState) -> Option<StrNumber> {
    let fname = st.input.source_filename_stack[st.input.in_open as usize];
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
        .new_whatsit(&mut st.mem, WhatsitSubtype::Special, WRITE_NODE_SIZE)?;
    w.set_write_stream(&mut st.mem, 0);
    let d = st.mem.get_avail_token()?;
    st.scanner.def_ref = d;
    d.set_ref_count(&mut st.mem, crate::mem::TEX_NULL);
    let b = crate::texmfmp::make_src_special(st, fname, st.mem.line)
        .map_err(|()| Abort::Internal("string pool overflow"))?;
    st.mem.str_toks_cat(&mut st.strings, b, None)?;
    let text = TEMP_HEAD_TOKS.link(&st.mem);
    d.set_link(&mut st.mem, text);
    w.set_write_tokens(&mut st.mem, d);
    crate::texmfmp::remember_source_info(st, fname, st.mem.line);
    Ok(())
}

/// Insert `\special{src:...}` for the current source line into the input.
pub(crate) fn insert_src_special(st: &mut EngineState) -> Result<(), Abort> {
    let Some(fname) = new_source_position(st) else {
        return Ok(());
    };
    let toklist = st.mem.get_avail_token()?;
    toklist.set_token(&mut st.mem, Token::from_cs(FROZEN_SPECIAL));
    let open = st.mem.get_avail_token()?;
    toklist.set_link(&mut st.mem, open);
    open.set_token(&mut st.mem, Token::left_brace(b'{'));
    let b = crate::texmfmp::make_src_special(st, fname, st.mem.line)
        .map_err(|()| Abort::Internal("string pool overflow"))?;
    let text_tail = st.mem.str_toks_cat(&mut st.strings, b, None)?;
    let text = TEMP_HEAD_TOKS.link(&st.mem);
    open.set_link(&mut st.mem, text);
    let close = st.mem.get_avail_token()?;
    text_tail.set_link(&mut st.mem, close);
    close.set_token(&mut st.mem, Token::right_brace(b'}'));
    crate::input::begin_token_list(st, toklist, TokenType::Inserted)?;
    crate::texmfmp::remember_source_info(st, fname, st.mem.line);
    Ok(())
}

/// Insert `\special{<ascii_text>}` into the input.
pub(crate) fn insert_special(st: &mut EngineState, text: &str) -> Result<(), Abort> {
    let start_pool = st.strings.pool_ptr;
    let text = text.as_bytes();
    if st.strings.pool_ptr + text.len() as i32 >= st.strings.pool_size {
        return Err(Abort::Overflow {
            what: "string pool overflow",
            n: 0,
        });
    }
    for &b in text {
        st.strings.append_char(u16::from(b));
    }

    let toklist = st.mem.get_avail_token()?;
    toklist.set_token(&mut st.mem, Token::from_cs(FROZEN_SPECIAL));
    let open = st.mem.get_avail_token()?;
    open.set_token(&mut st.mem, Token::left_brace(b'{'));
    toklist.set_link(&mut st.mem, open);
    let text_tail = st.mem.str_toks_cat(&mut st.strings, start_pool, None)?;
    let text = TEMP_HEAD_TOKS.link(&st.mem);
    open.set_link(&mut st.mem, text);
    let close = st.mem.get_avail_token()?;
    close.set_token(&mut st.mem, Token::right_brace(b'}'));
    text_tail.set_link(&mut st.mem, close);
    crate::input::begin_token_list(st, toklist, TokenType::Inserted)?;
    Ok(())
}

fn new_whatsit(
    st: &mut EngineState,
    subtype: WhatsitSubtype,
    size: i32,
) -> Result<NodePtr<WhatsitNode>, Abort> {
    st.input.cur_list.new_whatsit(&mut st.mem, subtype, size)
}

/// Append an `\openout`/`\write`/`\closeout` whatsit and scan its stream number.
fn new_write_whatsit(
    st: &mut EngineState,
    subtype: WhatsitSubtype,
    size: i32,
) -> Result<NodePtr<WhatsitNode>, Abort> {
    let w = new_whatsit(st, subtype, size)?;
    if size != WRITE_NODE_SIZE {
        crate::scan::scan_four_bit_int(st)?;
    } else {
        scan_int(st)?;
        if st.scanner.cur_val < 0 {
            st.scanner.cur_val = 17;
        } else if st.scanner.cur_val > 15 && st.scanner.cur_val != 18 {
            st.scanner.cur_val = 16;
        }
    }
    w.set_write_stream(&mut st.mem, st.scanner.cur_val);
    Ok(w)
}

/// Appends the `\openout`, `\write` or `\closeout` whatsit for `cur_chr`.
fn io_whatsit(st: &mut EngineState, code: ExtensionCode) -> Result<NodePtr<WhatsitNode>, Abort> {
    Ok(match code {
        ExtensionCode::Open => {
            let w = new_write_whatsit(st, WhatsitSubtype::Open, OPEN_NODE_SIZE)?;
            crate::scan::scan_optional_equals(st)?;
            crate::io::scan_file_name(st)?;
            w.set_open_name(&mut st.mem, st.scanner.cur_name);
            w.set_open_area(&mut st.mem, st.scanner.cur_area);
            w.set_open_ext(&mut st.mem, st.scanner.cur_ext);
            w
        }
        ExtensionCode::Write => {
            let k = st.scanner.cur_cs;
            let w = new_write_whatsit(st, WhatsitSubtype::Write, WRITE_NODE_SIZE)?;
            st.scanner.cur_cs = k;
            crate::scan::scan_toks(st, false, false)?;
            w.set_write_tokens(&mut st.mem, st.scanner.def_ref);
            w
        }
        _ => {
            let w = new_write_whatsit(st, WhatsitSubtype::Close, WRITE_NODE_SIZE)?;
            w.set_write_tokens(&mut st.mem, TokenList::NULL);
            w
        }
    })
}

pub(crate) fn do_extension(st: &mut EngineState) -> Result<(), Abort> {
    let mode = st.input.cur_list.mode.abs();
    let code = ExtensionCode::from_raw(st.scanner.cur_chr).ok_or(Abort::Confusion("ext1"))?;
    match code {
        ExtensionCode::Open | ExtensionCode::Write | ExtensionCode::Close => {
            io_whatsit(st, code)?;
        }

        ExtensionCode::Special => {
            let w = new_whatsit(st, WhatsitSubtype::Special, WRITE_NODE_SIZE)?;
            w.set_write_stream(&mut st.mem, crate::mem::TEX_NULL);
            crate::scan::scan_toks(st, false, true)?;
            w.set_write_tokens(&mut st.mem, st.scanner.def_ref);
        }

        ExtensionCode::Immediate => {
            get_x_token(st)?;
            let io_code =
                match ExtensionCode::from_raw(st.scanner.cur_chr) {
                    Some(
                        c @ (ExtensionCode::Open | ExtensionCode::Write | ExtensionCode::Close),
                    ) if st.scanner.cur_cmd == Cmd::Extension => Some(c),
                    _ => None,
                };
            if let Some(io_code) = io_code {
                let p = st.input.cur_list.tail;
                let w = io_whatsit(st, io_code)?;
                crate::shipout::out_what(st, w)?;
                st.mem.flush_node_list(st.input.cur_list.tail)?;
                st.input.cur_list.tail = p;
                p.set_link(&mut st.mem, MemPtr::NULL);
            } else {
                crate::input::back_input(st)?;
            }
        }

        ExtensionCode::SetLanguage => {
            if mode != HMODE {
                crate::errors::report_illegal_case(st)?;
            } else {
                let w = new_whatsit(st, WhatsitSubtype::Language, SMALL_NODE_SIZE)?;
                scan_int(st)?;
                let lang = if st.scanner.cur_val <= 0 || st.scanner.cur_val > 255 {
                    0
                } else {
                    st.scanner.cur_val
                };
                st.input.cur_list.set_clang(lang);
                w.set_what_lang(&mut st.mem, lang);
                let lhm = crate::helpers::norm_min(st.eqtb.int_par(IntPar::LeftHyphenMin));
                w.set_what_lhm(&mut st.mem, lhm as u16);
                let rhm = crate::helpers::norm_min(st.eqtb.int_par(IntPar::RightHyphenMin));
                w.set_what_rhm(&mut st.mem, rhm as u16);
            }
        }

        ExtensionCode::PdfSavePos => {
            new_whatsit(st, WhatsitSubtype::PdfSavePos, SMALL_NODE_SIZE)?;
        }

        ExtensionCode::ResetTimer => {
            if !st.config.deterministic_mode {
                (st.shipout.epochseconds, st.shipout.microseconds) =
                    crate::texmfmp::get_seconds_and_micros();
            }
        }

        ExtensionCode::SetRandomSeed => {
            scan_int(st)?;
            if st.scanner.cur_val < 0 {
                st.scanner.cur_val = st.scanner.cur_val.wrapping_neg();
            }
            st.errors.random_seed = st.scanner.cur_val;
            let random_seed = st.errors.random_seed;
            st.errors.init_randoms(random_seed);
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
                let w = new_whatsit(st, WhatsitSubtype::Glyph, GLYPH_NODE_SIZE)?;
                scan_int(st)?;
                if st.scanner.cur_val < 0 || st.scanner.cur_val > 65535 {
                    error_here_with_diagnostic(st, "Bad glyph number");
                    crate::output::append_to_diagnostic(st, &format!(" ({})", st.scanner.cur_val));
                    capture_to_diagnostic(st, None);
                    st.errors.help(&[
                        "A glyph number must be between 0 and 65535.",
                        "I changed this one to zero.",
                    ]);
                    crate::errors::int_error(st, st.scanner.cur_val)?;
                    st.scanner.cur_val = 0;
                }
                w.set_native_font(&mut st.mem, cur_font);
                w.set_native_glyph(&mut st.mem, st.scanner.cur_val as u16);
                let use_glyph_metrics = st.eqtb.int_par(IntPar::XetexUseGlyphMetrics) > 0;
                st.fonts
                    .measure_native_glyph(&mut st.mem, w, use_glyph_metrics)?;
            }
        }

        ExtensionCode::InputEncoding => {
            crate::io::scan_and_pack_name(st)?;
            let name = st.input.name_of_file_bytes().to_vec();
            let (enc_mode, info) = get_encoding_mode_and_info(st, &name)?;
            if enc_mode == AUTO {
                error_here_with_diagnostic(
                    st,
                    "Encoding mode `auto' is not valid for \\XeTeXinputencoding",
                );
                capture_to_diagnostic(st, None);
                st.errors.help(&[
                    r"You can't use `auto' encoding here, only for \XeTeXdefaultencoding.",
                    "I'll ignore this and leave the current encoding unchanged.",
                ]);
                crate::errors::error(st)?;
            } else {
                // `\scantokens` occupies an input level without a UFile.
                let slot = crate::io::FileSlot::Input(st.input.in_open as usize);
                crate::io::with_slot_file(st, slot, |st, f| {
                    crate::io::set_input_file_encoding(st, f, enc_mode, info)
                })
                .transpose()?;
            }
        }

        ExtensionCode::DefaultEncoding => {
            crate::io::scan_and_pack_name(st)?;
            let name = st.input.name_of_file_bytes().to_vec();
            let (enc_mode, info) = get_encoding_mode_and_info(st, &name)?;
            st.eqtb.set_int_par(IntPar::XetexDefaultInputMode, enc_mode);
            st.eqtb.set_int_par(IntPar::XetexDefaultInputEncoding, info);
        }

        ExtensionCode::LinebreakLocale => {
            crate::io::scan_file_name(st)?;
            let locale = if st.strings.length(st.scanner.cur_name) == 0 {
                StrNumber::NONE
            } else {
                st.scanner.cur_name
            };
            st.eqtb
                .set_int_par(IntPar::XetexLinebreakLocale, locale.raw());
        }
    }
    Ok(())
}
