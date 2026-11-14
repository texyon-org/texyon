//! The character main loop: ligatures, kerns, native words, interchar tokens.

use crate::base::cmd::Cmd;
use crate::base::consts::CHAR_CLASS_LIMIT;
use crate::base::mode::Mode;
use crate::base::params::IntPar;
use crate::base::token::{Token, TokenList};
use crate::base::units::MemIdx;
use crate::base::units::{FontId, MemPtr};
use crate::fonts::ext::append_src_special;
use crate::fonts::{
    CharInfo, CharTag, LigChar, LigKernAction, LigKernIdx, LigKernStep, LigOp, SPACE_CODE,
};
use crate::memory::node::{
    AnyNode, CharNode, KernSubtype, LigBoundary, LigItem, NativeWordNode, NodePtr,
};
use crate::print::errors::Abort;
use crate::print::errors::OrConfusion;
use crate::scanning::input::{TOKEN_LIST, TokenType};
use crate::state::{EngineState, MemState};
use crate::tables::eqtb::{EqStore, InterCharPair};

fn inter_char_tokens_enabled(st: &EngineState) -> bool {
    st.eqtb.int_par(IntPar::XetexInterCharTokens) > 0
}

/// The `\XeTeXinterchartoks` list for class pair `n`, if set.
fn inter_char_toks(st: &EngineState, first: i32, second: i32) -> Option<TokenList> {
    let pair = InterCharPair::new(first, second)?;
    let list = st.eqtb.inter_char.get(pair);
    (!list.is_null()).then_some(list)
}

/// Inserts the inter-character tokens between the last character's class and
/// the boundary class, ahead of the current token. Returns whether it did.
pub(super) fn insert_boundary_inter_char_toks(st: &mut EngineState) -> Result<bool, Abort> {
    if !inter_char_tokens_enabled(st) {
        return Ok(false);
    }
    let sc = st.interchar.space_class;
    if sc == CHAR_CLASS_LIMIT || st.interchar.prev_class == CHAR_CLASS_LIMIT - 1 {
        return Ok(false);
    }
    st.interchar.prev_class = CHAR_CLASS_LIMIT - 1;
    let Some(list) = inter_char_toks(st, sc, CHAR_CLASS_LIMIT - 1) else {
        return Ok(false);
    };
    st.scanner.cur_tok = match st.scanner.cur_cs {
        None => st.scanner.cur.char_token(),
        Some(cs) => Token::from_cs(cs),
    };
    crate::scanning::input::back_input(st)?;
    crate::scanning::input::begin_token_list(st, list, TokenType::InterChar)?;
    Ok(true)
}

/// Inserts the inter-character tokens between the previous character's class
/// and class `sc` of the current character `chr`. Returns whether it did.
fn insert_inter_char_toks_before(st: &mut EngineState, sc: i32, chr: i32) -> Result<bool, Abort> {
    if !inter_char_tokens_enabled(st) || sc == CHAR_CLASS_LIMIT {
        return Ok(false);
    }
    let prev = st.interchar.prev_class;
    if prev == CHAR_CLASS_LIMIT - 1
        && st.input.cur_input.state == TOKEN_LIST
        && st.input.cur_input.token_type() == TokenType::BackedUpChar
    {
        return Ok(false);
    }
    let Some(list) = inter_char_toks(st, prev, sc) else {
        return Ok(false);
    };
    let cmd = match st.scanner.cur.cmd() {
        Cmd::Letter => Cmd::Letter,
        _ => Cmd::OtherChar,
    };
    st.scanner.cur_tok = Token::new(cmd, chr);
    crate::scanning::input::back_input(st)?;
    st.input.cur_input.set_token_type(TokenType::BackedUpChar);
    crate::scanning::input::begin_token_list(st, list, TokenType::InterChar)?;
    Ok(true)
}

fn note_char_class(st: &mut EngineState, sc: i32) {
    if inter_char_tokens_enabled(st) && sc != CHAR_CLASS_LIMIT {
        st.interchar.prev_class = sc;
    }
}

/// TeX's `adjust_space_factor` for `chr`. Also records and returns the
/// character's space class.
#[inline(always)]
fn adjust_space_factor(st: &mut EngineState, chr: i32) -> Result<i32, Abort> {
    let sf_code = st.eqtb.sf_code(chr);
    let s = sf_code % 65536;
    let Mode::Horizontal { space_factor, .. } = &mut st.input.cur_list.mode else {
        return Err(Abort::Confusion("space factor"));
    };
    if s == 1000 {
        *space_factor = 1000;
    } else if s < 1000 {
        if s > 0 {
            *space_factor = s;
        }
    } else if *space_factor < 1000 {
        *space_factor = 1000;
    } else {
        *space_factor = s;
    }
    st.interchar.space_class = sf_code / 65536;
    Ok(st.interchar.space_class)
}

#[inline]
fn is_hyphen_unit(st: &EngineState, c: i32, hyphen: i32) -> bool {
    c == hyphen || (st.eqtb.int_par(IntPar::XetexDashBreak) > 0 && (c == 8212 || c == 8211))
}

/// The offset just past the first hyphen in `len` units of native text at
/// `start`, `len` when there is none.
fn past_next_hyphen(st: &EngineState, f: FontId, start: i32, len: i32) -> i32 {
    let hyphen = st.fonts.hyphen_char[f];
    let mut h = 0;
    while h < len
        && !is_hyphen_unit(
            st,
            i32::from(st.scanner.native_text[(start + h) as usize]),
            hyphen,
        )
    {
        h += 1;
    }
    if h < len {
        h += 1;
    }
    h
}

/// The node before `pp` in the current list, skipping discretionary replacement lists.
fn node_before(st: &EngineState, pp: MemIdx) -> Result<MemIdx, Abort> {
    let mut ppp = st.input.cur_list.head;
    while ppp != pp && ppp.link(&st.mem) != Some(pp) {
        if !st.mem.is_char_node(ppp)
            && let Ok(AnyNode::Disc(disc)) = st.mem.classify(ppp)
        {
            for _ in 0..st.mem[disc].replace_count {
                ppp = ppp.link(&st.mem).or_confusion("main_ppp")?;
            }
        }
        if ppp != pp {
            ppp = ppp.link(&st.mem).or_confusion("main_ppp")?;
        }
    }
    Ok(ppp)
}

/// The native word `pp` if new text in font `f` can extend it. `ppp` is the node before it.
/// The list head counts as no char node, so a word at the start of a list merges too.
fn extendable_native_word(
    st: &EngineState,
    f: FontId,
    pp: MemIdx,
    ppp: MemIdx,
) -> Option<NodePtr<NativeWordNode>> {
    let word = st.mem.as_native_word(Some(pp))?;
    let extendable = st.mem[word].font == f
        && ppp != pp
        && !st.mem.is_char_node(ppp)
        && !matches!(st.mem.classify(ppp), Ok(AnyNode::Disc(_)));
    extendable.then_some(word)
}

impl MemState {
    /// Writes `text` into `node`'s text at `at`.
    fn write_native_text(&mut self, node: NodePtr<NativeWordNode>, at: usize, text: &[u16]) {
        self.entries_mut(node)[at..at + text.len()].copy_from_slice(text);
    }

    /// Copies the text of `src` into `node`'s text at `at`.
    fn copy_native_text_into(
        &mut self,
        node: NodePtr<NativeWordNode>,
        at: usize,
        src: NodePtr<NativeWordNode>,
    ) -> Result<(), Abort> {
        let (to, from) = self.entries_mut_and(node, src)?;
        to[at..at + from.len()].copy_from_slice(from);
        Ok(())
    }
}

fn use_glyph_metrics(st: &EngineState) -> bool {
    st.eqtb.int_par(IntPar::XetexUseGlyphMetrics) > 0
}

/// The main loop for an AAT or OpenType font: collects the run of characters
/// starting with `chr` into native word nodes. Returns whether to go to `big_switch`.
fn main_loop_native(st: &mut EngineState, mf: FontId, mut chr: i32) -> Result<bool, Abort> {
    if st.input.cur_list.mode.is_outer()
        && st.eqtb.int_par(IntPar::Language) != st.input.cur_list.clang()?
    {
        st.input.cur_list.fix_language(&mut st.mem, &st.eqtb)?;
    }
    // The offset just past the first hyphen of the text still to append, 0 for none.
    let mut h = 0;
    let mut is_hyph = false;
    st.scanner.native_len = 0;

    let inserted_toks = loop {
        let sc = adjust_space_factor(st, chr)?;
        let was_boundary = st.interchar.prev_class == CHAR_CLASS_LIMIT - 1;
        if insert_inter_char_toks_before(st, sc, chr)? {
            if was_boundary {
                return Ok(true);
            }
            st.interchar.prev_class = CHAR_CLASS_LIMIT - 1;
            break true;
        }
        note_char_class(st, sc);

        if chr > 65535 {
            st.scanner.reserve_native_text(2)?;
            st.scanner
                .push_native_text(((chr - 65536) / 1024 + 0xD800) as u16);
            st.scanner
                .push_native_text(((chr - 65536) % 1024 + 0xDC00) as u16);
        } else {
            st.scanner.reserve_native_text(1)?;
            st.scanner.push_native_text(chr as u16);
        }
        is_hyph = is_hyphen_unit(st, chr, st.fonts.hyphen_char[mf]);
        if h == 0 && is_hyph {
            h = st.scanner.native_len;
        }

        crate::scanning::scan::get_next(st)?;
        if let Some(c) = st.scanner.cur.char_code() {
            chr = c;
            continue;
        }
        crate::scanning::scan::x_token(st)?;
        if let Some(c) = st.scanner.cur.char_code() {
            chr = c;
            continue;
        }
        if st.scanner.cur.cmd() == Cmd::CharNum {
            chr = crate::scanning::scan::scan_usv_num(st)?;
            continue;
        }
        break insert_boundary_inter_char_toks(st)?;
    };

    if let Some(mapping) = st.fonts.font_mapping[mf].clone() {
        let text = &st.scanner.native_text[..st.scanner.native_len as usize];
        let mapped_len = st.fonts.apply_mapping(&mapping, text);
        st.scanner.native_len = 0;
        st.scanner.reserve_native_text(mapped_len)?;
        h = 0;
        let hyphen = st.fonts.hyphen_char[mf];
        for i in 0..mapped_len {
            let c = st.fonts.mapped_text[i as usize];
            st.scanner.push_native_text(c);
            if h == 0 && is_hyphen_unit(st, i32::from(c), hyphen) {
                h = st.scanner.native_len;
            }
        }
    }

    if st.eqtb.int_par(IntPar::TracingLostChars) > 0 {
        let mut i = 0;
        while i < st.scanner.native_len {
            let mut c = i32::from(st.scanner.native_text[i as usize]);
            i += 1;
            if (0xD800..0xDC00).contains(&c) {
                c = 65536 + (c - 0xD800) * 1024 + i32::from(st.scanner.native_text[i as usize])
                    - 0xDC00;
                i += 1;
            }
            if st.fonts.map_char_to_glyph(mf, c)? == 0 {
                crate::print::char_warning(st, mf, c)?;
            }
        }
    }

    let len = st.scanner.native_len;
    let mut pp = st.input.cur_list.tail;
    let mut ppp = node_before(st, pp)?;
    if st.input.cur_list.mode.is_paragraph() {
        let mut start = 0;
        let mut remaining = len;
        loop {
            if h == 0 {
                h = remaining;
            }
            if let Some(word) = extendable_native_word(st, mf, pp, ppp) {
                let old_len = i32::from(st.mem[word].length);
                remaining = h + old_len;
                st.scanner.reserve_native_text(remaining)?;
                let saved_len = st.scanner.native_len;
                for &c in st.mem.entries(word) {
                    st.scanner.push_native_text(c);
                }
                for i in 0..h {
                    let c = st.scanner.native_text[(start + i) as usize];
                    st.scanner.push_native_text(c);
                }
                crate::build::linebreak::do_locale_linebreaks(st, mf, saved_len, remaining)?;
                st.scanner.native_len = saved_len;
                remaining = saved_len - h - start;
                start = h;
                h = past_next_hyphen(st, mf, start, remaining);
                let next = st.mem[word].link;
                ppp.set_link(&mut st.mem, next);
                st.mem[word].link = None;
                st.mem.flush_node_list(word.ptr())?;
                pp = st.input.cur_list.tail;
                ppp = st.mem.predecessor(ppp, Some(pp))?;
            } else {
                crate::build::linebreak::do_locale_linebreaks(st, mf, start, h)?;
                start += h;
                remaining -= h;
                h = past_next_hyphen(st, mf, start, remaining);
            }
            if remaining > 0 || is_hyph {
                let disc = st.mem.new_disc()?;
                st.input.cur_list.tail_append(&mut st.mem, disc);
                pp = st.input.cur_list.tail;
            }
            if remaining == 0 {
                break;
            }
        }
    } else {
        let old = extendable_native_word(st, mf, pp, ppp);
        let old_len = old.map_or(0, |w| st.mem[w].length);
        let node = st
            .mem
            .new_native_word_node(&st.eqtb, mf, len + i32::from(old_len))?;
        pp.set_link(&mut st.mem, node.ptr());
        st.input.cur_list.tail = node.idx();
        if let Some(word) = old {
            st.mem.copy_native_text_into(node, 0, word)?;
        }
        st.mem.write_native_text(
            node,
            usize::from(old_len),
            &st.scanner.native_text[..len as usize],
        );
        let metrics = use_glyph_metrics(st);
        st.fonts.measure_native_node(&mut st.mem, node, metrics)?;
        if let Some(word) = old {
            let mut prev = st.input.cur_list.head;
            if prev != word.idx() {
                prev = st.mem.predecessor(prev, word.ptr())?;
            }
            let next = st.mem[word].link;
            prev.set_link(&mut st.mem, next);
            st.mem[word].link = None;
            st.mem.flush_node_list(word.ptr())?;
        }
    }

    if st.eqtb.int_par(IntPar::XetexInterwordSpaceShaping) > 0 {
        shape_interword_space(st, mf)?;
    }
    Ok(inserted_toks)
}

/// Kerns the interword glue before the current word by what the font's shaping
/// of `word space word` implies.
fn shape_interword_space(st: &mut EngineState, mf: FontId) -> Result<(), Abort> {
    let tail = st.input.cur_list.tail;
    let mut p = st.input.cur_list.head;
    let mut prev_word = None;
    while p != tail {
        prev_word = st.mem.as_native_word(Some(p)).or(prev_word);
        p = p.link(&st.mem).or_confusion("interword space")?;
    }
    let Some(prev_word) = prev_word.filter(|&w| st.mem[w].font == mf) else {
        return Ok(());
    };
    let glue = st.mem.skip_interword_fillers(st.mem[prev_word].link);
    let Some(AnyNode::Glue(glue)) = st.mem.classify_non_char(glue) else {
        return Ok(());
    };
    if st.mem.skip_interword_fillers(st.mem[glue].link) != Some(tail) {
        return Ok(());
    }
    let Some(tail_word) = st.mem.as_native_word(Some(tail)) else {
        return Ok(());
    };
    let prev_len = usize::from(st.mem[prev_word].length);
    let tail_len = usize::from(st.mem[tail_word].length);
    let len = prev_len + 1 + tail_len;
    let joined = st.mem.new_native_word_node(&st.eqtb, mf, len as i32)?;
    st.mem.copy_native_text_into(joined, 0, prev_word)?;
    st.mem
        .write_native_text(joined, prev_len, &[u16::from(b' ')]);
    st.mem
        .copy_native_text_into(joined, prev_len + 1, tail_word)?;
    let metrics = use_glyph_metrics(st);
    st.fonts.measure_native_node(&mut st.mem, joined, metrics)?;
    let t = st.mem[joined].width - st.mem[prev_word].width - st.mem[tail_word].width;
    joined.free_native(&mut st.mem);
    // TeX reads `width(font_glue[f])`, which stays null while `\spaceskip` supplies the glue.
    let space = st.fonts.param(mf, SPACE_CODE);
    if t != space {
        let kern = st.mem.new_kern(t - space)?;
        st.mem[kern].subtype = KernSubtype::SpaceAdjustment.into();
        let next = st.mem[glue].link;
        st.mem[kern].link = next;
        st.mem[glue].link = kern.ptr();
    }
    Ok(())
}

/// The top of the ligature stack: the lookahead character, or a lig item that
/// a ligature instruction pushed.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum LigStack {
    Char(NodePtr<CharNode>),
    Item(NodePtr<LigItem>),
}

impl LigStack {
    #[inline]
    fn idx(self) -> MemIdx {
        match self {
            LigStack::Char(c) => c.idx(),
            LigStack::Item(i) => i.idx(),
        }
    }

    /// The character it carries.
    #[inline]
    fn character(self, mem: &MemState) -> LigChar {
        LigChar::of(match self {
            LigStack::Char(c) => mem[c].character,
            LigStack::Item(i) => mem[i].character,
        })
    }
}

impl MemState {
    /// The ligature stack entry at link `p`, `None` when the stack ends there.
    #[inline]
    fn lig_stack_at(&self, p: MemPtr) -> Option<LigStack> {
        let p = p?;
        Some(if self.is_char_node(p) {
            LigStack::Char(NodePtr::at(p))
        } else {
            LigStack::Item(NodePtr::at(p))
        })
    }
}

/// The registers of one run of the TFM character loop.
struct LigKern {
    font: FontId,
    /// The character the loop is at, TeX's `cur_chr`.
    chr: i32,
    bchar: LigChar,
    false_bchar: LigChar,
    cur_l: LigChar,
    cur_r: LigChar,
    /// The node the pending ligature's characters follow.
    cur_q: MemIdx,
    lig_stack: Option<LigStack>,
    ligature_present: bool,
    lft_hit: bool,
    rt_hit: bool,
    /// Whether inter-character tokens ended the word, so the big switch reads them next.
    inserted_toks: bool,
}

impl LigKern {
    /// The character on top of `lig_stack`, a char node or a lig item alike.
    #[inline(always)]
    fn lig_stack_char(&self, mem: &MemState) -> LigChar {
        self.lig_stack
            .map_or(LigChar::NONE, |top| top.character(mem))
    }

    /// Nothing follows `cur_l` but the right boundary.
    #[inline]
    fn end_word(&mut self) {
        self.cur_r = self.bchar;
        self.lig_stack = None;
    }

    /// The char node on top of `lig_stack`, the lookahead character.
    #[inline]
    fn lookahead_node(&self) -> Result<NodePtr<CharNode>, Abort> {
        let top = self.lig_stack.or_confusion("lig_stack")?;
        Ok(NodePtr::at(top.idx()))
    }
}

/// Where the ligature/kern program scan ended.
enum LigExit {
    /// `main_loop_wrapup`.
    Wrapup,
    /// `main_loop_move`, skipping the wrapup.
    Move,
    /// `main_loop_move+1`.
    Move1,
    /// Back to `main_lig_loop` with a new `cur_l`.
    Restart,
}

/// Where the next pass of the TFM main loop starts.
enum Resume {
    /// `main_lig_loop`.
    LigLoop,
    /// `main_lig_loop+1` at lig/kern program index `k`.
    LigProgram(LigKernIdx),
    /// `main_loop_move+2`.
    Move2,
}

/// TeX's `wrapup`: packs a pending ligature and inserts a pending
/// discretionary. `rt_hit` says whether a right boundary hit may count.
#[inline(always)]
fn wrapup(st: &mut EngineState, lk: &mut LigKern, rt_hit: bool) -> Result<(), Abort> {
    if !lk.cur_l.is_char() {
        return Ok(());
    }
    let ins_disc = lk.cur_q.link(&st.mem).is_some()
        && i32::from(st.mem[NodePtr::<CharNode>::at(st.input.cur_list.tail)].character)
            == st.fonts.hyphen_char[lk.font];
    if lk.ligature_present {
        let lig_list = lk.cur_q.link(&st.mem);
        let lig = st.mem.new_ligature(lk.font, lk.cur_l.code(), lig_list)?;
        let mut boundary = LigBoundary::None;
        if lk.lft_hit {
            boundary = LigBoundary::Left;
            lk.lft_hit = false;
        }
        if rt_hit && lk.rt_hit && lk.lig_stack.is_none() {
            boundary = boundary.with_right();
            lk.rt_hit = false;
        }
        st.mem[lig].boundary = boundary.into();
        lk.cur_q.set_link(&mut st.mem, lig.ptr());
        st.input.cur_list.tail = lig.idx();
        lk.ligature_present = false;
    }
    if ins_disc && st.input.cur_list.mode.is_outer() {
        let disc = st.mem.new_disc()?;
        st.input.cur_list.tail_append(&mut st.mem, disc);
    }
    Ok(())
}

#[inline(always)]
fn cur_l_info(st: &mut EngineState, lk: &LigKern) -> Option<CharInfo> {
    let c = st.fonts.effective_char(lk.font, lk.cur_l.code());
    st.fonts.char_info(lk.font, c)
}

/// The main loop for the character `chr`. `cancel_boundary` says `\noboundary`
/// came right before it. Returns true to go to `big_switch`, false to go to `reswitch`.
pub(super) fn main_control_main_loop(
    st: &mut EngineState,
    chr: i32,
    cancel_boundary: bool,
) -> Result<bool, Abort> {
    if st.input.cur_list.head == st.input.cur_list.tail
        && st.input.cur_list.mode.is_outer()
        && st.shipout.insert_src_special_auto
    {
        append_src_special(st)?;
    }
    st.interchar.prev_class = CHAR_CLASS_LIMIT - 1;

    let mf = st.eqtb.cur_font();
    if st.fonts.is_native_font(mf) {
        return main_loop_native(st, mf, chr);
    }

    let sc = adjust_space_factor(st, chr)?;
    if insert_inter_char_toks_before(st, sc, chr)? {
        return Ok(true);
    }
    note_char_class(st, sc);

    if st.input.cur_list.mode.is_outer()
        && st.eqtb.int_par(IntPar::Language) != st.input.cur_list.clang()?
    {
        st.input.cur_list.fix_language(&mut st.mem, &st.eqtb)?;
    }
    let cur_l = LigChar::from_raw(chr);
    let first = st.mem.new_char(mf, cur_l.code())?;
    let mut lk = LigKern {
        font: mf,
        chr,
        bchar: st.fonts.font_bchar[mf],
        false_bchar: st.fonts.font_false_bchar[mf],
        cur_l,
        cur_r: LigChar::NONE,
        cur_q: st.input.cur_list.tail,
        lig_stack: Some(LigStack::Char(first)),
        ligature_present: false,
        lft_hit: false,
        rt_hit: false,
        inserted_toks: false,
    };

    let bchar_label = if cancel_boundary {
        None
    } else {
        st.fonts.bchar_label(mf)
    };
    let mut resume = if let Some(label) = bchar_label {
        lk.cur_r = lk.cur_l;
        lk.cur_l = LigChar::NONE;
        Resume::LigProgram(label)
    } else {
        Resume::Move2
    };
    let mut main_i = None;

    loop {
        let exit = match std::mem::replace(&mut resume, Resume::LigLoop) {
            Resume::Move2 => None,
            Resume::LigProgram(k) => {
                let step = st.fonts.lig_kern_step(lk.font, k)?;
                Some(lig_kern_program(st, &mut lk, k, step, &mut main_i)?)
            }
            Resume::LigLoop => match main_i.map(|i| i.tag) {
                Some(CharTag::Lig(start)) if lk.cur_r != LigChar::NONE => {
                    let (k, step) = st.fonts.lig_kern_start(lk.font, start)?;
                    Some(lig_kern_program(st, &mut lk, k, step, &mut main_i)?)
                }
                _ => Some(LigExit::Wrapup),
            },
        };
        if let Some(exit) = exit {
            match exit {
                LigExit::Restart => continue,
                LigExit::Wrapup => wrapup(st, &mut lk, true)?,
                LigExit::Move | LigExit::Move1 => {}
            }
            if !matches!(exit, LigExit::Move1) {
                if lk.lig_stack.is_none() {
                    return Ok(lk.inserted_toks);
                }
                lk.cur_q = st.input.cur_list.tail;
                lk.cur_l = lk.lig_stack_char(&st.mem);
            }

            if let Some(LigStack::Item(item)) = lk.lig_stack {
                let lig_ptr = st.mem[item].component;
                if let Some(component) = lig_ptr {
                    st.input.cur_list.tail_append(&mut st.mem, component);
                }
                lk.lig_stack = st.mem.lig_stack_at(st.mem[item].link);
                item.free(&mut st.mem);
                main_i = cur_l_info(st, &lk);
                lk.ligature_present = true;
                if lk.lig_stack.is_none() {
                    if lig_ptr.is_some() {
                        lookahead(st, &mut lk)?;
                        continue;
                    }
                    lk.cur_r = lk.bchar;
                } else {
                    lk.cur_r = lk.lig_stack_char(&st.mem);
                }
                continue;
            }
        }

        let cur_chr = lk.chr as u16;
        // Each call resets `xtx_ligature_present`, so the two bounds may see different codes.
        let exists = if st.fonts.effective_char(lk.font, cur_chr)
            > i32::from(st.fonts.font_ec[lk.font])
            || st.fonts.effective_char(lk.font, cur_chr) < i32::from(st.fonts.font_bc[lk.font])
        {
            false
        } else {
            main_i = st.fonts.effective_char_info(lk.font, lk.cur_l.code());
            main_i.is_some()
        };
        let node = lk.lookahead_node()?;
        if !exists {
            crate::print::char_warning(st, lk.font, lk.chr)?;
            node.free(&mut st.mem);
            return Ok(true);
        }
        st.input.cur_list.tail_append(&mut st.mem, node);
        lookahead(st, &mut lk)?;
    }
}

/// Runs the font's ligature/kern program for `cur_l` and `cur_r` from
/// instruction `step` at index `k`.
#[inline(always)]
fn lig_kern_program(
    st: &mut EngineState,
    lk: &mut LigKern,
    mut k: LigKernIdx,
    mut step: LigKernStep,
    main_i: &mut Option<CharInfo>,
) -> Result<LigExit, Abort> {
    loop {
        let Some(action) = step.action_for(lk.cur_r) else {
            let Some(next) = step.next(k) else {
                return Ok(LigExit::Wrapup);
            };
            k = next;
            step = st.fonts.lig_kern_step(lk.font, k)?;
            continue;
        };
        let (op, lig_char) = match action {
            LigKernAction::Kern(width) => {
                wrapup(st, lk, true)?;
                let kern = st.mem.new_kern(width)?;
                st.input.cur_list.tail_append(&mut st.mem, kern);
                return Ok(LigExit::Move);
            }
            LigKernAction::Lig { op, char } => (op, LigChar::of(char)),
        };
        if lk.cur_l == LigChar::NONE {
            lk.lft_hit = true;
        } else if lk.lig_stack.is_none() {
            lk.rt_hit = true;
        }
        match op {
            LigOp::KeepRight | LigOp::KeepRightSkip => {
                lk.cur_l = lig_char;
                *main_i = cur_l_info(st, lk);
                lk.ligature_present = true;
            }
            LigOp::KeepLeft | LigOp::KeepLeftSkip => {
                lk.cur_r = lig_char;
                let c = lk.cur_r.code();
                match lk.lig_stack {
                    None => {
                        let item = st.mem.new_lig_item(c)?;
                        lk.lig_stack = Some(LigStack::Item(item));
                        lk.bchar = LigChar::NONE;
                    }
                    Some(LigStack::Char(prev)) => {
                        let item = st.mem.new_lig_item(c)?;
                        lk.lig_stack = Some(LigStack::Item(item));
                        st.mem[item].component = prev.ptr();
                    }
                    Some(LigStack::Item(item)) => st.mem[item].character = c,
                }
            }
            LigOp::KeepBoth => {
                lk.cur_r = lig_char;
                let below = lk.lig_stack.map(LigStack::idx);
                let item = st.mem.new_lig_item(lk.cur_r.code())?;
                lk.lig_stack = Some(LigStack::Item(item));
                item.idx().set_link(&mut st.mem, below);
            }
            LigOp::KeepBothSkip | LigOp::KeepBothSkip2 => {
                wrapup(st, lk, false)?;
                lk.cur_q = st.input.cur_list.tail;
                lk.cur_l = lig_char;
                *main_i = cur_l_info(st, lk);
                lk.ligature_present = true;
            }
            LigOp::Replace => {
                lk.cur_l = lig_char;
                lk.ligature_present = true;
                return Ok(if lk.lig_stack.is_none() {
                    LigExit::Wrapup
                } else {
                    LigExit::Move1
                });
            }
        }
        if matches!(
            op,
            LigOp::KeepRightSkip | LigOp::KeepLeftSkip | LigOp::KeepBothSkip2
        ) {
            return Ok(LigExit::Wrapup);
        }
        if lk.cur_l.is_char() {
            return Ok(LigExit::Restart);
        }
        k = st
            .fonts
            .bchar_label(lk.font)
            .or_confusion("boundary program")?;
        step = st.fonts.lig_kern_step(lk.font, k)?;
    }
}

/// TeX's `main_loop_lookahead`: fetches the next character into a new
/// `lig_stack` and `cur_r`. Anything else ends the word, and so do
/// inter-character tokens inserted before the character.
#[inline(always)]
fn lookahead(st: &mut EngineState, lk: &mut LigKern) -> Result<(), Abort> {
    crate::scanning::scan::get_next(st)?;
    if st.scanner.cur.char_code().is_none() {
        crate::scanning::scan::x_token(st)?;
    }
    if let Some(c) = st.scanner.cur.char_code() {
        lk.chr = c;
    } else {
        if st.scanner.cur.cmd() != Cmd::CharNum {
            if st.scanner.cur.cmd() == Cmd::NoBoundary {
                lk.bchar = LigChar::NONE;
            }
            lk.end_word();
            return Ok(());
        }
        lk.chr = crate::scanning::scan::scan_char_num(st)?;
    }

    let sc = adjust_space_factor(st, lk.chr)?;
    let was_boundary = st.interchar.prev_class == CHAR_CLASS_LIMIT - 1;
    if insert_inter_char_toks_before(st, sc, lk.chr)? {
        if !was_boundary {
            st.interchar.prev_class = CHAR_CLASS_LIMIT - 1;
        }
        // WEB leaves here without `wrapup`, which loses a pending ligature (UPSTREAM_BUGS.md 15).
        lk.inserted_toks = true;
        lk.end_word();
        return Ok(());
    }
    note_char_class(st, sc);

    let node = st.mem.new_char(lk.font, lk.chr as u16)?;
    lk.lig_stack = Some(LigStack::Char(node));
    lk.cur_r = LigChar::from_raw(lk.chr);
    if lk.cur_r == lk.false_bchar {
        lk.cur_r = LigChar::NONE;
    }
    Ok(())
}
