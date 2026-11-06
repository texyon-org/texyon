//! Engine init, run and cleanup: the `run_engine` entry point, the
//! initialize_* primers, primitive registration and teardown.

use crate::bridge::output_open_stdout;
use crate::cmd::Cmd;
use crate::codes::{BoxCode, FiOrElseCode, MarkCode, ScannerStatus, ValueLevel};
use crate::consts::{
    ACTIVE_BASE, BIGGEST_CHAR, BIGGEST_LANG, BOX_BASE, CAT_CODE_BASE, CLOSED, CS_TOKEN_FLAG,
    CUR_FONT_LOC, DEL_CODE_BASE, DIMEN_BASE, EMPTY, EMPTY_STRING, END_WRITE, EQTB_SIZE,
    ETEX_PEN_BASE, FONT_BASE, FROZEN_CONTROL_SEQUENCE, FROZEN_DONT_EXPAND, FROZEN_ENDV,
    FROZEN_END_TEMPLATE, FROZEN_PRIMITIVE, FROZEN_PROTECTION, GLUE_BASE, HASH_BASE,
    HISTORY_FATAL_ERROR, HISTORY_SPOTLESS, HISTORY_WARNING_ISSUED, IGNORE_DEPTH, INT_BASE,
    LC_CODE_BASE, LEFT_TO_RIGHT, LEVEL_ONE, LEVEL_ZERO, LOCAL_BASE, MATH_CODE_BASE, MATH_FONT_BASE,
    MAX_FONT_MAX, MAX_HALFWORD, MEM_TOP, MIN_TRIE_OP, NORMAL, NULL_FONT, NUMBER_MATH_FAMILIES,
    NUMBER_USVS, PAR_SHAPE_LOC, PRIM_SIZE, SF_CODE_BASE, SINGLE_BASE, TEX_NULL, TOKS_BASE,
    TOO_BIG_CHAR, UC_CODE_BASE, UNDEFINED_CONTROL_SEQUENCE, VMODE,
};
use crate::errors::Abort;
use crate::format_io::load_fmt_file;
use crate::hash::{HASH_OFFSET, HASH_PRIME, HASH_SIZE};
use crate::helpers::OnDrop;
use crate::hyphenation::{HYPH_PRIME, TRIE_OP_SIZE};
use crate::input::{NEW_LINE, TOKEN_LIST};
use crate::io::init_io;
use crate::linebreak::HYPHENATED;
use crate::mem::{
    ACTIVE_LIST, CONTRIB_HEAD, END_SPAN, FILL_GLUE, FIL_GLUE, FIL_NEG_GLUE, HI_MEM_STAT_MIN,
    HI_MEM_STAT_USAGE, NULL_LIST, OMIT_TEMPLATE, PAGE_HEAD, PAGE_INS_HEAD, SS_GLUE, ZERO_GLUE,
};
use crate::mem_types::{zeroed_vec, B32x2, MemoryWord};
use crate::node::{ActiveNode, CondNode, FreeBlock, NodePtr, PageInsNode};
use crate::packaging::MarksAction;
use crate::params::{IntPar, TokPar};
use crate::scan::{set_class, set_family};
use crate::stack::TexStack;
use crate::state::{
    ConfigState, CurMarks, EngineState, EqtbState, FontState, GroupCode, InteractionMode, MemState,
    PerFont, Selector,
};
use crate::token::Token;
use crate::units::{
    BufPos, EqtbPtr, FontId, FontInfoIdx, GlueOrder, MemPtr, NodeType, PoolPos, PrimNum, SavePtr,
    Scaled, StrNumber,
};

/// A `get_node` request this large merges every free block into the rover list.
const GET_NODE_HEAD_SENTINEL: i32 = 0x4000_0000;

pub(crate) const POOL_FREE: i32 = 47_500;
pub(crate) const STRINGS_FREE: i32 = 100;
const HYPH_SIZE: i32 = 8_191;

const NUM_ETEX_PENALTIES: i32 = 4;
const NUMBER_REGS: i32 = 256;
const NUMBER_MATH_FONTS: i32 = 3 * NUMBER_MATH_FAMILIES;
const TOO_BIG_LANG: i32 = 256;
const VAR_FAM_CLASS: i32 = 7;
const NEST_SIZE: usize = 500;
const PARAM_SIZE: usize = 10_000;
const STACK_SIZE: usize = 5_000;

const _: () = assert!(
    MEM_TOP >= 1100
        && HASH_PRIME <= HASH_SIZE
        && TEX_NULL <= 0
        && MAX_FONT_MAX >= TEX_NULL
        && MAX_FONT_MAX <= MAX_HALFWORD
        && HASH_OFFSET >= 0
        && HASH_OFFSET <= HASH_BASE.raw()
        && 2 * MAX_HALFWORD >= MEM_TOP
);

// These touch only process-global C state and take no pointers.
unsafe extern "C" {
    safe fn pdf_files_init();
    safe fn pdf_files_close_if_open();
    safe fn pdf_files_close();
}

pub(crate) fn output_file_extension(st: &EngineState) -> &'static str {
    if st.config.semantic_pagination_enabled {
        ".spx"
    } else {
        ".xdv"
    }
}

pub(crate) fn get_strings_started(st: &mut EngineState) -> Result<(), Abort> {
    st.strings.pool_ptr = PoolPos::ZERO;
    st.strings.str_start[0] = PoolPos::ZERO;
    st.strings.str_ptr = StrNumber::from_raw(TOO_BIG_CHAR);

    let spare_size = st.strings.pool_size.raw() - st.strings.string_vacancies;
    let loaded = st.strings.load_pool_strings(spare_size)?;
    if loaded == StrNumber::NONE {
        return Err(Abort::Internal("must increase pool_size"));
    }
    Ok(())
}

impl MemState {
    /// TeX's `sort_avail`: orders the variable-size free list by address.
    pub(crate) fn sort_avail(&mut self) {
        let block = NodePtr::<FreeBlock>::at;
        let end = MemPtr::from_raw(MAX_HALFWORD);
        let _ = self.get_node(GET_NODE_HEAD_SENTINEL);
        let old_rover = self.rover;
        let mut p = block(old_rover).rlink(self);
        block(old_rover).set_rlink(self, end);

        while p != old_rover {
            let rover = self.rover;
            if p < rover {
                let q = p;
                p = block(q).rlink(self);
                block(q).set_rlink(self, rover);
                self.rover = q;
            } else {
                let mut q = rover;
                while block(q).rlink(self) < p {
                    q = block(q).rlink(self);
                }
                let r = block(p).rlink(self);
                let after_q = block(q).rlink(self);
                block(p).set_rlink(self, after_q);
                block(q).set_rlink(self, p);
                p = r;
            }
        }

        let rover = self.rover;
        p = rover;
        while block(p).rlink(self) != end {
            let next = block(p).rlink(self);
            block(next).set_llink(self, p);
            p = next;
        }

        block(p).set_rlink(self, rover);
        block(rover).set_llink(self, p);
    }
}

impl EqtbState {
    /// Sets `eqtb[from..to]` to `w`.
    pub(crate) fn fill(&mut self, from: EqtbPtr, to: EqtbPtr, w: crate::mem_types::MemoryWord) {
        self.data[from.index()..to.index()].fill(w);
    }

    pub(crate) fn set_eq(&mut self, p: EqtbPtr, eq_type: Cmd, equiv: i32, level: u16) {
        self.set_eq_type(p, eq_type);
        self.set_equiv(p, equiv);
        self.set_eq_level(p, level);
    }
}

/// Defines primitive `ident` and returns its control sequence.
fn primitive(st: &mut EngineState, ident: &str, c: Cmd, o: i32) -> Result<EqtbPtr, Abort> {
    let bytes = ident.as_bytes();
    let cs;
    let prim_val = if bytes.len() > 1 {
        let len = bytes.len() as i32;
        let s = st.strings.make_tex_string(ident)?;

        if st.input.first + len > st.input.buf_size + 1 {
            return Err(st.input.buffer_overflow());
        }
        let first = st.input.first;
        for (slot, &b) in st.input.buffer.0[first.index()..].iter_mut().zip(bytes) {
            *slot = i32::from(b);
        }

        cs = st
            .eqtb
            .id_lookup(&mut st.strings, st.input.buffer.span(first, first + len))?;
        st.strings.flush_string();
        st.eqtb.set_text(cs, s);
        st.eqtb.prim_lookup(&mut st.strings, s)?
    } else {
        let ch = i32::from(bytes[0]);
        cs = SINGLE_BASE + ch;
        st.eqtb
            .prim_lookup(&mut st.strings, StrNumber::from_raw(ch))?
    };

    st.eqtb.set_eq(cs, c, o, LEVEL_ONE);
    st.eqtb.set_eq(prim_val.eqtb_loc(), c, o, LEVEL_ONE);
    Ok(cs)
}

pub(crate) fn final_cleanup(st: &mut EngineState) -> Result<(), Abort> {
    let c = st.scanner.cur_chr;

    if c != 1 {
        st.eqtb.set_int_par(IntPar::NewLineChar, -1);
    }

    if st.strings.job_name == StrNumber::NONE {
        crate::io::open_log_file(st)?;
    }

    while !st.input.input_stack.is_empty() {
        if st.input.cur_input.state == TOKEN_LIST {
            crate::input::end_token_list(st)?;
        } else {
            crate::input::end_file_reading(st)?;
        }
    }

    while st.input.open_parens > 0 {
        crate::output::print(st, " )");
        st.input.open_parens -= 1;
    }

    let cur_level = st.save_stack.cur_level;
    if cur_level > LEVEL_ONE {
        crate::output::print_nl(st, b'(');
        crate::output::print_esc(st, "end occurred ");
        crate::output::print(st, "inside a group at level ");
        crate::output::print_int(st, i32::from(cur_level) - 1);
        crate::output::print_char(st, b')');
        crate::output::show_save_groups(st);
    }

    while st.scanner.cond_ptr.raw() != TEX_NULL {
        crate::output::print_nl(st, b'(');
        crate::output::print_esc(st, "end occurred ");
        crate::output::print(st, "when ");
        crate::output::print_cmd_chr(st, Cmd::IfTest, i32::from(st.scanner.cur_if));
        let if_line = st.scanner.if_line;
        if if_line != 0 {
            crate::output::print(st, " on line ");
            crate::output::print_int(st, if_line);
        }
        crate::output::print(st, " was incomplete)");
        let cond = NodePtr::<CondNode>::at(st.scanner.cond_ptr);
        st.scanner.if_line = cond.if_line(&st.mem);
        st.scanner.cur_if = cond.cur_if(&st.mem) as i16;
        st.mem.temp_ptr = cond.ptr();
        st.scanner.cond_ptr = cond.link(&st.mem);
        cond.free(&mut st.mem);
    }

    if st.errors.history != HISTORY_SPOTLESS
        && (st.errors.history == HISTORY_WARNING_ISSUED
            || st.errors.interaction < InteractionMode::ErrorStop)
        && st.shipout.selector == Selector::TermAndLog
    {
        st.shipout.selector = Selector::TermOnly;
        crate::output::print_nl(st, "(see the transcript file for additional information)");
        st.shipout.selector = Selector::TermAndLog;
    }

    if c != 1 {
        return Ok(());
    }
    if !st.config.in_initex_mode {
        crate::output::print_nl(st, "(\\dump is performed only by INITEX)");
        return Ok(());
    }

    for mark_code in MarkCode::ALL {
        let mark = st.scanner.cur_mark[mark_code];
        if !mark.is_null() {
            st.mem.delete_token_ref(mark);
        }
    }

    if let Some(marks) = st.scanner.sa_root[ValueLevel::Mark].as_idx() {
        if st
            .mem
            .do_marks(&mut st.scanner, MarksAction::DestroyMarks, 0, marks)
        {
            st.scanner.sa_root[ValueLevel::Mark] = MemPtr::NULL;
        }
    }

    for box_code in BoxCode::LastBox as usize..=BoxCode::Vsplit as usize {
        st.mem.flush_node_list(st.scanner.disc_ptr[box_code])?;
    }

    if let Some(spec) = st.shipout.last_glue {
        st.mem.delete_glue_ref(spec);
    }

    crate::format_io::store_fmt_file(st)
}

fn initialize_more_variables(st: &mut EngineState) {
    st.shipout.doing_special = false;
    st.scanner.native_text_size = 128;
    st.scanner.native_text = vec![0; 128];

    st.errors.interaction = InteractionMode::ErrorStop;

    st.errors.deletions_allowed = true;
    st.scanner.set_box_allowed = true;
    st.errors.error_count = 0;
    st.errors.help(&[]);
    st.errors.use_err_help = false;

    st.errors.two_to_the[0] = 1;
    for k in 1..=30usize {
        st.errors.two_to_the[k] = 2 * st.errors.two_to_the[k - 1];
    }

    st.errors.spec_log[1..=13].copy_from_slice(&[
        93_032_640, 38_612_034, 17_922_280, 8_662_214, 4_261_238, 2_113_709, 1_052_693, 525_315,
        262_400, 131_136, 65_552, 32_772, 16_385,
    ]);
    for k in 14..=27usize {
        st.errors.spec_log[k] = st.errors.two_to_the[27 - k];
    }
    st.errors.spec_log[28] = 1;

    st.input.nest.reset();
    st.input.cur_list.mode = VMODE;
    st.input.cur_list.head = CONTRIB_HEAD;
    st.input.cur_list.tail = CONTRIB_HEAD;
    st.input.cur_list.e_tex_aux = MemPtr::NULL;
    st.input.cur_list.set_prev_depth(IGNORE_DEPTH);
    st.input.cur_list.mode_line = 0;
    st.input.cur_list.prev_graf = 0;
    st.scanner.shown_mode = 0;
    st.shipout.page_contents = crate::state::PageContents::Empty;
    st.shipout.page_tail = PAGE_HEAD;
    st.shipout.last_glue = None;
    st.shipout.last_penalty = 0;
    st.shipout.last_kern = Scaled::ZERO;
    st.shipout.last_node_type = -1;
    st.shipout.page_so_far[7] = Scaled::ZERO;

    for k in INT_BASE.through(EQTB_SIZE) {
        st.eqtb.set_xeq_level_at(k, LEVEL_ONE);
    }

    st.eqtb.no_new_control_sequence = true;
    for k in PrimNum::all() {
        st.eqtb.set_prim_next(k, PrimNum::UNDEFINED);
        st.eqtb.set_prim_text(k, None);
    }

    st.save_stack.save_ptr = SavePtr::ZERO;
    st.save_stack.cur_level = LEVEL_ONE;
    st.save_stack.cur_group = GroupCode::BottomLevel;
    st.save_stack.cur_boundary = SavePtr::ZERO;
    st.save_stack.max_save_stack = SavePtr::ZERO;
    st.scanner.mag_set = 0;
    st.scanner.expand_depth_count = 0;
    st.scanner.is_in_csname = false;
    st.scanner.cur_mark = CurMarks::NULL;
    st.scanner.cur_val = 0;
    st.scanner.cur_val_level = ValueLevel::Int;
    st.scanner.radix = 0;
    st.scanner.cur_order = GlueOrder::Normal;

    st.input.read_open = [CLOSED; 17];

    st.scanner.cond_ptr = MemPtr::NULL;
    st.scanner.if_limit = FiOrElseCode::Normal;
    st.scanner.cur_if = 0;
    st.scanner.if_line = 0;
    st.shipout.total_pages = 0;
    st.shipout.max_v = Scaled::ZERO;
    st.shipout.max_h = Scaled::ZERO;
    st.shipout.dvi.max_push = 0;
    st.shipout.dvi.last_bop = -1;
    st.shipout.doing_leaders = false;
    st.shipout.dead_cycles = 0;
    st.scanner.adjust_tail = MemPtr::NULL;
    st.shipout.last_badness = 0;
    st.scanner.pre_adjust_tail = MemPtr::NULL;
    st.scanner.pack_begin_line = 0;
    st.scanner.empty.s1 = EMPTY;
    st.scanner.empty.s0 = TEX_NULL;
    st.scanner.align_ptr = MemPtr::NULL;
    st.scanner.cur_align = MemPtr::NULL;
    st.scanner.cur_span = MemPtr::NULL;
    st.scanner.cur_loop = MemPtr::NULL;
    st.scanner.cur_head = MemPtr::NULL;
    st.scanner.cur_tail = MemPtr::NULL;
    st.scanner.cur_pre_head = MemPtr::NULL;
    st.scanner.cur_pre_tail = MemPtr::NULL;
    st.scanner.cur_f = NULL_FONT;
    st.hyph.max_hyph_char = TOO_BIG_LANG;

    let hyph_len = st.hyph.hyph_size as usize + 1;
    st.hyph.hyph_word[..hyph_len].fill(StrNumber::NONE);
    st.hyph.hyph_list[..hyph_len].fill(MemPtr::NULL);
    st.hyph.hyph_link[..hyph_len].fill(0);

    st.hyph.hyph_count = 0;
    st.hyph.hyph_next = HYPH_PRIME + 1;
    if st.hyph.hyph_next > st.hyph.hyph_size {
        st.hyph.hyph_next = HYPH_PRIME;
    }

    st.shipout.output_active = false;
    st.shipout.insert_penalties = 0;
    st.mainloop.ligature_present = false;
    st.mainloop.cancel_boundary = false;
    st.mainloop.lft_hit = false;
    st.mainloop.rt_hit = false;
    st.mainloop.ins_disc = false;
    st.scanner.after_token = Token::ZERO;
    st.errors.long_help_seen = false;
    st.strings.format_ident = StrNumber::NONE;

    st.shipout.write_open = [false; 18];

    st.scanner.lr_ptr = MemPtr::NULL;
    st.scanner.lr_problems = 0;
    st.scanner.cur_dir = LEFT_TO_RIGHT;
    st.input.pseudo_files = MemPtr::NULL;
    st.scanner.sa_root[ValueLevel::Mark] = MemPtr::NULL;
    st.scanner.sa_null = MemoryWord {
        b32: B32x2 {
            s0: TEX_NULL,
            s1: TEX_NULL,
        },
    };
    st.scanner.sa_chain = MemPtr::NULL;
    st.scanner.sa_level = LEVEL_ZERO;
    st.scanner.disc_ptr[BoxCode::LastBox as usize] = MemPtr::NULL;
    st.scanner.disc_ptr[BoxCode::Vsplit as usize] = MemPtr::NULL;
    st.scanner.stop_at_space = true;
}

fn initialize_more_initex_variables(st: &mut EngineState, format_ident: StrNumber) {
    let mem = &mut st.mem;
    for spec in [ZERO_GLUE, FIL_GLUE, FILL_GLUE, SS_GLUE, FIL_NEG_GLUE] {
        spec.set_width(mem, Scaled::ZERO);
        spec.set_stretch(mem, Scaled::ZERO);
        spec.set_shrink(mem, Scaled::ZERO);
        spec.set_ref_count(mem, TEX_NULL + 1);
        spec.set_stretch_order(mem, GlueOrder::Normal);
        spec.set_shrink_order(mem, GlueOrder::Normal);
    }

    FIL_GLUE.set_stretch(mem, Scaled::UNITY);
    FIL_GLUE.set_stretch_order(mem, GlueOrder::Fil);
    FILL_GLUE.set_stretch(mem, Scaled::UNITY);
    FILL_GLUE.set_stretch_order(mem, GlueOrder::Fill);
    SS_GLUE.set_stretch(mem, Scaled::UNITY);
    SS_GLUE.set_stretch_order(mem, GlueOrder::Fil);
    SS_GLUE.set_shrink(mem, Scaled::UNITY);
    SS_GLUE.set_shrink_order(mem, GlueOrder::Fil);
    FIL_NEG_GLUE.set_stretch(mem, -Scaled::UNITY);
    FIL_NEG_GLUE.set_stretch_order(mem, GlueOrder::Fil);

    mem.rover = MemPtr::from_raw(20);
    let rover = NodePtr::<FreeBlock>::at(mem.rover);
    rover.mark_empty(mem);
    rover.set_node_size(mem, 1000);
    rover.set_llink(mem, mem.rover);
    rover.set_rlink(mem, mem.rover);
    mem.lo_mem_max = mem.rover + 1000;
    let end = mem.lo_mem_max;
    end.set_link(mem, MemPtr::NULL);
    end.set_info(mem, TEX_NULL);

    let template = mem.word(end);
    for k in HI_MEM_STAT_MIN..=MEM_TOP {
        mem.set_word(MemPtr::from_raw(k), template);
    }

    OMIT_TEMPLATE.set_token(mem, Token::from_cs(FROZEN_END_TEMPLATE));
    END_SPAN.set_span_count(mem, i32::from(u16::MAX) + 1);
    mem.set_b32s0(END_SPAN.idx(), TEX_NULL);
    let active = NodePtr::<ActiveNode>::at(ACTIVE_LIST);
    active.set_break_type(mem, HYPHENATED as u16);
    active.set_line_number(mem, MAX_HALFWORD);
    active.set_fitness(mem, 0);
    let page_ins = NodePtr::<PageInsNode>::at(PAGE_INS_HEAD);
    page_ins.set_subtype(mem, 255);
    page_ins.set_split_up(mem, true);
    page_ins.set_link(mem, PAGE_INS_HEAD);
    mem.set_b16s1(PAGE_HEAD, NodeType::Glue.as_raw());
    mem.set_b16s0(PAGE_HEAD, NORMAL);
    mem.avail = MemPtr::NULL;
    mem.mem_end = MemPtr::from_raw(MEM_TOP);
    mem.hi_mem_min = MemPtr::from_raw(HI_MEM_STAT_MIN);
    mem.var_used = 20;
    mem.dyn_used = HI_MEM_STAT_USAGE;

    let eqtb = &mut st.eqtb;
    eqtb.set_eq(
        UNDEFINED_CONTROL_SEQUENCE,
        Cmd::UndefinedCs,
        TEX_NULL,
        LEVEL_ZERO,
    );
    let undefined_entry = eqtb.word(UNDEFINED_CONTROL_SEQUENCE);
    eqtb.fill(ACTIVE_BASE, eqtb.eqtb_top + 1, undefined_entry);

    eqtb.set_eq(GLUE_BASE, Cmd::GlueRef, 0, LEVEL_ONE);
    let glue_base_entry = eqtb.word(GLUE_BASE);
    eqtb.fill(GLUE_BASE + 1, LOCAL_BASE, glue_base_entry);

    let zero_glue_refs = ZERO_GLUE.ref_count(&st.mem) + 531;
    ZERO_GLUE.set_ref_count(&mut st.mem, zero_glue_refs);

    eqtb.set_eq(PAR_SHAPE_LOC, Cmd::ShapeRef, TEX_NULL, LEVEL_ONE);
    let par_shape_entry = eqtb.word(PAR_SHAPE_LOC);
    eqtb.fill(
        ETEX_PEN_BASE,
        ETEX_PEN_BASE + NUM_ETEX_PENALTIES,
        par_shape_entry,
    );

    let toks_end = TOKS_BASE + NUMBER_REGS;
    eqtb.fill(TokPar::OutputRoutine.loc(), toks_end, undefined_entry);

    eqtb.set_eq(BOX_BASE, Cmd::BoxRef, TEX_NULL, LEVEL_ONE);
    let box_base_entry = eqtb.word(BOX_BASE);
    eqtb.fill(BOX_BASE + 1, BOX_BASE + NUMBER_REGS, box_base_entry);

    eqtb.set_eq(CUR_FONT_LOC, Cmd::Data, FONT_BASE, LEVEL_ONE);
    let cur_font_entry = eqtb.word(CUR_FONT_LOC);
    eqtb.fill(
        MATH_FONT_BASE,
        MATH_FONT_BASE + NUMBER_MATH_FONTS,
        cur_font_entry,
    );

    eqtb.set_eq(CAT_CODE_BASE, Cmd::Data, 0, LEVEL_ONE);
    let cat_code_entry = eqtb.word(CAT_CODE_BASE);
    eqtb.fill(CAT_CODE_BASE + 1, INT_BASE, cat_code_entry);

    for k in 0..NUMBER_USVS {
        eqtb.set_equiv(CAT_CODE_BASE + k, Cmd::OtherChar.into());
        eqtb.set_equiv(MATH_CODE_BASE + k, k);
        eqtb.set_equiv(SF_CODE_BASE + k, 1000);
    }

    eqtb.set_equiv(CAT_CODE_BASE + 13, Cmd::CarRet.into());
    eqtb.set_equiv(CAT_CODE_BASE + 32, Cmd::Spacer.into());
    eqtb.set_equiv(CAT_CODE_BASE + 92, Cmd::ESCAPE.into());
    eqtb.set_equiv(CAT_CODE_BASE + 37, Cmd::COMMENT.into());
    eqtb.set_equiv(CAT_CODE_BASE + 127, Cmd::INVALID_CHAR.into());
    eqtb.set_equiv(CAT_CODE_BASE, Cmd::IGNORE.into());

    for k in (i32::from(b'0'))..=(i32::from(b'9')) {
        eqtb.set_equiv(MATH_CODE_BASE + k, k + set_class(VAR_FAM_CLASS));
    }

    for k in (i32::from(b'A'))..=(i32::from(b'Z')) {
        let lower = k + 32;
        eqtb.set_equiv(CAT_CODE_BASE + k, Cmd::Letter.into());
        eqtb.set_equiv(CAT_CODE_BASE + lower, Cmd::Letter.into());
        eqtb.set_equiv(
            MATH_CODE_BASE + k,
            k + set_family(1) + set_class(VAR_FAM_CLASS),
        );
        eqtb.set_equiv(
            MATH_CODE_BASE + lower,
            lower + set_family(1) + set_class(VAR_FAM_CLASS),
        );
        eqtb.set_equiv(LC_CODE_BASE + k, lower);
        eqtb.set_equiv(LC_CODE_BASE + lower, lower);
        eqtb.set_equiv(UC_CODE_BASE + k, k);
        eqtb.set_equiv(UC_CODE_BASE + lower, k);
        eqtb.set_equiv(SF_CODE_BASE + k, 999);
    }

    for k in INT_BASE.until(DEL_CODE_BASE) {
        eqtb.set_int_at(k, 0);
    }

    eqtb.set_int_par(IntPar::Mag, 1000);
    eqtb.set_int_par(IntPar::Tolerance, 10000);
    eqtb.set_int_par(IntPar::HangAfter, 1);
    eqtb.set_int_par(IntPar::MaxDeadCycles, 25);
    eqtb.set_int_par(IntPar::EscapeChar, i32::from(b'\\'));
    eqtb.set_int_par(IntPar::EndLineChar, i32::from(b'\r'));

    for k in 0..NUMBER_USVS {
        eqtb.set_int_at(DEL_CODE_BASE + k, -1);
    }
    eqtb.set_int_at(DEL_CODE_BASE + i32::from(b'.'), 0);

    for k in DIMEN_BASE.through(EQTB_SIZE) {
        eqtb.set_int_at(k, 0);
    }

    eqtb.prim_used = PrimNum::from_raw(PRIM_SIZE);
    eqtb.hash_used = FROZEN_CONTROL_SEQUENCE;
    eqtb.hash_high = 0;
    eqtb.cs_count = 0;

    let hyph = &mut st.hyph;
    hyph.trie_op_hash_array[..=(2 * TRIE_OP_SIZE) as usize].fill(0);
    hyph.trie_used[..=BIGGEST_LANG as usize].fill(MIN_TRIE_OP);
    hyph.max_op_used = MIN_TRIE_OP;
    hyph.trie_op_ptr = 0;
    hyph.trie_not_ready = true;

    st.strings.format_ident = format_ident;

    st.scanner.max_reg_num = 32767;
    st.scanner.sa_root.registers_mut().fill(MemPtr::NULL);

    st.eqtb.set_int_par(IntPar::XetexHyphenatableLength, 63);
}

fn set_frozen_text(st: &mut EngineState, p: EqtbPtr, text: &str) -> Result<(), Abort> {
    let s = st.strings.make_tex_string(text)?;
    st.eqtb.set_text(p, s);
    Ok(())
}

fn initialize_primitives(st: &mut EngineState) -> Result<(), Abort> {
    use crate::primitives::PrimInit;

    st.eqtb.no_new_control_sequence = false;
    st.input.first = BufPos::ZERO;

    for prim in crate::primitives::PRIMITIVES {
        let cs = primitive(st, prim.name, prim.cmd, prim.chr)?;
        match prim.init {
            PrimInit::None => {}
            PrimInit::Par => {
                st.input.par_loc = cs;
                st.input.par_token = Token::from_cs(st.input.par_loc);
            }
            PrimInit::Write => {
                st.shipout.write_loc = cs;
            }
            PrimInit::Frozen(frozen) => {
                set_frozen_text(st, frozen, prim.name)?;
                st.eqtb.data[frozen.index()] = st.eqtb.word(cs);
            }
        }
    }

    set_frozen_text(st, FROZEN_END_TEMPLATE, "endtemplate")?;
    st.eqtb.set_eq(
        FROZEN_END_TEMPLATE,
        Cmd::EndTemplate,
        NULL_LIST.raw(),
        LEVEL_ONE,
    );

    set_frozen_text(st, FROZEN_ENDV, "endtemplate")?;
    st.eqtb
        .set_eq(FROZEN_ENDV, Cmd::Endv, NULL_LIST.raw(), LEVEL_ONE);

    set_frozen_text(st, FROZEN_DONT_EXPAND, "notexpanded:")?;
    st.eqtb.set_eq_type(FROZEN_DONT_EXPAND, Cmd::DontExpand);

    set_frozen_text(st, FROZEN_PRIMITIVE, "primitive")?;
    st.eqtb
        .set_eq(FROZEN_PRIMITIVE, Cmd::IgnoreSpaces, 1, LEVEL_ONE);

    set_frozen_text(st, FROZEN_PROTECTION, "inaccessible")?;

    set_frozen_text(st, END_WRITE, "endwrite")?;
    st.eqtb
        .set_eq(END_WRITE, Cmd::OuterCall, TEX_NULL, LEVEL_ONE);

    st.eqtb.no_new_control_sequence = true;
    Ok(())
}

/// Batch entry point.
///
/// A Rust panic is caught and turned into a `HISTORY_FATAL_ERROR` return with
/// a surfaced message instead of unwinding into the host.
pub(crate) fn run_engine(
    api: &mut texyon_bridge_core::CoreBridgeState,
    config: ConfigState,
    dump_name: &str,
    input_file_name: &str,
    build_date: libc::time_t,
) -> i32 {
    let _bridge = crate::bridge::enter(api);

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_engine_inner(config, dump_name, input_file_name, build_date)
    }));
    match outcome {
        Ok(rv) => rv,
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("internal engine panic");
            crate::bridge::set_error_message(msg);
            HISTORY_FATAL_ERROR
        }
    }
}

fn run_engine_inner(
    config: ConfigState,
    dump_name: &str,
    input_file_name: &str,
    build_date: libc::time_t,
) -> i32 {
    // An abort that skips terminal cleanup must not leak this process-global
    // registry into the next batch run.
    let _pdf_files_guard = OnDrop(|| pdf_files_close_if_open());

    let mut st = Box::new(EngineState::default());
    st.config = config;

    match initialize_engine(&mut st, dump_name, build_date) {
        InitOutcome::Ready => run_document_to_end(&mut st, input_file_name),
        InitOutcome::Terminate(history) => history,
    }
}

/// Result of initializing an engine before any primary document is opened.
pub(crate) enum InitOutcome {
    Ready,
    Terminate(i32),
}

impl FontState {
    pub(crate) fn alloc_font_arrays(&mut self) {
        let len = self.font_max.index() + 1;
        self.font_layout_engine = PerFont::filled(len, None);
        self.font_mapping = PerFont::filled(len, None);
        self.font_flags = PerFont::filled(len, 0);
        self.font_letter_space = PerFont::filled(len, Scaled::ZERO);
        self.font_check = PerFont::filled(len, [0; 4]);
        self.font_size = PerFont::filled(len, Scaled::ZERO);
        self.font_dsize = PerFont::filled(len, Scaled::ZERO);
        self.font_params = PerFont::filled(len, 0);
        self.font_name = PerFont::filled(len, StrNumber::NONE);
        self.font_area = PerFont::filled(len, StrNumber::NONE);
        self.font_bc = PerFont::filled(len, 0);
        self.font_ec = PerFont::filled(len, 0);
        self.bchar_label = PerFont::filled(len, FontInfoIdx::ZERO);
        self.font_bchar = PerFont::filled(len, 0);
        self.font_false_bchar = PerFont::filled(len, 0);
        self.font_glue = PerFont::filled(len, MemPtr::default());
        self.hyphen_char = PerFont::filled(len, 0);
        self.skew_char = PerFont::filled(len, 0);
        self.char_base = PerFont::filled(len, FontInfoIdx::ZERO);
        self.width_base = PerFont::filled(len, FontInfoIdx::ZERO);
        self.height_base = PerFont::filled(len, FontInfoIdx::ZERO);
        self.depth_base = PerFont::filled(len, FontInfoIdx::ZERO);
        self.italic_base = PerFont::filled(len, FontInfoIdx::ZERO);
        self.lig_kern_base = PerFont::filled(len, FontInfoIdx::ZERO);
        self.kern_base = PerFont::filled(len, FontInfoIdx::ZERO);
        self.exten_base = PerFont::filled(len, FontInfoIdx::ZERO);
        self.param_base = PerFont::filled(len, FontInfoIdx::ZERO);
    }
}

/// Allocate engine storage, load the format, and initialize configuration.
///
/// On success the state is ready to open a primary input but owns no document
/// input stack, so batch and resident sessions share this path.
pub(crate) fn initialize_engine(
    st: &mut EngineState,
    dump_name: &str,
    build_date: libc::time_t,
) -> InitOutcome {
    st.shipout.rust_stdout = output_open_stdout();

    if st.config.file_line_error_style_p < 0 {
        st.config.file_line_error_style_p = 0;
    }

    st.strings.pool_size = PoolPos::from_raw(6_250_000);
    st.strings.string_vacancies = 90_000;
    st.strings.max_strings = StrNumber::from_raw(565_536);
    st.fonts.font_mem_size = FontInfoIdx::from_raw(8_000_000);
    st.fonts.font_max = FontId::from_raw(9_000);
    st.hyph.trie_size = 1_000_000;
    st.hyph.hyph_size = HYPH_SIZE;
    st.input.buf_size = BufPos::from_raw(200_000);

    st.input.max_in_open = 15;

    st.save_stack.save_size = 80_000;

    st.errors.error_line = 79;
    st.errors.half_error_line = 50;
    st.config.max_print_line = 79;
    st.scanner.expand_depth = 10_000;
    st.eqtb.hash_extra = 600_000;

    let in_open_words = st.input.max_in_open + 1;
    let hyph_words = HYPH_SIZE as usize + 1;
    st.input.buffer = crate::state::InputBuffer(vec![0; st.input.buf_size.index() + 1]);
    st.input.nest = TexStack::with_size(NEST_SIZE);
    st.save_stack.data = zeroed_vec(st.save_stack.save_size + 1);
    st.input.input_file = std::iter::repeat_with(|| crate::state::InputSlot::Empty)
        .take(in_open_words)
        .collect();
    st.input.input_stack = TexStack::with_size(STACK_SIZE);
    st.input.open_levels = vec![crate::state::OpenLevel::default(); in_open_words];
    st.input.param_stack = TexStack::with_size(PARAM_SIZE);
    st.hyph.hyph_word = vec![StrNumber::NONE; hyph_words];
    st.hyph.hyph_list = vec![MemPtr::NULL; hyph_words];
    st.hyph.hyph_link = vec![0; hyph_words];

    if st.config.in_initex_mode {
        st.mem.data = zeroed_vec(MEM_TOP as usize + 2);

        st.eqtb.eqtb_top = EQTB_SIZE + st.eqtb.hash_extra;
        st.eqtb.hash_top = if st.eqtb.hash_extra == 0 {
            UNDEFINED_CONTROL_SEQUENCE
        } else {
            st.eqtb.eqtb_top
        };
        st.eqtb.hash = zeroed_vec((1 + st.eqtb.hash_top.raw() - HASH_OFFSET) as usize);
        st.eqtb.hash_used = st.eqtb.hash_top;

        st.eqtb.data = zeroed_vec(st.eqtb.eqtb_top.index() + 1);
        st.strings.str_start = vec![PoolPos::ZERO; st.strings.max_strings.slot() + 1];
        st.strings.str_pool = vec![0; st.strings.pool_size.index() + 1];
        st.fonts.font_info = zeroed_vec(st.fonts.font_mem_size.index() + 1);
    }

    st.errors.history = HISTORY_FATAL_ERROR;
    let mut bad = 0;
    if st.errors.half_error_line < 30 || st.errors.half_error_line > st.errors.error_line - 15 {
        bad = 1;
    }
    if st.config.max_print_line < 60 {
        bad = 2;
    }
    if st.input.max_in_open >= 128 {
        bad = 6;
    }
    if st.fonts.font_max.raw() > FONT_BASE + 9000 {
        bad = 16;
    }
    if i32::try_from(st.save_stack.save_size).map_or(true, |s| s > MAX_HALFWORD)
        || st.strings.max_strings.raw() > MAX_HALFWORD
    {
        bad = 17;
    }
    if st.input.buf_size.raw() > MAX_HALFWORD {
        bad = 18;
    }
    if CS_TOKEN_FLAG + EQTB_SIZE.raw() + st.eqtb.hash_extra > MAX_HALFWORD {
        bad = 21;
    }
    if bad > 0 {
        crate::bridge::set_error_message(format!("failed internal consistency check #{bad}"));
        return InitOutcome::Terminate(HISTORY_FATAL_ERROR);
    }

    initialize_more_variables(st);
    if st.config.in_initex_mode {
        if let Err(a) = get_strings_started(st) {
            return InitOutcome::Terminate(a.into_history(st));
        }
        let format_ident = match st.strings.make_tex_string(" (INITEX)") {
            Ok(v) => v,
            Err(e) => return InitOutcome::Terminate(e.into_history(st)),
        };
        initialize_more_initex_variables(st, format_ident);
        if let Err(e) = initialize_primitives(st) {
            return InitOutcome::Terminate(e.into_history(st));
        }
        st.strings.init_str_ptr = st.strings.str_ptr;
        st.strings.init_pool_ptr = st.strings.pool_ptr;
    }

    crate::pagebuilder::initialize_pagebuilder_variables(st);
    crate::shipout::initialize_shipout_variables(st);
    if st.config.deterministic_mode {
        st.shipout.epochseconds = build_date as i32;
        st.shipout.microseconds = 0;
    } else {
        (st.shipout.epochseconds, st.shipout.microseconds) =
            crate::texmfmp::get_seconds_and_micros();
    }
    crate::texmfmp::init_start_time(st, build_date);

    st.shipout.selector = Selector::TermOnly;
    st.errors.tally = 0;
    st.shipout.term_offset = 0;
    st.shipout.file_offset = 0;
    st.strings.job_name = StrNumber::NONE;
    st.scanner.name_in_progress = false;
    st.errors.log_opened = false;

    st.input.input_stack.reset();
    st.input.open_levels[0] = crate::state::OpenLevel::default();
    st.input.in_open = 0;
    st.input.open_parens = 0;
    st.input.max_buf_stack = BufPos::ZERO;

    st.input.param_stack.reset();
    st.scanner.used_tectonic_coda_tokens = false;
    st.fonts.gave_char_warning_help = false;

    st.scanner.scanner_status = ScannerStatus::Normal;
    st.scanner.warning_index = EqtbPtr::NONE;
    st.input.first = BufPos::from_raw(1);
    st.input.cur_input.state = NEW_LINE;
    st.input.cur_input.start = BufPos::from_raw(1);
    st.input.cur_input.index = 0;
    st.mem.line = 0;
    st.input.cur_input.name = 0;
    st.input.force_eof = false;
    st.scanner.align_state = 1_000_000;

    init_io(st);

    if st.config.in_initex_mode {
        st.scanner.max_reg_num = 32767;
    }
    st.eqtb.no_new_control_sequence = true;

    if !st.config.in_initex_mode {
        match load_fmt_file(st, dump_name) {
            Ok(()) => crate::pgfmath::KnownCs::resolve(st),
            Err(a) => return InitOutcome::Terminate(a.into_history(st)),
        }
    }

    let end_line_char = st.eqtb.int_par(IntPar::EndLineChar);
    if (0..=BIGGEST_CHAR).contains(&end_line_char) {
        st.input.buffer[st.input.cur_input.limit] = end_line_char;
    } else {
        st.input.cur_input.limit -= 1;
    }

    let (time, day, month, year) = if st.config.in_initex_mode {
        (0, 0, 0, 0)
    } else {
        crate::texmfmp::get_date_and_time(build_date, st.config.deterministic_mode)
    };
    st.eqtb.set_int_par(IntPar::Time, time);
    st.eqtb.set_int_par(IntPar::Day, day);
    st.eqtb.set_int_par(IntPar::Month, month);
    st.eqtb.set_int_par(IntPar::Year, year);

    if st.hyph.trie_not_ready {
        let hyph = &mut st.hyph;
        let trie_len = hyph.trie_size as usize + 1;
        hyph.trie_trl = vec![0; trie_len];
        hyph.trie_tro = vec![0; trie_len];
        hyph.trie_trc = vec![0; trie_len];
        hyph.trie_c = vec![0; trie_len];
        hyph.trie_o = vec![0; trie_len];
        hyph.trie_l = vec![0; trie_len];
        hyph.trie_r = vec![0; trie_len];
        hyph.trie_hash = vec![0; trie_len];
        hyph.trie_taken = vec![false; trie_len];
        hyph.trie_ptr = 0;
        hyph.hyph_start = 0;

        st.fonts.alloc_font_arrays();

        let nullfont_name = match st.strings.make_tex_string("nullfont") {
            Ok(v) => v,
            Err(e) => return InitOutcome::Terminate(e.into_history(st)),
        };
        let fonts = &mut st.fonts;
        let f = NULL_FONT;
        fonts.font_ptr = NULL_FONT;
        fonts.fmem_ptr = FontInfoIdx::from_raw(7);
        fonts.font_name[f] = nullfont_name;
        fonts.font_area[f] = EMPTY_STRING;
        fonts.hyphen_char[f] = i32::from(b'-');
        fonts.skew_char[f] = -1;
        fonts.bchar_label[f] = FontInfoIdx::NON_ADDRESS;
        fonts.font_bchar[f] = TOO_BIG_CHAR;
        fonts.font_false_bchar[f] = TOO_BIG_CHAR;
        fonts.font_bc[f] = 1;
        fonts.font_ec[f] = 0;
        fonts.font_glue[f] = MemPtr::NULL;
        fonts.font_params[f] = 7;
        fonts.param_base[f] = FontInfoIdx::from_raw(-1);
        for word in &mut fonts.font_info[..7] {
            word.b32.s1 = 0;
        }
    }

    st.fonts.font_used = PerFont::filled(st.fonts.font_max.index() + 1, false);

    st.errors.random_seed = initial_random_seed(
        st.config.deterministic_mode,
        build_date,
        st.shipout.epochseconds,
        st.shipout.microseconds,
    );
    let random_seed = st.errors.random_seed;
    st.errors.init_randoms(random_seed);

    st.shipout.selector = if st.errors.interaction == InteractionMode::Batch {
        Selector::NoPrint
    } else {
        Selector::TermOnly
    };

    if st.config.semantic_pagination_enabled {
        st.eqtb.set_int_par(IntPar::XetexGenerateActualText, 1);
    }

    pdf_files_init();
    crate::synctex::synctex_init_command(st);
    InitOutcome::Ready
}

/// Open, execute, finalize, and release one batch document operation.
fn run_document_to_end(st: &mut EngineState, input_file_name: &str) -> i32 {
    let result = (|| {
        crate::io::start_input(st, Some(input_file_name))?;
        st.errors.history = HISTORY_SPOTLESS;
        crate::main_control::main_control(st)?;
        final_cleanup(st)?;
        crate::io::close_files_and_terminate(st)
    })();
    if let Err(a) = result {
        return a.into_history(st);
    }
    cleanup(st);
    st.errors.history
}

/// Run and finalize one document while retaining the initialized engine arenas.
/// Aborts are returned without terminal cleanup so a resident session can
/// close operation resources and rewind rung zero.
pub(crate) fn run_resident_document(
    st: &mut EngineState,
    input_file_name: &str,
) -> Result<i32, Abort> {
    crate::io::start_input(st, Some(input_file_name))?;
    st.errors.history = HISTORY_SPOTLESS;
    crate::main_control::main_control(st)?;
    final_cleanup(st)?;
    crate::io::close_files_and_terminate(st)?;
    Ok(st.errors.history)
}

/// Resume a resident document at a restored command boundary.
pub(crate) fn resume_resident_document(
    st: &mut EngineState,
    resume_pending_command: bool,
) -> Result<i32, Abort> {
    crate::io::reopen_resident_outputs(st)?;
    crate::main_control::main_control_resume(st, resume_pending_command)?;
    final_cleanup(st)?;
    crate::io::close_files_and_terminate(st)?;
    Ok(st.errors.history)
}

/// Close input slots that may remain live after an interrupted resident op.
pub(crate) fn release_operation_inputs(st: &mut EngineState) {
    for slot in st
        .input
        .input_file
        .iter_mut()
        .chain(&mut st.input.read_file)
    {
        let _ = crate::io::u_close(std::mem::take(slot));
    }
}

fn initial_random_seed(
    deterministic_mode: bool,
    build_date: libc::time_t,
    epochseconds: i32,
    microseconds: i32,
) -> i32 {
    if deterministic_mode {
        (build_date.wrapping_mul(1_000_003) & 0x7FFF_FFFF) as i32
    } else {
        microseconds * 1000 + (epochseconds % 1_000_000)
    }
}

pub(crate) fn cleanup(st: &mut EngineState) {
    crate::journal::disarm(st);
    pdf_files_close();
    st.shipout.dvi.release_buffer();

    texyon_xetex_layout::FontManager::destroy();
    st.fonts.release_linebreak_state();

    st.fonts.font_layout_engine.as_mut_slice().fill(None);
    st.fonts.font_mapping.as_mut_slice().fill(None);
    st.fonts.loaded_font_mapping = None;

    for i in 1..=st.input.in_open {
        // Teardown has begun, so a serious close error is absorbed.
        let _ = crate::io::u_close(std::mem::take(&mut st.input.input_file[i]));
    }
    for slot in st.input.read_file.iter_mut() {
        *slot = crate::state::InputSlot::Empty;
    }

    st.save_stack.data = Vec::new();

    let input = &mut st.input;
    input.name_of_input_file = Vec::new();
    input.abspath_of_input_file = Vec::new();
    input.nest.release();
    input.input_file = Vec::new();
    input.read_file = Default::default();
    input.input_stack.release();
    input.param_stack.release();
    input.open_levels = Vec::new();
    input.buffer = crate::state::InputBuffer::default();
    input.name_of_file = Vec::new();

    st.scanner.native_text = Vec::new();
    st.scanner.native_text_size = 0;
    st.scanner.native_len = 0;
    st.scanner.save_native_len = 0;

    st.eqtb.data = Vec::new();
    st.eqtb.hash = Vec::new();
    st.mem.data = Vec::new();
    st.mem.glyph_blobs.clear();

    st.strings.str_start = Vec::new();
    st.strings.str_pool = Vec::new();

    let fonts = &mut st.fonts;
    fonts.font_check = PerFont::default();
    fonts.font_size = PerFont::default();
    fonts.font_dsize = PerFont::default();
    fonts.font_params = PerFont::default();
    fonts.font_name = PerFont::default();
    fonts.font_area = PerFont::default();
    fonts.font_bc = PerFont::default();
    fonts.font_ec = PerFont::default();
    fonts.bchar_label = PerFont::default();
    fonts.font_bchar = PerFont::default();
    fonts.font_false_bchar = PerFont::default();
    fonts.font_glue = PerFont::default();
    fonts.font_used = PerFont::default();
    fonts.font_info = Vec::new();
    fonts.font_layout_engine = PerFont::default();
    fonts.font_mapping = PerFont::default();
    fonts.font_flags = PerFont::default();
    fonts.font_letter_space = PerFont::default();
    fonts.hyphen_char = PerFont::default();
    fonts.skew_char = PerFont::default();
    fonts.char_base = PerFont::default();
    fonts.width_base = PerFont::default();
    fonts.height_base = PerFont::default();
    fonts.depth_base = PerFont::default();
    fonts.italic_base = PerFont::default();
    fonts.lig_kern_base = PerFont::default();
    fonts.kern_base = PerFont::default();
    fonts.exten_base = PerFont::default();
    fonts.param_base = PerFont::default();
    fonts.mapped_text = Vec::new();

    let hyph = &mut st.hyph;
    hyph.hyph_word = Vec::new();
    hyph.hyph_list = Vec::new();
    hyph.hyph_link = Vec::new();
    hyph.trie_trl = Vec::new();
    hyph.trie_tro = Vec::new();
    hyph.trie_trc = Vec::new();
    hyph.trie_c = Vec::new();
    hyph.trie_o = Vec::new();
    hyph.trie_l = Vec::new();
    hyph.trie_r = Vec::new();
    hyph.trie_hash = Vec::new();
    hyph.trie_taken = Vec::new();
}

#[cfg(test)]
mod deterministic_input_tests {
    use super::initial_random_seed;

    #[test]
    fn deterministic_seed_depends_only_on_build_date() {
        let a = initial_random_seed(true, 1_700_000_000, 1, 2);
        let b = initial_random_seed(true, 1_700_000_000, 999_999, 888_888);
        let c = initial_random_seed(true, 1_700_000_001, 1, 2);

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a >= 0);
    }

    #[test]
    fn nondeterministic_seed_preserves_the_live_clock_formula() {
        assert_eq!(initial_random_seed(false, 0, 1_234_567, 42), 276_567);
        assert_eq!(
            initial_random_seed(false, 1_700_000_000, 1_234_567, 42),
            276_567
        );
    }
}
