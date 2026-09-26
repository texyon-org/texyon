//! ICU line breaking for `\XeTeXlinebreaklocale`.

use crate::errors::Abort;
use crate::mem::ZERO_GLUE;
use crate::params::{GluePar, IntPar};
use crate::state::EngineState;
use crate::units::{FontId, StrNumber};

/// Append a native word of font `f` holding `text` to the current list.
fn append_native_word(st: &mut EngineState, f: FontId, text: &[u16]) -> Result<(), Abort> {
    let w = st
        .mem
        .new_native_word_node(&st.eqtb, f, text.len() as i32)?;
    st.input.cur_list.tail_append(&mut st.mem, w.ptr());
    if !text.is_empty() {
        w.native_text_mut(&mut st.mem).copy_from_slice(text);
    }
    st.mem.set_native_metrics(&st.eqtb, &st.fonts, w)?;
    Ok(())
}

/// Where a run of native text may break.
enum TextBreaks<'t> {
    Icu(crate::layout::IcuBreaks<'t>),
    Graphite(tectonic_xetex_layout::GraphiteBreaks<'t>),
    /// An ICU iterator left over from earlier text, which it already ran through.
    Stale(&'t crate::layout::LinebreakHandle),
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
) -> Result<(), Abort> {
    if locale != st.fonts.brk_locale_str_num {
        st.fonts.brk_iter = None;
    }
    if st.fonts.brk_iter.is_none() {
        let (mut iter, status) = crate::layout::LinebreakHandle::open(name);
        if status > 0 {
            crate::output::begin_diagnostic(st);
            crate::output::print_nl(st, b'E');
            crate::output::print(st, "rror ");
            crate::output::print_int(st, status);
            crate::output::print(st, " creating linebreak iterator for locale `");
            crate::output::print(st, name);
            crate::output::print(st, "'; trying default locale `en_us'.");
            crate::output::end_diagnostic(st, true);
            drop(iter);
            iter = crate::layout::LinebreakHandle::open(b"en_us").0;
        }
        st.fonts.brk_locale_str_num = locale;
        st.fonts.brk_iter = iter;
    }
    if st.fonts.brk_iter.is_none() {
        return Err(Abort::Internal("failed to create linebreak iterator"));
    }
    Ok(())
}

/// Append `native_text[s..s + len]` as native words, split at the locale's
/// line-break opportunities when `\XeTeXlinebreaklocale` is set.
pub(crate) fn do_locale_linebreaks(st: &mut EngineState, s: i32, len: i32) -> Result<(), Abort> {
    let main_f = st.mainloop.main_f;
    let text = st.scanner.native_text[s as usize..(s + len) as usize].to_vec();

    let locale = StrNumber(st.eqtb.int_par(IntPar::XetexLinebreakLocale));
    if locale == StrNumber::NONE || len == 1 {
        return append_native_word(st, main_f, &text);
    }

    let use_skip = st.eqtb.glue_par(GluePar::XetexLinebreakSkip) != ZERO_GLUE;
    let penalty = st.eqtb.int_par(IntPar::XetexLinebreakPenalty);
    let use_penalty = penalty != 0 || !use_skip;
    let locale_name = st.strings.texstring_bytes(locale);

    let engine = st.fonts.layout_engine_arc(main_f);
    let graphite = engine
        .as_deref()
        .filter(|_| locale_name == b"G")
        .and_then(|engine| engine.graphite_breaks(&text));
    let mut iter = None;
    let mut breaks = match graphite {
        Some(graphite) => match &st.fonts.brk_iter {
            Some(_) => {
                iter = st.fonts.brk_iter.take();
                TextBreaks::Stale(iter.as_ref().unwrap())
            }
            None => TextBreaks::Graphite(graphite),
        },
        None => {
            open_linebreak_iterator(st, locale, &locale_name)?;
            iter = st.fonts.brk_iter.take();
            TextBreaks::Icu(iter.as_mut().unwrap().breaks(&text))
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
                        st.input.cur_list.tail_append(&mut st.mem, node.ptr());
                    }
                    if use_skip {
                        let node = st
                            .mem
                            .new_param_glue(&st.eqtb, GluePar::XetexLinebreakSkip)?;
                        st.input.cur_list.tail_append(&mut st.mem, node.ptr());
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
