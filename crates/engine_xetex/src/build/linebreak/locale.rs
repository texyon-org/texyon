//! ICU line breaking for `\XeTeXlinebreaklocale`.

use crate::base::params::{GluePar, IntPar};
use crate::base::units::{FontId, StrNumber};
use crate::ffi::teckit_icu::{IcuBreaks, LinebreakHandle};
use crate::memory::mem::ZERO_GLUE;
use crate::print::errors::Abort;
use crate::print::{begin_diagnostic, end_diagnostic, print, print_int, print_nl};
use crate::state::EngineState;

/// Append a native word of font `f` holding `text` to the current list.
fn append_native_word(st: &mut EngineState, f: FontId, text: &[u16]) -> Result<(), Abort> {
    let w = st
        .mem
        .new_native_word_node(&st.eqtb, f, text.len() as i32)?;
    st.input.cur_list.tail_append(&mut st.mem, w);
    if !text.is_empty() {
        st.mem.entries_mut(w).copy_from_slice(text);
    }
    st.mem.set_native_metrics(&st.eqtb, &st.fonts, w)?;
    Ok(())
}

/// Where a run of native text may break.
enum TextBreaks<'t> {
    Icu(IcuBreaks<'t>),
    Graphite(texyon_xetex_layout::GraphiteBreaks<'t>),
    /// An ICU iterator left over from earlier text, which it already ran through.
    Stale(&'t LinebreakHandle),
}

impl TextBreaks<'_> {
    pub(super) fn next_break(&mut self) -> i32 {
        match self {
            TextBreaks::Icu(breaks) => breaks.next_break(),
            TextBreaks::Graphite(breaks) => breaks.next_break(),
            TextBreaks::Stale(iter) => iter.next_after_end(),
        }
    }
}

/// Makes sure `fonts.brk_iter` is an ICU line breaker for `locale`.
fn open_linebreak_iterator(
    st: &mut EngineState,
    locale: StrNumber,
    name: &[u8],
) -> Result<LinebreakHandle, Abort> {
    if locale != st.fonts.brk_locale_str_num {
        st.fonts.brk_iter = None;
    }
    if st.fonts.brk_iter.is_none() {
        let (mut iter, status) = LinebreakHandle::open(name);
        if status > 0 {
            begin_diagnostic(st);
            print_nl(st, b'E');
            print(st, "rror ");
            print_int(st, status);
            print(st, " creating linebreak iterator for locale `");
            print(st, name);
            print(st, "'; trying default locale `en_us'.");
            end_diagnostic(st, true);
            drop(iter);
            iter = LinebreakHandle::open(b"en_us").0;
        }
        st.fonts.brk_locale_str_num = locale;
        st.fonts.brk_iter = iter;
    }
    st.fonts
        .brk_iter
        .take()
        .ok_or(Abort::Internal("failed to create linebreak iterator"))
}

/// Append `native_text[s..s + len]` as native words, split at the locale's
/// line-break opportunities when `\XeTeXlinebreaklocale` is set.
pub(crate) fn do_locale_linebreaks(
    st: &mut EngineState,
    main_f: FontId,
    s: i32,
    len: i32,
) -> Result<(), Abort> {
    let text = st.scanner.native_text[s as usize..(s + len) as usize].to_vec();

    let locale = StrNumber::from_raw(st.eqtb.int_par(IntPar::XetexLinebreakLocale));
    if locale == StrNumber::NONE || len == 1 {
        return append_native_word(st, main_f, &text);
    }

    let use_skip = st.eqtb.glue_par(GluePar::XetexLinebreakSkip) != ZERO_GLUE;
    let penalty = st.eqtb.int_par(IntPar::XetexLinebreakPenalty);
    let use_penalty = penalty != 0 || !use_skip;
    let locale_name = st.strings.texstring_bytes(locale);

    let engine = st.fonts.shared_layout_engine(main_f);
    let graphite = engine
        .as_deref()
        .filter(|_| locale_name == b"G")
        .and_then(|engine| engine.graphite_breaks(&text));
    let mut iter = None;
    let mut breaks = match graphite {
        Some(graphite) => match st.fonts.brk_iter.take() {
            Some(stale) => TextBreaks::Stale(iter.insert(stale)),
            None => TextBreaks::Graphite(graphite),
        },
        None => {
            let fresh = open_linebreak_iterator(st, locale, &locale_name)?;
            TextBreaks::Icu(iter.insert(fresh).breaks(&text))
        }
    };

    let result = (|| {
        let mut offs = 0;
        loop {
            let prev_offs = offs;
            offs = breaks.next_break();
            if offs > 0 {
                if prev_offs != 0 {
                    if use_penalty {
                        let node = st.mem.new_penalty(penalty)?;
                        st.input.cur_list.tail_append(&mut st.mem, node);
                    }
                    if use_skip {
                        let node = st
                            .mem
                            .new_param_glue(&st.eqtb, GluePar::XetexLinebreakSkip)?;
                        st.input.cur_list.tail_append(&mut st.mem, node);
                    }
                }
                append_native_word(st, main_f, &text[prev_offs as usize..offs as usize])?;
            }
            if offs < 0 {
                return Ok(());
            }
        }
    })();
    if iter.is_some() {
        st.fonts.brk_iter = iter;
    }
    result
}
