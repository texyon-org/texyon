//! Shipping whatsits: `\special`, `\write`, the `\openout` streams and pictures.

use super::pic::PageboxType;
use super::{dvi_special_from_cur_string, synch_h, synch_v};
use crate::base::mode::Mode;
use crate::base::params::IntPar;
use crate::base::token::{TokCursor, Token, TokenList};
use crate::base::units::StrNumber;
use crate::env::{EngineEnv, WriteStream};
use crate::files::io::{FileName, may_write};
use crate::memory::node::{NodePtr, OpenNode, PicNode, WhatsitNode, WhatsitVariant, WriteNode};
use crate::print::diagnostic::{DiagnosticKind, begin_capture, end_capture};
use crate::print::errors::{Abort, FilePrompt, error, prompt_file_name};
use crate::print::{
    Selector, print, print_char, print_int, print_ln, print_nl, print_raw_char, print_scaled,
    show_token_list, token_show,
};
use crate::scanning::input::{TokenType, begin_token_list, end_token_list};
use crate::scanning::scan::{get_token, scan_toks};
use crate::state::EngineState;
use crate::tables::cs::Frozen;
use crate::tables::eqtb::EqtbState;

fn special_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    special: NodePtr<WriteNode>,
    late: bool,
) -> Result<(), Abort> {
    synch_h(st, env)?;
    synch_v(st, env)?;
    st.shipout.doing_special = true;
    let old_setting = st.shipout.selector;
    let list = if late {
        expand_write_tokens(st, env, st.mem[special].tokens)?;
        st.scanner.def_ref
    } else {
        st.mem[special].tokens
    };
    st.shipout.selector = Selector::NewString;
    let tokens = st.mem.tokens.cursor(list);
    let avail = st.strings.pool_size - st.strings.pool_ptr;
    show_token_list(st, env, tokens, TokCursor::END, avail);
    st.shipout.selector = old_setting;
    if st.strings.pool_ptr + 1 > st.strings.pool_size {
        return Err(st.strings.pool_overflow());
    }
    dvi_special_from_cur_string(st, env)?;
    if late {
        st.mem.tokens.flush(list);
    }
    st.shipout.doing_special = false;
    Ok(())
}

impl EqtbState {
    fn log_selector(&self) -> Selector {
        if self.int_par(IntPar::TracingOnline) <= 0 {
            Selector::LogOnly
        } else {
            Selector::TermAndLog
        }
    }
}

fn expand_write_tokens(
    st: &mut EngineState,
    env: &mut EngineEnv,
    tokens: TokenList,
) -> Result<(), Abort> {
    let close = st.mem.tokens.single(Token::right_brace(b'}'))?;
    st.mem
        .tokens
        .append(close, Token::from_cs(Frozen::EndWrite.cs()))?;
    begin_token_list(st, env, close, TokenType::Inserted)?;
    begin_token_list(st, env, tokens, TokenType::Write)?;
    let open = st.mem.tokens.single(Token::left_brace(b'{'))?;
    begin_token_list(st, env, open, TokenType::Inserted)?;

    let old_mode = st.input.cur_list.mode;
    st.input.cur_list.mode = Mode::None;
    st.scanner.cur_cs = st.shipout.write_loc;
    scan_toks(st, env, false, true)?;
    st.input.cur_list.mode = old_mode;
    get_token(st, env)?;
    if st.scanner.cur_tok != Token::from_cs(Frozen::EndWrite.cs()) {
        error(
            st,
            env,
            "Unbalanced write command",
            &[
                r"On this page there's a \write with fewer real {'s than }'s.",
                "I can't handle that very well; good luck.",
            ],
        )?;
        loop {
            get_token(st, env)?;
            if st.scanner.cur_tok == Token::from_cs(Frozen::EndWrite.cs()) {
                break;
            }
        }
    }
    end_token_list(st)
}

fn write_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    write: NodePtr<WriteNode>,
) -> Result<(), Abort> {
    expand_write_tokens(st, env, st.mem[write].tokens)?;

    let old_setting = st.shipout.selector;
    let j = st.mem[write].stream;
    let to_transcript = if j == 18 {
        st.shipout.selector = Selector::NewString;
        false
    } else if st.shipout.write_open[j as usize] {
        ensure_write_stream_open(st, env, j as usize)?;
        st.shipout.selector = Selector::File(j as u8);
        false
    } else {
        if j == 17 && st.shipout.selector == Selector::TermAndLog {
            st.shipout.selector = Selector::LogOnly;
        }
        print_nl(st, env, "");
        true
    };
    if to_transcript {
        begin_capture(st, env, DiagnosticKind::Written);
    }
    let def_ref = st.scanner.def_ref;
    token_show(st, env, def_ref);
    if to_transcript {
        end_capture(st, env);
    }
    print_ln(st, env);
    st.mem.tokens.flush(def_ref);

    if j == 18 {
        st.shipout.selector = st.eqtb.log_selector();
        if !st.errors.log_opened {
            st.shipout.selector = Selector::TermOnly;
        }
        // Tectonic leaves shell-escape warnings to the driver when it is enabled.
        if !st.config.shell_escape_enabled {
            print_nl(st, env, "runsystem(");
            let base = st.strings.cur_string_start();
            for k in base.until(st.strings.pool_ptr) {
                print(
                    st,
                    env,
                    StrNumber::single_char(i32::from(st.strings.str_pool[k.index()])),
                );
            }
            print(st, env, ")...");
            print(st, env, "disabled");
            print_char(st, env, b'.');
            print_nl(st, env, "");
            print_ln(st, env);
        } else {
            env.shell_escape(st.strings.cur_string());
        }
        st.strings.discard_cur_string();
    }
    st.shipout.selector = old_setting;
    Ok(())
}

pub(super) fn pic_out(
    st: &mut EngineState,
    env: &mut EngineEnv,
    pic: NodePtr<PicNode>,
) -> Result<(), Abort> {
    synch_h(st, env)?;
    synch_v(st, env)?;
    let node = st.mem[pic];
    let old_setting = st.shipout.selector;
    st.shipout.selector = Selector::NewString;
    print(st, env, "pdf:image ");
    print(st, env, "matrix ");
    for v in node.transform {
        print_scaled(st, env, v);
        print(st, env, b' ');
    }
    print(st, env, "page ");
    print_int(st, env, i32::from(node.page));
    print(st, env, b' ');
    if let Some(label) = PageboxType::from_u16(node.pagebox).and_then(|pb| pb.label()) {
        print(st, env, label);
    }
    print(st, env, b'(');
    let path = st.mem.entries(pic).to_vec();
    for byte in path {
        print_raw_char(st, env, i32::from(byte), true);
    }
    print(st, env, b')');
    st.shipout.selector = old_setting;
    dvi_special_from_cur_string(st, env)
}

pub(crate) fn out_what(
    st: &mut EngineState,
    env: &mut EngineEnv,
    w: NodePtr<WhatsitNode>,
) -> Result<(), Abort> {
    let leaders = st.shipout.doing_leaders;
    match w.variant(&st.mem) {
        Some(WhatsitVariant::Write(write)) if !leaders => write_out(st, env, write)?,
        Some(WhatsitVariant::Open(open)) if !leaders => {
            out_write_stream(st, env, st.mem[open].stream, Some(open))?
        }
        Some(WhatsitVariant::Close(close)) if !leaders => {
            out_write_stream(st, env, st.mem[close].stream, None)?
        }
        Some(
            WhatsitVariant::Write(_)
            | WhatsitVariant::Open(_)
            | WhatsitVariant::Close(_)
            | WhatsitVariant::Language(_),
        ) => {}
        Some(WhatsitVariant::Special(special)) => special_out(st, env, special, false)?,
        Some(WhatsitVariant::LateSpecial(special)) => special_out(st, env, special, true)?,
        _ => return Err(Abort::Confusion("ext4")),
    }
    Ok(())
}

fn out_write_stream(
    st: &mut EngineState,
    env: &mut EngineEnv,
    j: i32,
    open: Option<NodePtr<OpenNode>>,
) -> Result<(), Abort> {
    let stream = j as usize;
    if st.shipout.write_open[stream] {
        ensure_write_stream_open(st, env, stream)?;
        if let Some(file) = WriteStream::new(stream) {
            env.close_write(file);
        }
    }
    let Some(open) = open else {
        st.shipout.write_open[stream] = false;
        return Ok(());
    };
    let Some(write_stream) = WriteStream::new(stream) else {
        return Ok(());
    };

    let open = st.mem[open];
    let ext = if st.strings.length(open.ext) == 0 {
        st.strings.make_tex_string(".tex")?
    } else {
        open.ext
    };
    let file = FileName {
        name: open.name,
        area: open.area,
        ext,
    };
    let path = file.path(&st.strings);
    if !(may_write(env, &path) && env.open_write(write_stream, &path)) {
        return Err(prompt_file_name(st, env, FilePrompt::Output, file, &path));
    }
    st.shipout.write_open[stream] = true;
    st.shipout.write_name[stream] = Some(path);
    st.shipout.write_prefix[stream].clear();
    if st.errors.log_opened {
        let old_setting = st.shipout.selector;
        st.shipout.selector = st.eqtb.log_selector();
        print_nl(st, env, "\\openout");
        print_int(st, env, j);
        print(st, env, (" = `", file, "'."));
        print_nl(st, env, "");
        print_ln(st, env);
        st.shipout.selector = old_setting;
    }
    Ok(())
}

/// Lazily recreate a checkpoint-restored TeX output stream and its prefix.
pub(crate) fn ensure_write_stream_open(
    st: &mut EngineState,
    env: &mut EngineEnv,
    stream: usize,
) -> Result<(), Abort> {
    let Some(write_stream) = WriteStream::new(stream) else {
        return Ok(());
    };
    if env.write_is_open(write_stream) {
        return Ok(());
    }
    let Some(name) = &st.shipout.write_name[stream] else {
        return Err(Abort::CannotOpenOutput {
            name: Box::default(),
        });
    };
    if !env.open_write(write_stream, name) {
        return Err(Abort::CannotOpenOutput {
            name: name.as_slice().into(),
        });
    }
    let prefix: Vec<u8> = st.shipout.write_prefix[stream]
        .iter()
        .map(|&code_unit| code_unit as u8)
        .collect();
    env.write_put(write_stream, &prefix);
    Ok(())
}
