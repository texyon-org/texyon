//! Engine init, run and cleanup: the `run_engine` entry point, the
//! initialize_* primers, primitive registration and teardown.

use std::panic::AssertUnwindSafe;
use std::{iter, mem, panic};

use crate::base::cmd::Cmd;
use crate::base::codes::{BoxCode, FiOrElseCode, IfTest, MarkCode, ScannerStatus};
use crate::base::command::Command;
use crate::base::consts::{
    BIGGEST_CHAR, BIGGEST_LANG, CLOSED, EMPTY_STRING, FONT_BASE, HISTORY_FATAL_ERROR,
    HISTORY_SPOTLESS, HISTORY_WARNING_ISSUED, LEVEL_ONE, LEVEL_ZERO, MAX_FONT_MAX, MAX_HALFWORD,
    NORMAL, NULL_FONT, NUMBER_USVS, TEX_NULL, TOO_BIG_CHAR,
};
use crate::base::mode::{Mode, ModeName};
use crate::base::params::IntPar;
use crate::base::stack::TexStack;
use crate::base::token::{Token, TokenList};
use crate::base::units::{
    BufPos, FontId, GlueOrder, NodeType, PoolPos, PrimNum, SavePtr, Scaled, StrNumber,
};
use crate::build::hyphenation::{HYPH_PRIME, TRIE_OP_SIZE, TrieOp, TriePtr};
use crate::build::main_control::{main_control, main_control_resume};
use crate::build::packaging::{CurMarks, MarksAction};
use crate::build::pagebuilder::{PageContents, initialize_pagebuilder_variables};
use crate::ffi::bridge::{enter, output_open_stdout, set_error_message};
use crate::ffi::clock::{get_date_and_time, get_seconds_and_micros, init_start_time};
use crate::files::format_io::{load_fmt_file, store_fmt_file};
use crate::files::io::{
    close_files_and_terminate, init_io, open_log_file, reopen_resident_outputs, start_input,
    u_close,
};
use crate::fonts::tfm::{FontMetrics, FontParams};
use crate::fonts::{FontState, LigChar, PerFont};
use crate::memory::mem::{
    CONTRIB_HEAD, END_SPAN, FIL_GLUE, FIL_NEG_GLUE, FILL_GLUE, FIXED_HEADS, FreeLists, PAGE_HEAD,
    PAGE_INS_HEAD, RESERVED_WORDS, SS_GLUE, ZERO_GLUE,
};
use crate::memory::mem_types::zeroed_vec;
use crate::memory::node::{NodePtr, PageInsNode};
use crate::preview::journal::disarm;
use crate::print::errors::{Abort, InteractionMode, OrConfusion};
use crate::print::{
    Selector, print, print_char, print_command, print_esc, print_int, print_nl, show_save_groups,
};
use crate::scanning::cond_stack::CondStack;
use crate::scanning::input::{
    InputBuffer, InputName, InputSlot, NEW_LINE, OpenLevel, STACK_SIZE, TOKEN_LIST,
    end_file_reading, end_token_list,
};
use crate::scanning::pgfmath::KnownCs;
use crate::scanning::scan::{set_class, set_family};
use crate::shipout::initialize_shipout_variables;
use crate::shipout::synctex::synctex_init_command;
use crate::state::{ConfigState, EngineState};
use crate::tables::cs::{CsId, Frozen, NameSlot};
use crate::tables::eqtb::{CatLoc, DelLoc, GlueLoc, LcLoc, MathLoc, SfLoc, UcLoc};
use crate::tables::hash::{HASH_PRIME, HASH_SIZE};
use crate::tables::primitives::{PRIMITIVES, PrimInit};
use crate::tables::save::GroupCode;

pub(crate) const POOL_FREE: i32 = 47_500;
pub(crate) const STRINGS_FREE: i32 = 100;
const HYPH_SIZE: i32 = 8_191;

const TOO_BIG_LANG: i32 = 256;
const VAR_FAM_CLASS: i32 = 7;
const NEST_SIZE: usize = 500;
const PARAM_SIZE: usize = 10_000;

const _: () = assert!(
    HASH_PRIME <= HASH_SIZE
        && TEX_NULL <= 0
        && MAX_FONT_MAX >= TEX_NULL
        && MAX_FONT_MAX <= MAX_HALFWORD
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

/// Defines primitive `ident` and returns its control sequence.
fn primitive(st: &mut EngineState, ident: &str, c: Cmd, o: i32) -> Result<CsId, Abort> {
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
        st.eqtb
            .prim_lookup(&mut st.strings, s)?
            .or_confusion("primitive")?
    } else {
        cs = CsId::single(u16::from(bytes[0]));
        st.eqtb
            .prim_lookup(&mut st.strings, StrNumber::single_char(i32::from(bytes[0])))?
            .or_confusion("primitive")?
    };

    let meaning = Command::decode(c, o).ok_or(Abort::Confusion("primitive"))?;
    st.eqtb.set_meaning(cs, meaning, LEVEL_ONE);
    st.eqtb
        .set_meaning(CsId::primitive(prim_val), meaning, LEVEL_ONE);
    Ok(cs)
}

pub(crate) fn final_cleanup(st: &mut EngineState) -> Result<(), Abort> {
    let dump = st.scanner.cur == Command::Stop { dump: true };

    if !dump {
        st.eqtb.set_int_par(IntPar::NewLineChar, -1);
    }

    if st.strings.job_name == StrNumber::NONE {
        open_log_file(st)?;
    }

    while !st.input.input_stack.is_empty() {
        if st.input.cur_input.state == TOKEN_LIST {
            end_token_list(st)?;
        } else {
            end_file_reading(st)?;
        }
    }

    while st.input.open_parens > 0 {
        print(st, " )");
        st.input.open_parens -= 1;
    }

    let cur_level = st.save_stack.cur_level;
    if cur_level > LEVEL_ONE {
        print_nl(st, b'(');
        print_esc(st, "end occurred ");
        print(st, "inside a group at level ");
        print_int(st, i32::from(cur_level) - 1);
        print_char(st, b')');
        show_save_groups(st);
    }

    while let Some(cond) = st.scanner.conds.pop() {
        print_nl(st, b'(');
        print_esc(st, "end occurred ");
        print(st, "when ");
        print_command(st, Command::IfTest(st.scanner.cur_if));
        let if_line = st.scanner.if_line;
        if if_line != 0 {
            print(st, " on line ");
            print_int(st, if_line);
        }
        print(st, " was incomplete)");
        st.scanner.if_line = cond.if_line;
        st.scanner.cur_if = cond.cur_if;
    }

    if st.errors.history != HISTORY_SPOTLESS
        && (st.errors.history == HISTORY_WARNING_ISSUED
            || st.errors.interaction < InteractionMode::ErrorStop)
        && st.shipout.selector == Selector::TermAndLog
    {
        st.shipout.selector = Selector::TermOnly;
        print_nl(st, "(see the transcript file for additional information)");
        st.shipout.selector = Selector::TermAndLog;
    }

    if !dump {
        return Ok(());
    }
    if !st.config.in_initex_mode {
        print_nl(st, "(\\dump is performed only by INITEX)");
        return Ok(());
    }

    for mark_code in MarkCode::ALL {
        let mark = st.scanner.cur_mark[mark_code];
        if !mark.is_null() {
            st.mem.tokens.delete_ref(mark);
        }
    }

    st.mem
        .do_marks(&mut st.scanner.mark_classes, MarksAction::DestroyMarks);

    for box_code in BoxCode::LastBox as usize..=BoxCode::Vsplit as usize {
        st.mem.flush_node_list(st.scanner.disc_ptr[box_code])?;
    }

    if let Some(spec) = st.shipout.last_glue {
        st.mem.delete_glue_ref(spec);
    }

    store_fmt_file(st)
}

fn initialize_more_variables(st: &mut EngineState) {
    st.shipout.doing_special = false;
    st.scanner.native_text_size = 128;
    st.scanner.native_text = vec![0; 128];

    // There is no terminal to answer TeX, so it runs as in nonstopmode.
    st.errors.interaction = InteractionMode::Nonstop;

    st.errors.deletions_allowed = true;
    st.scanner.set_box_allowed = true;
    st.errors.error_count = 0;
    st.errors.help(&[]);
    st.errors.use_err_help = false;

    st.input.nest.reset();
    st.input.cur_list.mode = Mode::vertical(false);
    st.input.cur_list.head = CONTRIB_HEAD;
    st.input.cur_list.tail = CONTRIB_HEAD;
    st.input.cur_list.e_tex_aux = None;
    st.input.cur_list.mode_line = 0;
    st.input.cur_list.prev_graf = 0;
    st.scanner.shown_mode = ModeName::NoMode;
    st.shipout.page_contents = PageContents::Empty;
    st.shipout.page_tail = Some(PAGE_HEAD);
    st.shipout.last_glue = None;
    st.shipout.last_penalty = 0;
    st.shipout.last_kern = Scaled::ZERO;
    st.shipout.last_node_type = -1;
    st.shipout.page_so_far[7] = Scaled::ZERO;

    st.eqtb.no_new_control_sequence = true;
    for k in PrimNum::all() {
        st.eqtb.set_prim_next(k, None);
        st.eqtb.set_prim_text(k, None);
    }

    st.save_stack.save_ptr = SavePtr::ZERO;
    st.save_stack.cur_level = LEVEL_ONE;
    st.save_stack.cur_group = GroupCode::BottomLevel;
    st.save_stack.cur_boundary = None;
    st.save_stack.max_save_stack = SavePtr::ZERO;
    st.scanner.mag_set = 0;
    st.scanner.expand_depth_count = 0;
    st.scanner.is_in_csname = false;
    st.scanner.cur_mark = CurMarks::NULL;

    st.input.read_open = [CLOSED; 17];

    st.scanner.conds = CondStack::default();
    st.scanner.if_limit = FiOrElseCode::Normal.into();
    st.scanner.cur_if = IfTest::default();
    st.scanner.if_line = 0;
    st.shipout.total_pages = 0;
    st.shipout.max_v = Scaled::ZERO;
    st.shipout.max_h = Scaled::ZERO;
    st.shipout.dvi.max_push = 0;
    st.shipout.dvi.last_bop = -1;
    st.shipout.doing_leaders = false;
    st.shipout.dead_cycles = 0;
    st.shipout.last_badness = 0;
    st.scanner.align_stack.clear();
    st.scanner.cur_align = None;
    st.scanner.cur_span = None;
    st.scanner.cur_loop = None;
    st.scanner.cur_head = None;
    st.scanner.cur_tail = None;
    st.scanner.cur_pre_head = None;
    st.scanner.cur_pre_tail = None;
    st.scanner.cur_f = NULL_FONT;
    st.hyph.max_hyph_char = TOO_BIG_LANG;

    let hyph_len = st.hyph.hyph_size as usize + 1;
    st.hyph.hyph_word[..hyph_len].fill(StrNumber::NONE);
    st.hyph.hyph_list[..hyph_len].fill(None);
    st.hyph.hyph_link[..hyph_len].fill(0);

    st.hyph.hyph_count = 0;
    st.hyph.hyph_next = HYPH_PRIME + 1;
    if st.hyph.hyph_next > st.hyph.hyph_size {
        st.hyph.hyph_next = HYPH_PRIME;
    }

    st.shipout.output_active = false;
    st.shipout.output_can_end = false;
    st.shipout.insert_penalties = 0;
    st.scanner.after_token = Token::ZERO;
    st.errors.long_help_seen = false;
    st.strings.format_ident = StrNumber::NONE;

    st.shipout.write_open = [false; 18];

    st.input.pseudo_files = None;
    st.scanner.mark_classes.clear();
    st.scanner.disc_ptr[BoxCode::LastBox as usize] = None;
    st.scanner.disc_ptr[BoxCode::Vsplit as usize] = None;
}

fn initialize_more_initex_variables(st: &mut EngineState, format_ident: StrNumber) {
    let mem = &mut st.mem;
    for spec in [ZERO_GLUE, FIL_GLUE, FILL_GLUE, SS_GLUE, FIL_NEG_GLUE] {
        mem[spec].width = Scaled::ZERO;
        mem[spec].stretch = Scaled::ZERO;
        mem[spec].shrink = Scaled::ZERO;
        mem[spec].ref_count = TEX_NULL + 1;
        mem[spec].stretch_order = GlueOrder::Normal.into();
        mem[spec].shrink_order = GlueOrder::Normal.into();
    }

    mem[FIL_GLUE].stretch = Scaled::UNITY;
    mem[FIL_GLUE].stretch_order = GlueOrder::Fil.into();
    mem[FILL_GLUE].stretch = Scaled::UNITY;
    mem[FILL_GLUE].stretch_order = GlueOrder::Fill.into();
    mem[SS_GLUE].stretch = Scaled::UNITY;
    mem[SS_GLUE].stretch_order = GlueOrder::Fil.into();
    mem[SS_GLUE].shrink = Scaled::UNITY;
    mem[SS_GLUE].shrink_order = GlueOrder::Fil.into();
    mem[FIL_NEG_GLUE].stretch = -Scaled::UNITY;
    mem[FIL_NEG_GLUE].stretch_order = GlueOrder::Fil.into();

    for head in FIXED_HEADS {
        mem.set_header(head, NodeType::Head, 0);
    }

    mem.tokens
        .init_omit_template(Token::from_cs(Frozen::EndTemplate.cs()));
    mem[END_SPAN].span_count = i32::from(u16::MAX) + 1;
    let page_ins = NodePtr::<PageInsNode>::at(PAGE_INS_HEAD);
    mem[page_ins].box_number = 255;
    mem[page_ins].set_split_up(true);
    mem[page_ins].link = Some(PAGE_INS_HEAD);
    mem.set_header(PAGE_HEAD, NodeType::Glue, NORMAL);

    let eqtb = &mut st.eqtb;
    eqtb.meanings.fill(Command::UndefinedCs, LEVEL_ZERO);

    // One reference per glue table entry, as WEB's `local_base - glue_base`.
    let entries = i32::try_from(GlueLoc::LEN).unwrap_or(i32::MAX);
    let zero_glue_refs = st.mem[ZERO_GLUE].ref_count + entries;
    st.mem[ZERO_GLUE].ref_count = zero_glue_refs;

    for k in 0..NUMBER_USVS {
        eqtb.cat_codes.set_value(CatLoc::of(k), Cmd::OtherChar);
        eqtb.math_codes.set_value(MathLoc::of(k), k);
        eqtb.sf_codes.set_value(SfLoc::of(k), 1000);
    }

    for (c, cat) in [
        (13, Cmd::CarRet),
        (32, Cmd::Spacer),
        (92, Cmd::ESCAPE),
        (37, Cmd::COMMENT),
        (127, Cmd::INVALID_CHAR),
        (0, Cmd::IGNORE),
    ] {
        eqtb.cat_codes.set_value(CatLoc::of(c), cat);
    }

    for k in (i32::from(b'0'))..=(i32::from(b'9')) {
        eqtb.math_codes
            .set_value(MathLoc::of(k), k + set_class(VAR_FAM_CLASS));
    }

    for k in (i32::from(b'A'))..=(i32::from(b'Z')) {
        let lower = k + 32;
        for c in [k, lower] {
            eqtb.cat_codes.set_value(CatLoc::of(c), Cmd::Letter);
            let math = c + set_family(1) + set_class(VAR_FAM_CLASS);
            eqtb.math_codes.set_value(MathLoc::of(c), math);
            eqtb.lc_codes.set_value(LcLoc::of(c), lower);
            eqtb.uc_codes.set_value(UcLoc::of(c), k);
        }
        eqtb.sf_codes.set_value(SfLoc::of(k), 999);
    }

    eqtb.set_int_par(IntPar::Mag, 1000);
    eqtb.set_int_par(IntPar::Tolerance, 10000);
    eqtb.set_int_par(IntPar::HangAfter, 1);
    eqtb.set_int_par(IntPar::MaxDeadCycles, 25);
    eqtb.set_int_par(IntPar::EscapeChar, i32::from(b'\\'));
    eqtb.set_int_par(IntPar::EndLineChar, i32::from(b'\r'));

    eqtb.del_codes.set_value(DelLoc::of(i32::from(b'.')), 0);
    eqtb.set_int_par(IntPar::ShowStream, -1);

    eqtb.prim_used = PrimNum::LAST;
    eqtb.hash_used = NameSlot::BUCKETS_END;
    eqtb.hash_high = 0;
    eqtb.cs_count = 0;

    let hyph = &mut st.hyph;
    hyph.trie_op_hash_array[..=(2 * TRIE_OP_SIZE) as usize].fill(0);
    hyph.trie_used[..=BIGGEST_LANG as usize].fill(TrieOp::NONE.raw());
    hyph.max_op_used = TrieOp::NONE.raw();
    hyph.trie_op_ptr = 0;
    hyph.trie_not_ready = true;

    st.strings.format_ident = format_ident;

    st.scanner.max_reg_num = 32767;

    st.eqtb.set_int_par(IntPar::XetexHyphenatableLength, 63);
}

fn set_frozen_text(st: &mut EngineState, p: CsId, text: &str) -> Result<(), Abort> {
    let s = st.strings.make_tex_string(text)?;
    st.eqtb.set_text(p, s);
    Ok(())
}

fn initialize_primitives(st: &mut EngineState) -> Result<(), Abort> {
    st.eqtb.no_new_control_sequence = false;
    st.input.first = BufPos::ZERO;

    for prim in PRIMITIVES {
        let cs = primitive(st, prim.name, prim.cmd, prim.chr)?;
        match prim.init {
            PrimInit::None => {}
            PrimInit::Par => {
                st.input.par_loc = Some(cs);
                st.input.par_token = Token::from_cs(cs);
            }
            PrimInit::Write => {
                st.shipout.write_loc = Some(cs);
            }
            PrimInit::Frozen(frozen) => {
                set_frozen_text(st, frozen, prim.name)?;
                st.eqtb
                    .set_meaning(frozen, st.eqtb.meaning(cs), st.eqtb.cs_level(cs));
            }
        }
    }

    set_frozen_text(st, Frozen::EndTemplate.cs(), "endtemplate")?;
    st.eqtb
        .set_meaning(Frozen::EndTemplate.cs(), Command::EndTemplate, LEVEL_ONE);

    set_frozen_text(st, Frozen::Endv.cs(), "endtemplate")?;
    st.eqtb
        .set_meaning(Frozen::Endv.cs(), Command::Endv, LEVEL_ONE);

    set_frozen_text(st, Frozen::DontExpand.cs(), "notexpanded:")?;
    st.eqtb
        .set_meaning(Frozen::DontExpand.cs(), Command::DontExpand, LEVEL_ZERO);

    set_frozen_text(st, Frozen::Primitive.cs(), "primitive")?;
    st.eqtb.set_meaning(
        Frozen::Primitive.cs(),
        Command::IgnoreSpaces { primitive: true },
        LEVEL_ONE,
    );

    set_frozen_text(st, Frozen::Protection.cs(), "inaccessible")?;

    set_frozen_text(st, Frozen::EndWrite.cs(), "endwrite")?;
    st.eqtb.set_meaning(
        Frozen::EndWrite.cs(),
        Command::OuterCall(TokenList::NULL),
        LEVEL_ONE,
    );

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
    let _bridge = enter(api);

    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
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
            set_error_message(msg);
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
        self.font_name = PerFont::filled(len, StrNumber::NONE);
        self.font_area = PerFont::filled(len, StrNumber::NONE);
        self.font_bc = PerFont::filled(len, 0);
        self.font_ec = PerFont::filled(len, 0);
        self.font_bchar = PerFont::filled(len, LigChar::of(0));
        self.font_false_bchar = PerFont::filled(len, LigChar::of(0));
        self.font_glue = PerFont::filled(len, None);
        self.hyphen_char = PerFont::filled(len, 0);
        self.skew_char = PerFont::filled(len, 0);
        self.metrics = PerFont::filled(len, FontMetrics::default());
        self.params = PerFont::filled(len, FontParams::default());
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
    st.fonts.font_mem_size = 8_000_000;
    st.fonts.font_max = FontId::new(9_000);
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
    st.input.buffer = InputBuffer(vec![0; st.input.buf_size.index() + 1]);
    st.input.nest = TexStack::with_size(NEST_SIZE);
    st.save_stack.alloc(st.save_stack.save_size + 1);
    st.input.input_file = iter::repeat_with(|| InputSlot::Empty)
        .take(in_open_words)
        .collect();
    st.input.input_stack = TexStack::with_size(STACK_SIZE);
    st.input.open_levels = vec![OpenLevel::default(); in_open_words];
    st.input.param_stack = TexStack::with_size(PARAM_SIZE);
    st.hyph.hyph_word = vec![StrNumber::NONE; hyph_words];
    st.hyph.hyph_list = vec![None; hyph_words];
    st.hyph.hyph_link = vec![0; hyph_words];

    if st.config.in_initex_mode {
        st.mem.data = zeroed_vec(RESERVED_WORDS);
        st.mem.free = FreeLists::default();

        st.eqtb.alloc_tables();
        st.strings.str_start = vec![PoolPos::ZERO; st.strings.max_strings.slot() + 1];
        st.strings.str_pool = vec![0; st.strings.pool_size.index() + 1];
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
    if !Token::can_name(CsId::top(st.eqtb.hash_extra)) {
        bad = 21;
    }
    if bad > 0 {
        set_error_message(format!("failed internal consistency check #{bad}"));
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

    initialize_pagebuilder_variables(st);
    initialize_shipout_variables(st);
    if st.config.deterministic_mode {
        st.shipout.epochseconds = build_date as i32;
        st.shipout.microseconds = 0;
    } else {
        (st.shipout.epochseconds, st.shipout.microseconds) = get_seconds_and_micros();
    }
    init_start_time(st, build_date);

    st.shipout.selector = Selector::TermOnly;
    st.errors.tally = 0;
    st.shipout.term_offset = 0;
    st.shipout.file_offset = 0;
    st.strings.job_name = StrNumber::NONE;
    st.scanner.name_in_progress = false;
    st.errors.log_opened = false;

    st.input.input_stack.reset();
    st.input.open_levels[0] = OpenLevel::default();
    st.input.in_open = 0;
    st.input.open_parens = 0;
    st.input.max_buf_stack = BufPos::ZERO;

    st.input.param_stack.reset();
    st.scanner.used_tectonic_coda_tokens = false;
    st.fonts.gave_char_warning_help = false;

    st.scanner.scanner_status = ScannerStatus::Normal;
    st.scanner.warning_index = None;
    st.input.first = BufPos::from_raw(1);
    st.input.cur_input.state = NEW_LINE;
    st.input.cur_input.start = BufPos::from_raw(1);
    st.input.cur_input.index = 0;
    st.mem.line = 0;
    st.input.cur_input.name = InputName::Terminal;
    st.input.force_eof = false;
    st.scanner.align_state = 1_000_000;

    init_io(st);

    if st.config.in_initex_mode {
        st.scanner.max_reg_num = 32767;
    }
    st.eqtb.no_new_control_sequence = true;

    if !st.config.in_initex_mode {
        match load_fmt_file(st, dump_name) {
            Ok(()) => KnownCs::resolve(st),
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
        get_date_and_time(build_date, st.config.deterministic_mode)
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
        hyph.hyph_start = TriePtr::NULL;

        st.fonts.alloc_font_arrays();

        let nullfont_name = match st.strings.make_tex_string("nullfont") {
            Ok(v) => v,
            Err(e) => return InitOutcome::Terminate(e.into_history(st)),
        };
        let fonts = &mut st.fonts;
        let f = NULL_FONT;
        fonts.font_ptr = NULL_FONT;
        fonts.fmem_ptr = 7;
        fonts.font_name[f] = nullfont_name;
        fonts.font_area[f] = EMPTY_STRING;
        fonts.hyphen_char[f] = i32::from(b'-');
        fonts.skew_char[f] = -1;
        fonts.font_bchar[f] = LigChar::NONE;
        fonts.font_false_bchar[f] = LigChar::NONE;
        fonts.font_bc[f] = 1;
        fonts.font_ec[f] = 0;
        fonts.font_glue[f] = None;
        fonts.metrics[f] = FontMetrics::default();
        fonts.params[f] = FontParams::new(vec![Scaled::ZERO; 7]);
    }

    st.fonts.font_used = PerFont::filled(st.fonts.font_max.index() + 1, false);

    st.random.reseed(initial_random_seed(
        st.config.deterministic_mode,
        build_date,
        st.shipout.epochseconds,
        st.shipout.microseconds,
    ));

    st.shipout.selector = if st.errors.interaction == InteractionMode::Batch {
        Selector::NoPrint
    } else {
        Selector::TermOnly
    };

    if st.config.semantic_pagination_enabled {
        st.eqtb.set_int_par(IntPar::XetexGenerateActualText, 1);
    }

    pdf_files_init();
    synctex_init_command(st);
    InitOutcome::Ready
}

/// Open, execute, finalize, and release one batch document operation.
fn run_document_to_end(st: &mut EngineState, input_file_name: &str) -> i32 {
    let result = (|| {
        start_input(st, Some(input_file_name))?;
        st.errors.history = HISTORY_SPOTLESS;
        main_control(st)?;
        final_cleanup(st)?;
        close_files_and_terminate(st)
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
    start_input(st, Some(input_file_name))?;
    st.errors.history = HISTORY_SPOTLESS;
    main_control(st)?;
    final_cleanup(st)?;
    close_files_and_terminate(st)?;
    Ok(st.errors.history)
}

/// Resume a resident document at a restored command boundary.
pub(crate) fn resume_resident_document(
    st: &mut EngineState,
    resume_pending_command: bool,
) -> Result<i32, Abort> {
    reopen_resident_outputs(st)?;
    main_control_resume(st, resume_pending_command)?;
    final_cleanup(st)?;
    close_files_and_terminate(st)?;
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
        let _ = u_close(mem::take(slot));
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
    disarm(st);
    pdf_files_close();
    st.shipout.dvi.release_buffer();

    texyon_xetex_layout::FontManager::destroy();
    st.fonts.release_linebreak_state();

    st.fonts.font_layout_engine.as_mut_slice().fill(None);
    st.fonts.font_mapping.as_mut_slice().fill(None);
    st.fonts.loaded_font_mapping = None;

    for i in 1..=st.input.in_open {
        // Teardown has begun, so a serious close error is absorbed.
        let _ = u_close(mem::take(&mut st.input.input_file[i]));
    }
    for slot in st.input.read_file.iter_mut() {
        *slot = InputSlot::Empty;
    }

    st.save_stack.data = Vec::new();

    let input = &mut st.input;
    input.nest.release();
    input.input_file = Vec::new();
    input.read_file = Default::default();
    input.input_stack.release();
    input.param_stack.release();
    input.open_levels = Vec::new();
    input.buffer = InputBuffer::default();

    st.scanner.native_text = Vec::new();
    st.scanner.native_text_size = 0;
    st.scanner.native_len = 0;

    st.eqtb.release_tables();
    st.eqtb.hash = Vec::new();
    st.mem.data = Vec::new();
    st.mem.glyph_blobs.clear();

    st.strings.str_start = Vec::new();
    st.strings.str_pool = Vec::new();

    let fonts = &mut st.fonts;
    fonts.font_check = PerFont::default();
    fonts.font_size = PerFont::default();
    fonts.font_dsize = PerFont::default();
    fonts.font_name = PerFont::default();
    fonts.font_area = PerFont::default();
    fonts.font_bc = PerFont::default();
    fonts.font_ec = PerFont::default();
    fonts.font_bchar = PerFont::default();
    fonts.font_false_bchar = PerFont::default();
    fonts.font_glue = PerFont::default();
    fonts.font_used = PerFont::default();
    fonts.font_layout_engine = PerFont::default();
    fonts.font_mapping = PerFont::default();
    fonts.font_flags = PerFont::default();
    fonts.font_letter_space = PerFont::default();
    fonts.hyphen_char = PerFont::default();
    fonts.skew_char = PerFont::default();
    fonts.metrics = PerFont::default();
    fonts.params = PerFont::default();
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

/// Runs its closure when dropped, including during unwinding.
struct OnDrop<F: FnMut()>(F);

impl<F: FnMut()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        (self.0)();
    }
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
