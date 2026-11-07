//! The character main loop: ligatures, kerns, native words, interchar tokens.

use crate::cmd::Cmd;
use crate::codes::ValueLevel;
use crate::consts::{CHAR_CLASS_LIMIT, HMODE, NATIVE_NODE_SIZE, SPACE_ADJUSTMENT, TEX_NULL};
use crate::errors::Abort;
use crate::ext::append_src_special;
use crate::fonts::{CharInfo, CharTag, LigChar, LigKernStep, SPACE_CODE};
use crate::input::{TokenType, TOKEN_LIST};
use crate::node::{AnyNode, CharNode, LigItem, NodePtr, WhatsitNode};
use crate::params::IntPar;
use crate::state::{EngineState, MemState};
use crate::token::{Token, TokenList};
use crate::units::{FontId, FontInfoIdx, MemIdx, MemPtr};

fn inter_char_tokens_enabled(st: &EngineState) -> bool {
    st.eqtb.int_par(IntPar::XetexInterCharTokens) > 0
}

/// The `\XeTeXinterchartoks` list for class pair `n`, if set. Leaves the
/// pair's trie record, possibly null, in `cur_ptr`.
fn inter_char_toks(st: &mut EngineState, n: i32) -> Option<TokenList> {
    let list = st
        .mem
        .find_sa_element(&mut st.scanner, ValueLevel::InterChar, n)?
        .sa_toks(&st.mem);
    (!list.is_null()).then_some(list)
}

/// Inserts the inter-character tokens between the last character's class and
/// the boundary class, ahead of the current token. Returns whether it did.
pub(super) fn insert_boundary_inter_char_toks(st: &mut EngineState) -> Result<bool, Abort> {
    if !inter_char_tokens_enabled(st) {
        return Ok(false);
    }
    let sc = st.mainloop.space_class;
    if sc == CHAR_CLASS_LIMIT || st.mainloop.prev_class == CHAR_CLASS_LIMIT - 1 {
        return Ok(false);
    }
    st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;
    let Some(list) = inter_char_toks(st, sc * CHAR_CLASS_LIMIT + CHAR_CLASS_LIMIT - 1) else {
        return Ok(false);
    };
    if st.scanner.cur_cs.is_none() {
        if st.scanner.cur_cmd == Cmd::CharNum {
            st.scanner.cur_cmd = Cmd::OtherChar;
        }
        st.scanner.cur_tok = Token::new(st.scanner.cur_cmd, st.scanner.cur_chr);
    } else {
        st.scanner.cur_tok = Token::from_cs(st.scanner.cur_cs);
    }
    crate::input::back_input(st)?;
    crate::input::begin_token_list(st, list, TokenType::InterChar)?;
    Ok(true)
}

/// Inserts the inter-character tokens between the previous character's class
/// and class `sc` of the current character. Returns whether it did.
fn insert_inter_char_toks_before(st: &mut EngineState, sc: i32) -> Result<bool, Abort> {
    if !inter_char_tokens_enabled(st) || sc == CHAR_CLASS_LIMIT {
        return Ok(false);
    }
    let prev = st.mainloop.prev_class;
    if prev == CHAR_CLASS_LIMIT - 1
        && st.input.cur_input.state == TOKEN_LIST
        && st.input.cur_input.token_type() == TokenType::BackedUpChar
    {
        return Ok(false);
    }
    let Some(list) = inter_char_toks(st, prev * CHAR_CLASS_LIMIT + sc) else {
        return Ok(false);
    };
    if st.scanner.cur_cmd != Cmd::Letter {
        st.scanner.cur_cmd = Cmd::OtherChar;
    }
    st.scanner.cur_tok = Token::new(st.scanner.cur_cmd, st.scanner.cur_chr);
    crate::input::back_input(st)?;
    st.input.cur_input.set_token_type(TokenType::BackedUpChar);
    crate::input::begin_token_list(st, list, TokenType::InterChar)?;
    Ok(true)
}

fn note_char_class(st: &mut EngineState, sc: i32) {
    if inter_char_tokens_enabled(st) && sc != CHAR_CLASS_LIMIT {
        st.mainloop.prev_class = sc;
    }
}

/// TeX's `adjust_space_factor` for `cur_chr`. Also resets `cur_ptr` and
/// records and returns the character's space class.
#[inline(always)]
fn adjust_space_factor(st: &mut EngineState) -> i32 {
    let sf_code = st.eqtb.sf_code(st.scanner.cur_chr);
    let s = sf_code % 65536;
    if s == 1000 {
        st.input.cur_list.set_space_factor(1000);
    } else if s < 1000 {
        if s > 0 {
            st.input.cur_list.set_space_factor(s);
        }
    } else if st.input.cur_list.space_factor() < 1000 {
        st.input.cur_list.set_space_factor(1000);
    } else {
        st.input.cur_list.set_space_factor(s);
    }
    st.scanner.cur_ptr = MemPtr::NULL;
    st.mainloop.space_class = sf_code / 65536;
    st.mainloop.space_class
}

fn is_char_cmd(cmd: Cmd) -> bool {
    matches!(cmd, Cmd::Letter | Cmd::OtherChar | Cmd::CharGiven)
}

#[inline]
fn is_hyphen_unit(st: &EngineState, c: i32, hyphen: i32) -> bool {
    c == hyphen || (st.eqtb.int_par(IntPar::XetexDashBreak) > 0 && (c == 8212 || c == 8211))
}

/// Advances `main_h` just past the next hyphen in `len` units of native text at `start`.
fn advance_past_hyphen(st: &mut EngineState, start: i32, len: i32) {
    let hyphen = st.fonts.hyphen_char[st.mainloop.main_f];
    while st.mainloop.main_h < len
        && !is_hyphen_unit(
            st,
            i32::from(st.scanner.native_text[(start + st.mainloop.main_h) as usize]),
            hyphen,
        )
    {
        st.mainloop.main_h += 1;
    }
    if st.mainloop.main_h < len {
        st.mainloop.main_h += 1;
    }
}

/// Points `main_ppp` at the node before `main_pp`, skipping discretionary replacement lists.
fn find_main_ppp(st: &mut EngineState) {
    let pp = st.mainloop.main_pp;
    let mut ppp = st.input.cur_list.head;
    while ppp != pp && ppp.link(&st.mem) != pp {
        if !st.mem.is_char_node(ppp) {
            if let Ok(AnyNode::Disc(disc)) = st.mem.classify(ppp) {
                for _ in 0..st.mem[disc].subtype {
                    ppp = ppp.link(&st.mem);
                }
            }
        }
        if ppp != pp {
            ppp = ppp.link(&st.mem);
        }
    }
    st.mainloop.main_ppp = ppp;
}

/// The native word `main_pp` if new text in font `f` can extend it.
fn extendable_native_word(st: &EngineState, f: FontId) -> Option<NodePtr<WhatsitNode>> {
    let (pp, ppp) = (st.mainloop.main_pp, st.mainloop.main_ppp);
    if pp.is_null() {
        return None;
    }
    let word = st.mem.as_native_word(pp)?;
    let extendable = word.native_font(&st.mem) == f
        && ppp != pp
        && !st.mem.is_char_node(ppp)
        && !matches!(st.mem.classify(ppp), Ok(AnyNode::Disc(_)));
    extendable.then_some(word)
}

impl MemState {
    /// Writes `text` into `node`'s text at `at`, unjournaled.
    fn write_native_text(&mut self, node: NodePtr<WhatsitNode>, at: i32, text: &[u16]) {
        let to = node.native_text_start() + at as usize;
        self.units_mut()[to..to + text.len()].copy_from_slice(text);
    }

    /// Copies the text of `src` into `node`'s text at `at`, unjournaled.
    fn copy_native_text_into(
        &mut self,
        node: NodePtr<WhatsitNode>,
        at: i32,
        src: NodePtr<WhatsitNode>,
    ) {
        let from = src.native_text_start();
        let len = src.native_length(self) as usize;
        self.units_mut()
            .copy_within(from..from + len, node.native_text_start() + at as usize);
    }
}

fn use_glyph_metrics(st: &EngineState) -> bool {
    st.eqtb.int_par(IntPar::XetexUseGlyphMetrics) > 0
}

/// The main loop for an AAT or OpenType font: collects a run of characters
/// into native word nodes. Returns whether to go to `big_switch`.
fn main_loop_native(st: &mut EngineState, mf: FontId) -> Result<bool, Abort> {
    if st.input.cur_list.mode > 0 && st.eqtb.int_par(IntPar::Language) != st.input.cur_list.clang()
    {
        st.input.cur_list.fix_language(&mut st.mem, &st.eqtb)?;
    }
    st.mainloop.main_h = 0;
    st.mainloop.main_f = mf;
    st.scanner.native_len = 0;

    loop {
        let sc = adjust_space_factor(st);
        let was_boundary = st.mainloop.prev_class == CHAR_CLASS_LIMIT - 1;
        if insert_inter_char_toks_before(st, sc)? {
            if was_boundary {
                return Ok(true);
            }
            st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;
            break;
        }
        note_char_class(st, sc);

        let ch = st.scanner.cur_chr;
        if ch > 65535 {
            st.scanner.reserve_native_text(2)?;
            st.scanner
                .push_native_text(((ch - 65536) / 1024 + 0xD800) as u16);
            st.scanner
                .push_native_text(((ch - 65536) % 1024 + 0xDC00) as u16);
        } else {
            st.scanner.reserve_native_text(1)?;
            st.scanner.push_native_text(ch as u16);
        }
        let is_hyph = is_hyphen_unit(st, ch, st.fonts.hyphen_char[mf]);
        st.hyph.is_hyph = is_hyph;
        if st.mainloop.main_h == 0 && is_hyph {
            st.mainloop.main_h = st.scanner.native_len;
        }

        crate::scan::get_next(st)?;
        if is_char_cmd(st.scanner.cur_cmd) {
            continue;
        }
        crate::scan::x_token(st)?;
        if is_char_cmd(st.scanner.cur_cmd) {
            continue;
        }
        if st.scanner.cur_cmd == Cmd::CharNum {
            st.scanner.cur_chr = crate::scan::scan_usv_num(st)?;
            continue;
        }
        insert_boundary_inter_char_toks(st)?;
        break;
    }

    if let Some(mapping) = st.fonts.font_mapping[mf].clone() {
        let text = &st.scanner.native_text[..st.scanner.native_len as usize];
        let mapped_len = st.fonts.apply_mapping(&mapping, text);
        st.scanner.native_len = 0;
        st.scanner.reserve_native_text(mapped_len)?;
        st.mainloop.main_h = 0;
        let hyphen = st.fonts.hyphen_char[mf];
        for i in 0..mapped_len {
            let c = st.fonts.mapped_text[i as usize];
            st.scanner.push_native_text(c);
            if st.mainloop.main_h == 0 && is_hyphen_unit(st, i32::from(c), hyphen) {
                st.mainloop.main_h = st.scanner.native_len;
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
                crate::output::char_warning(st, mf, c)?;
            }
        }
    }

    let len = st.scanner.native_len;
    st.mainloop.main_pp = st.input.cur_list.tail;
    find_main_ppp(st);
    if st.input.cur_list.mode == HMODE {
        let mut start = 0;
        let mut remaining = len;
        loop {
            if st.mainloop.main_h == 0 {
                st.mainloop.main_h = remaining;
            }
            if let Some(word) = extendable_native_word(st, mf) {
                let old_len = i32::from(word.native_length(&st.mem));
                remaining = st.mainloop.main_h + old_len;
                st.scanner.reserve_native_text(remaining)?;
                st.scanner.save_native_len = st.scanner.native_len;
                for &c in word.native_text(&st.mem) {
                    st.scanner.push_native_text(c);
                }
                for i in 0..st.mainloop.main_h {
                    let c = st.scanner.native_text[(start + i) as usize];
                    st.scanner.push_native_text(c);
                }
                crate::linebreak::do_locale_linebreaks(st, st.scanner.save_native_len, remaining)?;
                st.scanner.native_len = st.scanner.save_native_len;
                remaining = st.scanner.native_len - st.mainloop.main_h - start;
                start = st.mainloop.main_h;
                st.mainloop.main_h = 0;
                advance_past_hyphen(st, start, remaining);
                let next = word.link(&st.mem);
                st.mainloop.main_ppp.set_link(&mut st.mem, next);
                word.set_link(&mut st.mem, MemPtr::NULL);
                st.mem.flush_node_list(word.ptr())?;
                st.mainloop.main_pp = st.input.cur_list.tail;
                while st.mainloop.main_ppp.link(&st.mem) != st.mainloop.main_pp {
                    st.mainloop.main_ppp = st.mainloop.main_ppp.link(&st.mem);
                }
            } else {
                crate::linebreak::do_locale_linebreaks(st, start, st.mainloop.main_h)?;
                start += st.mainloop.main_h;
                remaining -= st.mainloop.main_h;
                st.mainloop.main_h = 0;
                advance_past_hyphen(st, start, remaining);
            }
            if remaining > 0 || st.hyph.is_hyph {
                let disc = st.mem.new_disc()?;
                st.input.cur_list.tail_append(&mut st.mem, disc.ptr());
                st.mainloop.main_pp = st.input.cur_list.tail;
            }
            if remaining == 0 {
                break;
            }
        }
    } else {
        let pp = st.mainloop.main_pp;
        let old = extendable_native_word(st, mf);
        let old_len = old.map_or(0, |w| i32::from(w.native_length(&st.mem)));
        let node = st.mem.new_native_word_node(&st.eqtb, mf, len + old_len)?;
        pp.set_link(&mut st.mem, node.ptr());
        st.input.cur_list.tail = node.ptr();
        st.mem.journal_mark_range(
            node.idx().offset(NATIVE_NODE_SIZE),
            crate::state::native_text_words(len + old_len),
        );
        if let Some(word) = old {
            st.mem.copy_native_text_into(node, 0, word);
        }
        st.mem
            .write_native_text(node, old_len, &st.scanner.native_text[..len as usize]);
        let metrics = use_glyph_metrics(st);
        st.fonts.measure_native_node(&mut st.mem, node, metrics)?;
        if let Some(word) = old {
            let mut prev = st.input.cur_list.head;
            if prev != word.ptr() {
                while prev.link(&st.mem) != word.ptr() {
                    prev = prev.link(&st.mem);
                }
            }
            let next = word.link(&st.mem);
            prev.set_link(&mut st.mem, next);
            word.set_link(&mut st.mem, MemPtr::NULL);
            st.mem.flush_node_list(word.ptr())?;
        }
    }

    if st.eqtb.int_par(IntPar::XetexInterwordSpaceShaping) > 0 {
        shape_interword_space(st, mf)?;
    }
    Ok(!st.scanner.cur_ptr.is_null())
}

/// Kerns the interword glue before the current word by what the font's shaping
/// of `word space word` implies.
fn shape_interword_space(st: &mut EngineState, mf: FontId) -> Result<(), Abort> {
    let tail = st.input.cur_list.tail;
    let mut p = st.input.cur_list.head;
    let mut prev_word = None;
    while p != tail {
        if !p.is_null() {
            prev_word = st.mem.as_native_word(p).or(prev_word);
        }
        p = p.link(&st.mem);
    }
    let Some(prev_word) = prev_word.filter(|w| w.native_font(&st.mem) == mf) else {
        return Ok(());
    };
    let glue = st.mem.skip_interword_fillers(prev_word.link(&st.mem));
    if st.mem.is_char_node(glue) {
        return Ok(());
    }
    let Ok(AnyNode::Glue(glue)) = st.mem.classify(glue) else {
        return Ok(());
    };
    if st.mem.skip_interword_fillers(glue.link(&st.mem)) != tail {
        return Ok(());
    }
    let tail_word = NodePtr::<WhatsitNode>::from_idx_unchecked(MemIdx::new(tail));
    let prev_len = i32::from(prev_word.native_length(&st.mem));
    let tail_len = i32::from(tail_word.native_length(&st.mem));
    let len = prev_len + 1 + tail_len;
    let joined = st.mem.new_native_word_node(&st.eqtb, mf, len)?;
    st.mem.journal_mark_range(
        joined.idx().offset(NATIVE_NODE_SIZE),
        crate::state::native_text_words(len),
    );
    st.mem.copy_native_text_into(joined, 0, prev_word);
    st.mem
        .write_native_text(joined, prev_len, &[u16::from(b' ')]);
    st.mem
        .copy_native_text_into(joined, prev_len + 1, tail_word);
    let metrics = use_glyph_metrics(st);
    st.fonts.measure_native_node(&mut st.mem, joined, metrics)?;
    let t = joined.width(&st.mem) - prev_word.width(&st.mem) - tail_word.width(&st.mem);
    joined.free_native(&mut st.mem);
    // TeX reads `width(font_glue[f])`, which stays null while `\spaceskip` supplies the glue.
    let space = st.fonts.param(mf, SPACE_CODE);
    if t != space {
        let kern = st.mem.new_kern(t - space)?;
        st.mem[kern].subtype = SPACE_ADJUSTMENT;
        let next = glue.link(&st.mem);
        st.mem[kern].link = next;
        glue.set_link(&mut st.mem, kern.ptr());
    }
    Ok(())
}

/// The top of the ligature stack: the lookahead character, or a lig item that
/// a ligature instruction pushed.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum LigStack {
    Char(NodePtr<CharNode>),
    Item(NodePtr<LigItem>),
}

impl LigStack {
    #[inline]
    pub(crate) fn ptr(self) -> MemPtr {
        match self {
            LigStack::Char(c) => c.ptr(),
            LigStack::Item(i) => i.ptr(),
        }
    }

    /// The character it carries.
    #[inline]
    pub(crate) fn character(self, mem: &MemState) -> LigChar {
        LigChar::of(match self {
            LigStack::Char(c) => c.character(mem),
            LigStack::Item(i) => mem[i].character,
        })
    }
}

impl MemState {
    /// The ligature stack entry at link `p`, `None` when the stack ends there.
    #[inline]
    pub(crate) fn lig_stack_at(&self, p: MemPtr) -> Option<LigStack> {
        if p.is_null() {
            None
        } else if self.is_char_node(p) {
            Some(LigStack::Char(NodePtr::at(p)))
        } else {
            Some(LigStack::Item(NodePtr::at(p)))
        }
    }
}

/// The link value of the ligature stack top, null when empty.
#[inline]
fn lig_stack_ptr(st: &EngineState) -> MemPtr {
    st.mainloop.lig_stack.map_or(MemPtr::NULL, LigStack::ptr)
}

/// The character on top of `lig_stack`, a char node or a lig item alike.
#[inline(always)]
fn lig_stack_char(st: &EngineState) -> LigChar {
    st.mainloop
        .lig_stack
        .map_or(LigChar::NONE, |top| top.character(&st.mem))
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
pub(super) enum Resume {
    /// `main_lig_loop`.
    LigLoop,
    /// `main_lig_loop+1` at lig/kern program index `k`.
    LigProgram(FontInfoIdx),
    /// `main_loop_move+2`.
    Move2,
}

/// TeX's `wrapup`: packs a pending ligature and inserts a pending
/// discretionary. `rt_hit` says whether a right boundary hit may count.
#[inline(always)]
fn wrapup(st: &mut EngineState, rt_hit: bool) -> Result<(), Abort> {
    if !st.mainloop.cur_l.is_char() {
        return Ok(());
    }
    let main_f = st.mainloop.main_f;
    if !st.mainloop.cur_q.link(&st.mem).is_null()
        && i32::from(NodePtr::<CharNode>::at(st.input.cur_list.tail).character(&st.mem))
            == st.fonts.hyphen_char[main_f]
    {
        st.mainloop.ins_disc = true;
    }
    if st.mainloop.ligature_present {
        let lig_list = st.mainloop.cur_q.link(&st.mem);
        let lig = st
            .mem
            .new_ligature(main_f, st.mainloop.cur_l.code(), lig_list)?;
        if st.mainloop.lft_hit {
            lig.set_subtype(&mut st.mem, 2);
            st.mainloop.lft_hit = false;
        }
        if rt_hit && st.mainloop.rt_hit && st.mainloop.lig_stack.is_none() {
            let s = lig.subtype(&st.mem);
            lig.set_subtype(&mut st.mem, s + 1);
            st.mainloop.rt_hit = false;
        }
        st.mainloop.cur_q.set_link(&mut st.mem, lig.ptr());
        st.input.cur_list.tail = lig.ptr();
        st.mainloop.ligature_present = false;
    }
    if st.mainloop.ins_disc {
        st.mainloop.ins_disc = false;
        if st.input.cur_list.mode > 0 {
            let disc = st.mem.new_disc()?;
            st.input.cur_list.tail_append(&mut st.mem, disc.ptr());
        }
    }
    Ok(())
}

#[inline(always)]
fn cur_l_info(st: &mut EngineState) -> CharInfo {
    let main_f = st.mainloop.main_f;
    let c = st.fonts.effective_char(main_f, st.mainloop.cur_l.code());
    st.fonts.char_info(main_f, c)
}

/// Returns true to go to `big_switch`, false to go to `reswitch`.
pub(super) fn main_control_main_loop(st: &mut EngineState) -> Result<bool, Abort> {
    if st.input.cur_list.head == st.input.cur_list.tail
        && st.input.cur_list.mode > 0
        && st.shipout.insert_src_special_auto
    {
        append_src_special(st)?;
    }
    st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;

    let mf = st.eqtb.cur_font();
    if st.fonts.is_native_font(mf) {
        return main_loop_native(st, mf);
    }

    let sc = adjust_space_factor(st);
    if insert_inter_char_toks_before(st, sc)? {
        return Ok(true);
    }
    note_char_class(st, sc);

    st.mainloop.main_f = mf;
    st.mainloop.bchar = st.fonts.font_bchar[mf];
    st.mainloop.false_bchar = st.fonts.font_false_bchar[mf];
    if st.input.cur_list.mode > 0 && st.eqtb.int_par(IntPar::Language) != st.input.cur_list.clang()
    {
        st.input.cur_list.fix_language(&mut st.mem, &st.eqtb)?;
    }
    st.mainloop.cur_l = LigChar::from_raw(st.scanner.cur_chr);
    let ls = st.mem.new_char(mf, st.mainloop.cur_l.code())?;
    st.mainloop.lig_stack = Some(LigStack::Char(ls));
    st.mainloop.cur_q = st.input.cur_list.tail;

    let bchar_label = if st.mainloop.cancel_boundary {
        st.mainloop.cancel_boundary = false;
        FontInfoIdx::NON_ADDRESS
    } else {
        st.fonts.bchar_label[mf]
    };
    let mut resume = if bchar_label != FontInfoIdx::NON_ADDRESS {
        st.mainloop.cur_r = st.mainloop.cur_l;
        st.mainloop.cur_l = LigChar::NONE;
        Resume::LigProgram(bchar_label)
    } else {
        Resume::Move2
    };
    let mut main_i = CharInfo::NULL;

    loop {
        let exit = match std::mem::replace(&mut resume, Resume::LigLoop) {
            Resume::Move2 => None,
            Resume::LigProgram(k) => {
                let step = st.fonts.lig_kern_step(k);
                Some(lig_kern_program(st, k, step, &mut main_i)?)
            }
            Resume::LigLoop
                if main_i.tag() != CharTag::Lig || st.mainloop.cur_r == LigChar::NONE =>
            {
                Some(LigExit::Wrapup)
            }
            Resume::LigLoop => {
                let (k, step) = st.fonts.lig_kern_start(st.mainloop.main_f, main_i);
                Some(lig_kern_program(st, k, step, &mut main_i)?)
            }
        };
        if let Some(exit) = exit {
            match exit {
                LigExit::Restart => continue,
                LigExit::Wrapup => wrapup(st, true)?,
                LigExit::Move | LigExit::Move1 => {}
            }
            if !matches!(exit, LigExit::Move1) {
                if st.mainloop.lig_stack.is_none() {
                    return Ok(false);
                }
                st.mainloop.cur_q = st.input.cur_list.tail;
                st.mainloop.cur_l = lig_stack_char(st);
            }

            if let Some(LigStack::Item(item)) = st.mainloop.lig_stack {
                let lig_ptr = st.mem[item].component;
                if lig_ptr.raw() > TEX_NULL {
                    st.input.cur_list.tail_append(&mut st.mem, lig_ptr);
                }
                st.mainloop.lig_stack = st.mem.lig_stack_at(st.mem[item].link);
                item.free(&mut st.mem);
                main_i = cur_l_info(st);
                st.mainloop.ligature_present = true;
                if st.mainloop.lig_stack.is_none() {
                    if lig_ptr.raw() > TEX_NULL {
                        if !lookahead(st)? {
                            return Ok(true);
                        }
                        continue;
                    }
                    st.mainloop.cur_r = st.mainloop.bchar;
                } else {
                    st.mainloop.cur_r = lig_stack_char(st);
                }
                continue;
            }
        }

        let main_f = st.mainloop.main_f;
        let cur_chr = st.scanner.cur_chr as u16;
        // Each call resets `xtx_ligature_present`, so the two bounds may see different codes.
        let exists = if st.fonts.effective_char(main_f, cur_chr)
            > i32::from(st.fonts.font_ec[main_f])
            || st.fonts.effective_char(main_f, cur_chr) < i32::from(st.fonts.font_bc[main_f])
        {
            false
        } else {
            main_i = st
                .fonts
                .effective_char_info(main_f, st.mainloop.cur_l.code());
            main_i.exists()
        };
        if !exists {
            crate::output::char_warning(st, main_f, st.scanner.cur_chr)?;
            st.mem.free_avail(lig_stack_ptr(st));
            return Ok(true);
        }
        let top = lig_stack_ptr(st);
        st.input.cur_list.tail_append(&mut st.mem, top);
        if !lookahead(st)? {
            return Ok(true);
        }
    }
}

/// Runs the font's ligature/kern program for `cur_l` and `cur_r` from
/// instruction `step` at index `k`.
#[inline(always)]
fn lig_kern_program(
    st: &mut EngineState,
    mut k: FontInfoIdx,
    mut step: LigKernStep,
    main_i: &mut CharInfo,
) -> Result<LigExit, Abort> {
    loop {
        if !step.matches(st.mainloop.cur_r) {
            if step.is_last() {
                return Ok(LigExit::Wrapup);
            }
            k = step.next_index(k);
            step = st.fonts.lig_kern_step(k);
            continue;
        }
        if step.is_kern() {
            wrapup(st, true)?;
            let main_f = st.mainloop.main_f;
            let kern = st.fonts.char_kern(main_f, step);
            let kern = st.mem.new_kern(kern)?;
            st.input.cur_list.tail_append(&mut st.mem, kern.ptr());
            return Ok(LigExit::Move);
        }
        if st.mainloop.cur_l == LigChar::NONE {
            st.mainloop.lft_hit = true;
        } else if st.mainloop.lig_stack.is_none() {
            st.mainloop.rt_hit = true;
        }
        let op = step.lig_op();
        match op {
            1 | 5 => {
                st.mainloop.cur_l = LigChar::of(step.lig_char());
                *main_i = cur_l_info(st);
                st.mainloop.ligature_present = true;
            }
            2 | 6 => {
                st.mainloop.cur_r = LigChar::of(step.lig_char());
                let c = st.mainloop.cur_r.code();
                match st.mainloop.lig_stack {
                    None => {
                        let item = st.mem.new_lig_item(c)?;
                        st.mainloop.lig_stack = Some(LigStack::Item(item));
                        st.mainloop.bchar = LigChar::NONE;
                    }
                    Some(LigStack::Char(prev)) => {
                        let item = st.mem.new_lig_item(c)?;
                        st.mainloop.lig_stack = Some(LigStack::Item(item));
                        st.mem[item].component = prev.ptr();
                    }
                    Some(LigStack::Item(item)) => st.mem[item].character = c,
                }
            }
            3 => {
                st.mainloop.cur_r = LigChar::of(step.lig_char());
                let prev = lig_stack_ptr(st);
                let item = st.mem.new_lig_item(st.mainloop.cur_r.code())?;
                st.mainloop.lig_stack = Some(LigStack::Item(item));
                item.ptr().set_link(&mut st.mem, prev);
            }
            7 | 11 => {
                wrapup(st, false)?;
                st.mainloop.cur_q = st.input.cur_list.tail;
                st.mainloop.cur_l = LigChar::of(step.lig_char());
                *main_i = cur_l_info(st);
                st.mainloop.ligature_present = true;
            }
            _ => {
                st.mainloop.cur_l = LigChar::of(step.lig_char());
                st.mainloop.ligature_present = true;
                return Ok(if st.mainloop.lig_stack.is_none() {
                    LigExit::Wrapup
                } else {
                    LigExit::Move1
                });
            }
        }
        if op > 4 && op != 7 {
            return Ok(LigExit::Wrapup);
        }
        if st.mainloop.cur_l.is_char() {
            return Ok(LigExit::Restart);
        }
        k = st.fonts.bchar_label[st.mainloop.main_f];
        step = st.fonts.lig_kern_step(k);
    }
}

/// TeX's `main_loop_lookahead`: fetches the next character into a new
/// `lig_stack` and `cur_r`. Returns false when inter-character tokens were
/// inserted instead.
#[inline(always)]
fn lookahead(st: &mut EngineState) -> Result<bool, Abort> {
    crate::scan::get_next(st)?;
    if !is_char_cmd(st.scanner.cur_cmd) {
        crate::scan::x_token(st)?;
    }
    if !is_char_cmd(st.scanner.cur_cmd) {
        if st.scanner.cur_cmd != Cmd::CharNum {
            if st.scanner.cur_cmd == Cmd::NoBoundary {
                st.mainloop.bchar = LigChar::NONE;
            }
            st.mainloop.cur_r = st.mainloop.bchar;
            st.mainloop.lig_stack = None;
            return Ok(true);
        }
        st.scanner.cur_chr = crate::scan::scan_char_num(st)?;
    }

    let sc = adjust_space_factor(st);
    let was_boundary = st.mainloop.prev_class == CHAR_CLASS_LIMIT - 1;
    if insert_inter_char_toks_before(st, sc)? {
        if !was_boundary {
            st.mainloop.prev_class = CHAR_CLASS_LIMIT - 1;
        }
        return Ok(false);
    }
    note_char_class(st, sc);

    let c = st.scanner.cur_chr;
    let ls = st.mem.new_char(st.mainloop.main_f, c as u16)?;
    st.mainloop.lig_stack = Some(LigStack::Char(ls));
    st.mainloop.cur_r = LigChar::from_raw(c);
    if st.mainloop.cur_r == st.mainloop.false_bchar {
        st.mainloop.cur_r = LigChar::NONE;
    }
    Ok(true)
}
