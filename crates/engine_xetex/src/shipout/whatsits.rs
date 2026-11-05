//! Shipping whatsits: `\special`, `\write`, the `\openout` streams and pictures.

use crate::bridge::{output_close, output_open, shell_escape};
use crate::consts::{END_WRITE, INVALID_HANDLE, TEX_NULL};
use crate::errors::Abort;
use crate::input::TokenType;
use crate::node::{NodePtr, WhatsitNode, WhatsitSubtype};
use crate::params::IntPar;
use crate::state::{EqtbState, Selector};
use crate::token::Token;

use super::{dvi_special_from_cur_string, synch_h, synch_v};

fn special_out(
    st: &mut crate::state::EngineState,
    special: NodePtr<WhatsitNode>,
) -> Result<(), Abort> {
    synch_h(st)?;
    synch_v(st)?;
    st.shipout.doing_special = true;
    let old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    let tokens = special.write_tokens(&st.mem).link(&st.mem);
    let avail = st.strings.pool_size - st.strings.pool_ptr;
    crate::output::show_token_list(st, tokens, TEX_NULL, avail);
    st.shipout.selector = old_setting;
    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(Abort::PoolOverflow);
    }
    dvi_special_from_cur_string(st)?;
    st.shipout.doing_special = false;
    Ok(())
}

impl EqtbState {
    /// Log only, or terminal and log when `\tracingonline` is positive.
    fn log_selector(&self) -> Selector {
        if self.int_par(IntPar::TracingOnline) <= 0 {
            Selector::LogOnly
        } else {
            Selector::TermAndLog
        }
    }
}

fn write_out(st: &mut crate::state::EngineState, write: NodePtr<WhatsitNode>) -> Result<(), Abort> {
    let q = st.mem.get_avail_token()?;
    q.set_token(&mut st.mem, Token::right_brace(b'}'));
    let r = st.mem.get_avail_token()?;
    q.set_link(&mut st.mem, r);
    r.set_token(&mut st.mem, Token::from_cs(END_WRITE));
    crate::input::begin_token_list(st, q, TokenType::Inserted)?;
    crate::input::begin_token_list(st, write.write_tokens(&st.mem), TokenType::Write)?;
    let q = st.mem.get_avail_token()?;
    q.set_token(&mut st.mem, Token::left_brace(b'{'));
    crate::input::begin_token_list(st, q, TokenType::Inserted)?;

    let old_mode = st.input.cur_list.mode;
    st.input.cur_list.mode = 0;
    st.scanner.cur_cs = st.shipout.write_loc;
    crate::scan::scan_toks(st, false, true)?;
    crate::scan::get_token(st)?;
    if st.scanner.cur_tok != Token::from_cs(END_WRITE) {
        crate::output::error_here_with_diagnostic(st, "Unbalanced write command");
        crate::output::capture_to_diagnostic(st, None);
        st.errors.help(&[
            r"On this page there's a \write with fewer real {'s than }'s.",
            "I can't handle that very well; good luck.",
        ]);
        crate::errors::error(st)?;
        loop {
            crate::scan::get_token(st)?;
            if st.scanner.cur_tok == Token::from_cs(END_WRITE) {
                break;
            }
        }
    }
    st.input.cur_list.mode = old_mode;
    crate::input::end_token_list(st)?;

    let old_setting = st.shipout.selector;
    let j = write.write_stream(&st.mem);
    if j == 18 {
        st.shipout.selector = Selector::NewString;
    } else if st.shipout.write_open[j as usize] {
        ensure_write_stream_open(st, j as usize)?;
        st.shipout.selector = Selector::File(j as u8);
    } else {
        if j == 17 && st.shipout.selector == Selector::TermAndLog {
            st.shipout.selector = Selector::LogOnly;
        }
        crate::output::print_nl(st, "");
    }
    let def_ref = st.scanner.def_ref;
    crate::output::token_show(st, def_ref);
    crate::output::print_ln(st);
    st.mem.flush_token_list(def_ref);

    if j == 18 {
        st.shipout.selector = st.eqtb.log_selector();
        if !st.errors.log_opened {
            st.shipout.selector = Selector::TermOnly;
        }
        // Tectonic leaves shell-escape warnings to the driver when it is enabled.
        if !st.config.shell_escape_enabled {
            crate::output::diagnostic_begin_capture_warning_here(st);
            crate::output::print_nl(st, "runsystem(");
            let base = st.strings.cur_string_start();
            for k in base..st.strings.pool_ptr {
                crate::output::print(st, i32::from(st.strings.str_pool[k as usize]));
            }
            crate::output::print(st, ")...");
            crate::output::print(st, "disabled");
            crate::output::print_char(st, b'.');
            crate::output::capture_to_diagnostic(st, None);
            crate::output::print_nl(st, "");
            crate::output::print_ln(st);
        } else {
            shell_escape(st.strings.cur_string());
        }
        st.strings.discard_cur_string();
    }
    st.shipout.selector = old_setting;
    Ok(())
}

pub(super) fn pic_out(
    st: &mut crate::state::EngineState,
    pic: NodePtr<WhatsitNode>,
) -> Result<(), Abort> {
    synch_h(st)?;
    synch_v(st)?;
    let old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    crate::output::print(st, "pdf:image ");
    crate::output::print(st, "matrix ");
    for v in pic.pic_transform(&st.mem) {
        crate::output::print_scaled(st, v);
        crate::output::print(st, b' ');
    }
    crate::output::print(st, "page ");
    crate::output::print_int(st, i32::from(pic.pic_page(&st.mem)));
    crate::output::print(st, b' ');
    if let Some(label) =
        crate::pic::PageboxType::from_raw(pic.pic_pagebox(&st.mem)).and_then(|pb| pb.label())
    {
        crate::output::print(st, label);
    }
    crate::output::print(st, b'(');
    let path = pic.pic_path(&st.mem).to_vec();
    for byte in path {
        crate::output::print_raw_char(st, i32::from(byte), true);
    }
    crate::output::print(st, b')');
    st.shipout.selector = old_setting;
    dvi_special_from_cur_string(st)
}

/// `out_what` for the node at the tail of the current list (`\immediate`).
pub(crate) fn out_what(
    st: &mut crate::state::EngineState,
    w: NodePtr<WhatsitNode>,
) -> Result<(), Abort> {
    let subtype = w.whatsit_subtype(&st.mem);
    match subtype {
        Some(WhatsitSubtype::Open | WhatsitSubtype::Write | WhatsitSubtype::Close) => {
            if !st.shipout.doing_leaders {
                out_write_stream(st, w, subtype)?;
            }
        }
        Some(WhatsitSubtype::Special) => special_out(st, w)?,
        Some(WhatsitSubtype::Language) => {}
        _ => return Err(Abort::Confusion("ext4")),
    }
    Ok(())
}

fn out_write_stream(
    st: &mut crate::state::EngineState,
    w: NodePtr<WhatsitNode>,
    subtype: Option<WhatsitSubtype>,
) -> Result<(), Abort> {
    if subtype == Some(WhatsitSubtype::Write) {
        return write_out(st, w);
    }
    let j = w.write_stream(&st.mem);
    let stream = j as usize;
    if st.shipout.write_open[stream] {
        ensure_write_stream_open(st, stream)?;
        output_close(st.shipout.write_file[stream]);
    }
    if subtype == Some(WhatsitSubtype::Close) {
        st.shipout.write_open[stream] = false;
        if j < 16 {
            st.shipout.write_file[stream] = INVALID_HANDLE;
        }
        return Ok(());
    }
    if j >= 16 {
        return Ok(());
    }

    st.scanner.cur_name = w.open_name(&st.mem);
    st.scanner.cur_area = w.open_area(&st.mem);
    st.scanner.cur_ext = w.open_ext(&st.mem);
    if st.strings.length(st.scanner.cur_ext) == 0 {
        st.scanner.cur_ext = st.strings.make_tex_string(".tex")?;
    }
    let (n, a, e) = (st.scanner.cur_name, st.scanner.cur_area, st.scanner.cur_ext);
    crate::io::pack_file_name(st, n, a, e);
    let handle = output_open(st.input.name_of_file_bytes(), false);
    if handle == INVALID_HANDLE {
        return Err(Abort::CannotOpenOutput);
    }
    st.shipout.write_file[stream] = handle;
    st.shipout.write_open[stream] = true;
    st.shipout.write_name[stream] = Some(st.input.name_of_file.clone());
    st.shipout.write_prefix[stream].clear();
    if st.errors.log_opened {
        let old_setting = st.shipout.selector;
        st.shipout.selector = st.eqtb.log_selector();
        crate::output::print_nl(st, "\\openout");
        crate::output::print_int(st, j);
        crate::output::print(st, " = `");
        crate::output::print_file_name(st, n, a, e);
        crate::output::print(st, "'.");
        crate::output::print_nl(st, "");
        crate::output::print_ln(st);
        st.shipout.selector = old_setting;
    }
    Ok(())
}

/// Lazily recreate a checkpoint-restored TeX output stream and its prefix.
pub(crate) fn ensure_write_stream_open(
    st: &mut crate::state::EngineState,
    stream: usize,
) -> Result<(), Abort> {
    if st.shipout.write_file[stream] != INVALID_HANDLE {
        return Ok(());
    }
    let Some(name) = &st.shipout.write_name[stream] else {
        return Err(Abort::CannotOpenOutput);
    };
    let handle = output_open(name, false);
    if handle == INVALID_HANDLE {
        return Err(Abort::CannotOpenOutput);
    }
    for &code_unit in &st.shipout.write_prefix[stream] {
        crate::bridge::output_putc(handle, code_unit as u8);
    }
    st.shipout.write_file[stream] = handle;
    Ok(())
}
