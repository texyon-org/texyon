//! Pre-hyphenation: finding the words `hyphenate` should try.

use crate::consts::TEX_NULL;
use crate::errors::Abort;
use crate::fonts::LigChar;
use crate::node::{
    AnyNode, CharNode, KernSubtype, LanguageNode, MathCode, NativeWordNode, NodePtr, WhatsitVariant,
};
use crate::params::IntPar;
use crate::state::{EngineState, MemState};
use crate::units::MemPtr;

use super::hyphenate::{hyphenate, split_native_word};

/// A word found for hyphenation, its letters in `hu`/`hc[1..=hn]`.
pub(super) struct Word {
    /// The glue node before the word, where the search for its predecessor starts.
    pub start: MemPtr,
    /// The node before the word's first letter.
    pub ha: MemPtr,
    /// The word's last node.
    pub hb: MemPtr,
    pub hn: u16,
}

/// Switch hyphenation to the language of a `\language` whatsit.
pub(super) fn set_language(st: &mut EngineState, w: NodePtr<LanguageNode>) {
    let language = st.mem[w];
    st.hyph.cur_lang = language.lang as u8;
    st.hyph.l_hyf = i32::from(language.lhm);
    st.hyph.r_hyf = i32::from(language.rhm);
    st.hyph.set_hyph_index();
}

impl MemState {
    /// Scan from `s` to the end of the word, returning the node that ends it, or
    /// `None` if a node there forbids hyphenation. `math_ok` lets a math node end it.
    fn find_word_end(&self, mut s: MemPtr, math_ok: bool) -> Option<MemPtr> {
        loop {
            if !self.is_char_node(s) {
                match self.classify(s) {
                    Ok(AnyNode::Ligature(_)) => {}
                    Ok(AnyNode::Kern(k)) => {
                        if self[k].subtype != KernSubtype::Normal {
                            return Some(s);
                        }
                    }
                    Ok(
                        AnyNode::Whatsit(_)
                        | AnyNode::Glue(_)
                        | AnyNode::Penalty(_)
                        | AnyNode::Ins(_)
                        | AnyNode::Adjust(_)
                        | AnyNode::Mark(_),
                    ) => return Some(s),
                    Ok(AnyNode::Math(m)) if math_ok => {
                        return self[m]
                            .subtype
                            .get()
                            .is_some_and(MathCode::is_lr)
                            .then_some(s);
                    }
                    _ => return None,
                }
            }
            s = s.link(self);
        }
    }
}

/// Try to hyphenate the word following the glue node `cur`.
pub(super) fn try_hyphenate(st: &mut EngineState, cur: MemPtr) -> Result<(), Abort> {
    let max_len = st.eqtb.max_hyphenatable_length();
    let uc_hyph = st.eqtb.int_par(IntPar::UcHyph) > 0;
    let mut prev_s = cur;
    let mut s = prev_s.link(&st.mem);
    if s.is_null() {
        return Ok(());
    }

    // Skip to the first letter, setting `hf`.
    loop {
        let c;
        if let Some(ch) = st.mem.as_char(s) {
            c = i32::from(st.mem[ch].character);
            st.hyph.hf = st.mem[ch].font;
        } else {
            match st.mem.classify(s) {
                Ok(AnyNode::Ligature(lig)) if !st.mem[lig].lig_ptr.is_null() => {
                    let first = NodePtr::<CharNode>::at(st.mem[lig].lig_ptr);
                    c = i32::from(st.mem[first].character);
                    st.hyph.hf = st.mem[first].font;
                }
                Ok(AnyNode::Ligature(_)) => {
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                Ok(AnyNode::Kern(k)) if st.mem[k].subtype == KernSubtype::Normal => {
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                Ok(AnyNode::Math(m)) if st.mem[m].subtype.get().is_some_and(MathCode::is_lr) => {
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                Ok(AnyNode::Whatsit(w)) => {
                    let variant = w.variant(&st.mem);
                    if let Some(WhatsitVariant::NativeWord(word)) = variant {
                        let len = usize::from(st.mem[word].length);
                        let mut l = 0;
                        while l < len {
                            let c = st.mem.get_native_usv(word, l);
                            if st.eqtb.lc_code(c) != 0 {
                                st.hyph.hf = st.mem[word].font;
                                prev_s = s;
                                if st.eqtb.lc_code(c) == c || uc_hyph {
                                    break;
                                }
                                return Ok(());
                            }
                            if c >= 65_536 {
                                l += 1;
                            }
                            l += 1;
                        }
                        if l < len {
                            break;
                        }
                    }

                    if let Some(WhatsitVariant::Language(language)) = variant {
                        set_language(st, language);
                    }
                    prev_s = s;
                    s = prev_s.link(&st.mem);
                    continue;
                }
                _ => return Ok(()),
            }
        }

        st.hyph.hc[0] = st.hyph.hyph_code(&st.eqtb, c);
        if st.hyph.hc[0] != 0 {
            if st.hyph.hc[0] == c || uc_hyph {
                break;
            }
            return Ok(());
        }

        prev_s = s;
        s = prev_s.link(&st.mem);
    }

    st.hyph.hyf_char = LigChar::from_raw(st.fonts.hyphen_char[st.hyph.hf]);
    if st.hyph.hyf_char.raw() < 0 || !st.hyph.hyf_char.is_char() {
        return Ok(());
    }
    let mut word = Word {
        start: cur,
        ha: prev_s,
        hb: MemPtr::NULL,
        hn: 0,
    };
    if st.hyph.l_hyf + st.hyph.r_hyf > max_len {
        return Ok(());
    }

    let native_ha = match st.mem.as_native_word(word.ha) {
        Some(ha) => {
            let Some(end) = st.mem.find_word_end(st.mem[ha].link, false) else {
                return Ok(());
            };
            s = end;
            Some(collect_native_word(st, &mut word, ha, max_len)?)
        }
        None => {
            s = collect_word(st, &mut word, s, max_len);
            None
        }
    };

    if i32::from(word.hn) < st.hyph.l_hyf + st.hyph.r_hyf || st.mem.find_word_end(s, true).is_none()
    {
        return Ok(());
    }

    hyphenate(st, &word, native_ha)
}

/// Collect the letters of the native word `ha` into `hu`/`hc`, splitting it
/// where the hyphenatable part begins or ends. Returns the word's final node.
fn collect_native_word(
    st: &mut EngineState,
    word: &mut Word,
    mut ha: NodePtr<NativeWordNode>,
    max_len: i32,
) -> Result<NodePtr<NativeWordNode>, Abort> {
    word.hn = 0;
    'restart: loop {
        let len = usize::from(st.mem[ha].length);
        let mut l = 0;
        while l < len {
            let c = st.mem.get_native_usv(ha, l);
            let hc0 = st.hyph.hyph_code(&st.eqtb, c);
            st.hyph.hc[0] = hc0;

            if hc0 == 0 {
                if word.hn > 0 {
                    split_native_word(st, ha, l)?;
                    return Ok(ha);
                }
            } else if word.hn == 0 && l > 0 {
                ha = split_native_word(st, ha, l)?;
                word.ha = ha.ptr();
                continue 'restart;
            } else if i32::from(word.hn) == max_len {
                return Ok(ha);
            } else {
                word.hn += 1;
                let hn = usize::from(word.hn);
                if c < 65_536 {
                    st.hyph.hu[hn] = LigChar::from_raw(c);
                    st.hyph.hc[hn] = hc0;
                } else {
                    st.hyph.hu[hn] = LigChar::from_raw((c - 65_536) / 1024 + 0xD800);
                    st.hyph.hc[hn] = (hc0 - 65_536) / 1024 + 0xD800;
                    word.hn += 1;
                    st.hyph.hu[hn + 1] = LigChar::from_raw(c % 1024 + 0xDC00);
                    st.hyph.hc[hn + 1] = hc0 % 1024 + 0xDC00;
                    l += 1;
                }
                st.hyph.hyf_bchar = LigChar::NONE;
            }
            l += 1;
        }
        return Ok(ha);
    }
}

/// Collect the letters of the word starting at `s` into `hu`/`hc`, setting
/// `hb` and `hyf_bchar`. Returns the node after the collected letters.
fn collect_word(st: &mut EngineState, word: &mut Word, mut s: MemPtr, max_len: i32) -> MemPtr {
    word.hn = 0;
    loop {
        if let Some(ch) = st.mem.as_char(s) {
            if st.mem[ch].font != st.hyph.hf {
                return s;
            }
            let c = i32::from(st.mem[ch].character);
            st.hyph.hyf_bchar = LigChar::from_raw(c);
            let hc0 = st.hyph.hyph_code(&st.eqtb, c);
            st.hyph.hc[0] = hc0;
            if hc0 == 0 || hc0 > st.hyph.max_hyph_char || i32::from(word.hn) == max_len {
                return s;
            }
            word.hb = s;
            word.hn += 1;
            let hn = usize::from(word.hn);
            st.hyph.hu[hn] = LigChar::from_raw(c);
            st.hyph.hc[hn] = hc0;
            st.hyph.hyf_bchar = LigChar::NONE;
        } else {
            match st.mem.classify(s) {
                Ok(AnyNode::Ligature(lig)) => {
                    if st.mem[lig].font != st.hyph.hf {
                        return s;
                    }
                    let mut j = word.hn;
                    let mut q = st.mem[lig].lig_ptr;
                    if q.raw() > TEX_NULL {
                        st.hyph.hyf_bchar =
                            LigChar::of(st.mem[NodePtr::<CharNode>::at(q)].character);
                    }
                    while q.raw() > TEX_NULL {
                        let c = i32::from(st.mem[NodePtr::<CharNode>::at(q)].character);
                        let hc0 = st.hyph.hyph_code(&st.eqtb, c);
                        st.hyph.hc[0] = hc0;
                        if hc0 == 0 || hc0 > st.hyph.max_hyph_char || i32::from(j) == max_len {
                            return s;
                        }
                        j += 1;
                        st.hyph.hu[usize::from(j)] = LigChar::from_raw(c);
                        st.hyph.hc[usize::from(j)] = hc0;
                        q = q.link(&st.mem);
                    }
                    word.hb = s;
                    word.hn = j;
                    st.hyph.hyf_bchar = if st.mem[lig].boundary.has_right() {
                        st.fonts.font_bchar[st.hyph.hf]
                    } else {
                        LigChar::NONE
                    };
                }
                Ok(AnyNode::Kern(k)) if st.mem[k].subtype == KernSubtype::Normal => {
                    word.hb = s;
                    st.hyph.hyf_bchar = st.fonts.font_bchar[st.hyph.hf];
                }
                _ => return s,
            }
        }
        s = s.link(&st.mem);
    }
}
