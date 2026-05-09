//! Page builder functions translated from `xetex/xetex-pagebuilder.c`.
#![allow(non_upper_case_globals)]

use crate::globals::{
    best_height_plus_depth, cur_mark, cur_ptr, dead_cycles, disc_ptr, eqtb_b32s1, insert_penalties,
    last_glue, line, mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1, nest_ptr, output_active,
    page_contents, page_so_far, page_tail, sa_root, semantic_pagination_enabled,
    set_best_height_plus_depth, set_cur_list_aux_b32s1, set_cur_list_mode, set_cur_list_mode_line,
    set_cur_list_tail, set_cur_mark, set_dead_cycles, set_disc_ptr, set_eqtb_b32s1, set_help_line,
    set_help_ptr, set_insert_penalties, set_last_glue, set_last_kern, set_last_node_type,
    set_last_penalty, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1, set_nest_tail,
    set_output_active, set_page_contents, set_page_so_far, set_page_tail, set_sa_root, temp_ptr,
};

// File-static variables (C module-level statics)
static mut BEST_PAGE_BREAK: i32 = 0;
static mut BEST_SIZE: i32 = 0;
static mut LEAST_PAGE_COST: i32 = 0;
static mut PAGE_MAX_DEPTH: i32 = 0;

// Node types (xetex_format.h)
const HLIST_NODE: i32 = 0;
const VLIST_NODE: i32 = 1;
const RULE_NODE: i32 = 2;
const INS_NODE: i32 = 3;
const MARK_NODE: i32 = 4;
const WHATSIT_NODE: i32 = 8;
const MATH_NODE: i32 = 9;
const GLUE_NODE: i32 = 10;
const KERN_NODE: i32 = 11;
const PENALTY_NODE: i32 = 12;
const PIC_NODE: i32 = 43;
const PDF_NODE: i32 = 44;

// Glue/page constants (xetex-constants.h)
const TEX_NULL: i32 = -0x0FFF_FFFF;
const MEM_TOP: i32 = 4_999_999;
const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
const PAGE_INS_HEAD: i32 = MEM_TOP;
const CONTRIB_HEAD: i32 = MEM_TOP - 1;
const PAGE_HEAD: i32 = MEM_TOP - 2;
const HOLD_HEAD: i32 = MEM_TOP - 4;
const NORMAL: i32 = 0;
const EMPTY: u8 = 0;
const INSERTS_ONLY: u8 = 1;
const BOX_THERE: u8 = 2;
const INSERTING: u16 = 0;
const SPLIT_UP: i32 = 1;
const MARK_VAL: usize = 7;
const FIRE_UP_INIT: i16 = 1;
const FIRE_UP_DONE: i16 = 2;
const PAGE_INS_NODE_SIZE: i32 = 4;
const INS_NODE_SIZE: i32 = 5;
const BOX_NODE_SIZE: i32 = 8;
const ADDITIONAL: i16 = 1;
const EXACTLY: i16 = 0;
const VMODE: i16 = 1;
const OUTPUT_GROUP: u8 = 8;
const OUTPUT_TEXT: u16 = 7;
const IGNORE_DEPTH: i32 = -65_536_000;
const EJECT_PENALTY: i32 = -10_000;
const INF_BAD: i32 = 10_000;
const INF_PENALTY: i32 = 10_000;
const AWFUL_BAD: i32 = MAX_HALFWORD;
const TOP_MARK_CODE: usize = 0;
const FIRST_MARK_CODE: usize = 1;
const BOT_MARK_CODE: usize = 2;
const LAST_BOX_CODE: usize = 2;
const COPY_CODE: usize = 1;

// eqtb bases and parameter indices (xetex_format.h)
const INT_BASE: i32 = 7_826_729;
const INT_PAR__output_penalty: i32 = 39;
const INT_PAR__max_dead_cycles: i32 = 40;
const INT_PAR__holding_inserts: i32 = 53;
const INT_PAR__saving_vdiscards: i32 = 63;
const INT_PAR__vbadness: i32 = 27;
const DIMEN_BASE: i32 = 8_941_180;
const DIMEN_PAR__vsize: i32 = 4;
const DIMEN_PAR__max_depth: i32 = 5;
const DIMEN_PAR__vfuzz: i32 = 9;
const GLUE_BASE: i32 = 2_254_340;
const GLUE_PAR__top_skip: i32 = 9;
const GLUE_PAR__split_top_skip: i32 = 10;
const LOCAL_BASE: i32 = 2_254_871;
const LOCAL__output_routine: i32 = 1;
const BOX_BASE: i32 = 2_255_144;
const SKIP_BASE: i32 = 2_254_359;
const COUNT_BASE: i32 = 7_826_812;
const SCALED_BASE: i32 = 8_941_203;

extern "C" {
    fn geq_word_define(p: i32, w: i32);
    fn do_marks(a: i16, l: i16, q: i32) -> bool;
    fn delete_token_ref(p: i32);
    fn find_sa_element(t: u16, n: i32, w: bool);
    fn new_null_box() -> i32;
    fn prune_page_top(p: i32, s: bool) -> i32;
    fn vpackage(p: i32, h: i32, m: i16, l: i32) -> i32;
    fn free_node(p: i32, s: i32);
    fn get_node(s: i32) -> i32;
    fn delete_glue_ref(p: i32);
    fn box_error(n: u8);
    fn error_here_with_diagnostic(message: *const libc::c_char) -> *mut std::ffi::c_void;
    fn capture_to_diagnostic(d: *mut std::ffi::c_void);
    fn begin_token_list(p: i32, t: u16);
    fn new_save_level(c: u8);
    fn normal_paragraph();
    fn scan_left_brace();
    fn flush_node_list(p: i32);
    fn ship_out(p: i32);
    fn push_nest();
    fn badness(t: i32, s: i32) -> i32;
    fn x_over_n(x: i32, n: i32) -> i32;
    fn vert_break(p: i32, h: i32, d: i32) -> i32;
    fn new_skip_param(n: i16) -> i32;
    fn new_spec(p: i32) -> i32;
    fn confusion(s: *const libc::c_char) -> !;
    fn error();
    fn print_esc_cstr(s: *const libc::c_char);
    fn print_int(n: i32);
    fn print_cstr(s: *const libc::c_char);
}

#[no_mangle]
pub unsafe extern "C" fn initialize_pagebuilder_variables() {
    PAGE_MAX_DEPTH = 0;
}

unsafe fn freeze_page_specs(s: u8) {
    set_page_contents(s);
    set_page_so_far(0, eqtb_b32s1(DIMEN_BASE + DIMEN_PAR__vsize));
    PAGE_MAX_DEPTH = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR__max_depth);
    set_page_so_far(7, 0);
    set_page_so_far(1, 0);
    set_page_so_far(2, 0);
    set_page_so_far(3, 0);
    set_page_so_far(4, 0);
    set_page_so_far(5, 0);
    set_page_so_far(6, 0);
    LEAST_PAGE_COST = MAX_HALFWORD;
}

unsafe fn ensure_vbox(n: u8) {
    let p = eqtb_b32s1(BOX_BASE + n as i32);
    if p == TEX_NULL {
        return;
    }
    if mem_b16s1(p) as i32 != HLIST_NODE {
        return;
    }
    error_here_with_diagnostic(c"Insertions can only be added to a vbox".as_ptr());
    capture_to_diagnostic(std::ptr::null_mut());
    set_help_ptr(3);
    set_help_line(2, c"Tut tut: You're trying to \\insert into a".as_ptr());
    set_help_line(1, c"\\box register that now contains an \\hbox.".as_ptr());
    set_help_line(
        0,
        c"Proceed, and I'll discard its present contents.".as_ptr(),
    );
    box_error(n);
}

unsafe fn fire_up(c: i32) {
    // 1048: Set output_penalty
    if mem_b16s1(BEST_PAGE_BREAK) as i32 == PENALTY_NODE {
        geq_word_define(
            INT_BASE + INT_PAR__output_penalty,
            mem_b32s1(BEST_PAGE_BREAK + 1),
        );
        set_mem_b32s1(BEST_PAGE_BREAK + 1, INF_PENALTY);
    } else {
        geq_word_define(INT_BASE + INT_PAR__output_penalty, INF_PENALTY);
    }

    // Update extended marks
    if sa_root(MARK_VAL) != TEX_NULL {
        if do_marks(FIRE_UP_INIT, 0, sa_root(MARK_VAL)) {
            set_sa_root(MARK_VAL, TEX_NULL);
        }
    }

    // 1047: Update top_mark / first_mark / bot_mark
    if cur_mark(BOT_MARK_CODE) != TEX_NULL {
        if cur_mark(TOP_MARK_CODE) != TEX_NULL {
            delete_token_ref(cur_mark(TOP_MARK_CODE));
        }
        set_cur_mark(TOP_MARK_CODE, cur_mark(BOT_MARK_CODE));
        set_mem_b32s0(
            cur_mark(TOP_MARK_CODE),
            mem_b32s0(cur_mark(TOP_MARK_CODE)) + 1,
        ); // TOKEN_LIST_ref_count++
        delete_token_ref(cur_mark(FIRST_MARK_CODE));
        set_cur_mark(FIRST_MARK_CODE, TEX_NULL);
    }

    // 1049: Put optimal page into box 255
    if c == BEST_PAGE_BREAK {
        BEST_PAGE_BREAK = TEX_NULL;
    }

    if eqtb_b32s1(BOX_BASE + 255) != TEX_NULL {
        // 1050: box 255 is not void — error and discard
        error_here_with_diagnostic(c"".as_ptr());
        print_esc_cstr(c"box".as_ptr());
        print_cstr(c"255 is not void".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(
            1,
            c"You shouldn't use \\box255 except in \\output routines.".as_ptr(),
        );
        set_help_line(
            0,
            c"Proceed, and I'll discard its present contents.".as_ptr(),
        );
        box_error(255);
    }

    set_insert_penalties(0);
    let save_split_top_skip = eqtb_b32s1(GLUE_BASE + GLUE_PAR__split_top_skip);
    let process_inserts =
        eqtb_b32s1(INT_BASE + INT_PAR__holding_inserts) <= 0 && !semantic_pagination_enabled();

    if process_inserts {
        // 1053: Prepare insertion boxes as queues
        let mut r = mem_b32s1(PAGE_INS_HEAD);
        while r != PAGE_INS_HEAD {
            if mem_b32s0(r + 2) != TEX_NULL {
                // PAGE_INS_NODE_best_ins_ptr != TEX_NULL
                let n = mem_b16s0(r) as u8; // NODE_subtype
                ensure_vbox(n);
                if eqtb_b32s1(BOX_BASE + n as i32) == TEX_NULL {
                    set_eqtb_b32s1(BOX_BASE + n as i32, new_null_box());
                }
                let mut p = eqtb_b32s1(BOX_BASE + n as i32) + 5;
                while mem_b32s1(p) != TEX_NULL {
                    p = mem_b32s1(p);
                }
                set_mem_b32s1(r + 2, p); // PAGE_INS_NODE_last_ins_ptr = p
            }
            r = mem_b32s1(r); // LLIST_link
        }
    }

    let mut hold_q = HOLD_HEAD;
    set_mem_b32s1(hold_q, TEX_NULL);
    let mut prev_p = PAGE_HEAD;
    let mut p = mem_b32s1(prev_p);

    // Walk page list up to best_page_break, processing insertions and marks
    while p != BEST_PAGE_BREAK {
        let reset_p = if mem_b16s1(p) as i32 == INS_NODE && process_inserts {
            // 1055: Process insertion node
            let mut r = mem_b32s1(PAGE_INS_HEAD);
            while mem_b16s0(r) != mem_b16s0(p) {
                // NODE_subtype(r) != NODE_subtype(p)
                r = mem_b32s1(r);
            }

            let wait = if mem_b32s0(r + 2) == TEX_NULL {
                // PAGE_INS_NODE_best_ins_ptr == TEX_NULL
                true
            } else {
                let mut s = mem_b32s1(r + 2); // PAGE_INS_NODE_last_ins_ptr
                set_mem_b32s1(s, mem_b32s0(p + 4)); // link(s) = INSERTION_NODE_ins_ptr(p)

                if mem_b32s0(r + 2) == p {
                    // 1056: best_ins_ptr == p — wrap up the box
                    let mut inner_wait = false;
                    if mem_b16s1(r) as i32 == SPLIT_UP {
                        if mem_b32s0(r + 1) == p && mem_b32s1(r + 1) != TEX_NULL {
                            // broken_ins == p && broken_ptr != TEX_NULL
                            while mem_b32s1(s) != mem_b32s1(r + 1) {
                                s = mem_b32s1(s);
                            }
                            set_mem_b32s1(s, TEX_NULL);
                            set_eqtb_b32s1(
                                GLUE_BASE + GLUE_PAR__split_top_skip,
                                mem_b32s1(p + 4), // INSERTION_NODE_split_top_ptr
                            );
                            set_mem_b32s0(
                                p + 4,
                                prune_page_top(mem_b32s1(r + 1), false), // broken_ptr
                            );
                            if mem_b32s0(p + 4) != TEX_NULL {
                                // INSERTION_NODE_ins_ptr != TEX_NULL
                                let tmp = vpackage(mem_b32s0(p + 4), 0, ADDITIONAL, MAX_HALFWORD);
                                set_mem_b32s1(p + 3, mem_b32s1(tmp + 3) + mem_b32s1(tmp + 2)); // BOX_height = BOX_height(tmp)+BOX_depth(tmp)
                                free_node(tmp, BOX_NODE_SIZE);
                                inner_wait = true;
                            }
                        }
                    }
                    set_mem_b32s0(r + 2, TEX_NULL); // best_ins_ptr = TEX_NULL
                    let n2 = mem_b16s0(r) as u8;
                    let box_lp = mem_b32s1(eqtb_b32s1(BOX_BASE + n2 as i32) + 5); // BOX_list_ptr
                    free_node(eqtb_b32s1(BOX_BASE + n2 as i32), BOX_NODE_SIZE);
                    set_eqtb_b32s1(
                        BOX_BASE + n2 as i32,
                        vpackage(box_lp, 0, ADDITIONAL, MAX_HALFWORD),
                    );
                    inner_wait
                } else {
                    // best_ins_ptr != p — advance s to end of list
                    while mem_b32s1(s) != TEX_NULL {
                        s = mem_b32s1(s);
                    }
                    set_mem_b32s1(r + 2, s); // PAGE_INS_NODE_last_ins_ptr = s
                    false
                }
            };

            // 1057: Append to hold list or delete node p
            set_mem_b32s1(prev_p, mem_b32s1(p)); // unlink p
            set_mem_b32s1(p, TEX_NULL);
            if wait {
                set_mem_b32s1(hold_q, p);
                hold_q = p;
                set_insert_penalties(insert_penalties() + 1);
            } else {
                delete_glue_ref(mem_b32s1(p + 4)); // INSERTION_NODE_split_top_ptr
                free_node(p, INS_NODE_SIZE);
            }
            true // reset p to prev_p
        } else {
            if mem_b16s1(p) as i32 == MARK_NODE {
                if mem_b32s0(p + 1) != 0 {
                    // MARK_NODE_class != 0
                    // 1618: Update current marks (extended)
                    find_sa_element(MARK_VAL as u16, mem_b32s0(p + 1), true);
                    if mem_b32s1(cur_ptr() + 1) == TEX_NULL {
                        // ETEX_MARK_sa_first_mark == TEX_NULL
                        set_mem_b32s1(cur_ptr() + 1, mem_b32s1(p + 1)); // = MARK_NODE_ptr
                        set_mem_b32s0(mem_b32s1(p + 1), mem_b32s0(mem_b32s1(p + 1)) + 1);
                        // TOKEN_LIST_ref_count++
                    }
                    if mem_b32s0(cur_ptr() + 2) != TEX_NULL {
                        // ETEX_MARK_sa_bot_mark != TEX_NULL
                        delete_token_ref(mem_b32s0(cur_ptr() + 2));
                    }
                    set_mem_b32s0(cur_ptr() + 2, mem_b32s1(p + 1)); // sa_bot_mark = MARK_NODE_ptr
                    set_mem_b32s0(mem_b32s1(p + 1), mem_b32s0(mem_b32s1(p + 1)) + 1);
                // TOKEN_LIST_ref_count++
                } else {
                    // 1051: Update first_mark and bot_mark (default)
                    if cur_mark(FIRST_MARK_CODE) == TEX_NULL {
                        set_cur_mark(FIRST_MARK_CODE, mem_b32s1(p + 1));
                        set_mem_b32s0(
                            cur_mark(FIRST_MARK_CODE),
                            mem_b32s0(cur_mark(FIRST_MARK_CODE)) + 1,
                        );
                    }
                    if cur_mark(BOT_MARK_CODE) != TEX_NULL {
                        delete_token_ref(cur_mark(BOT_MARK_CODE));
                    }
                    set_cur_mark(BOT_MARK_CODE, mem_b32s1(p + 1));
                    set_mem_b32s0(
                        cur_mark(BOT_MARK_CODE),
                        mem_b32s0(cur_mark(BOT_MARK_CODE)) + 1,
                    );
                }
            }
            false
        };

        if !reset_p {
            prev_p = p;
        }
        p = mem_b32s1(prev_p);
    }

    set_eqtb_b32s1(GLUE_BASE + GLUE_PAR__split_top_skip, save_split_top_skip);

    // 1052: Break page at p, put into box 255, put rest on contribution list
    if p != TEX_NULL {
        if mem_b32s1(CONTRIB_HEAD) == TEX_NULL {
            if nest_ptr() == 0 {
                set_cur_list_tail(page_tail());
            } else {
                set_nest_tail(0, page_tail());
            }
        }
        set_mem_b32s1(page_tail(), mem_b32s1(CONTRIB_HEAD));
        set_mem_b32s1(CONTRIB_HEAD, p);
        set_mem_b32s1(prev_p, TEX_NULL);
    }

    // Temporarily suppress badness errors
    let save_vbadness = eqtb_b32s1(INT_BASE + INT_PAR__vbadness);
    set_eqtb_b32s1(INT_BASE + INT_PAR__vbadness, INF_BAD);
    let save_vfuzz = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR__vfuzz);
    set_eqtb_b32s1(DIMEN_BASE + DIMEN_PAR__vfuzz, MAX_HALFWORD);
    set_eqtb_b32s1(
        BOX_BASE + 255,
        vpackage(mem_b32s1(PAGE_HEAD), BEST_SIZE, EXACTLY, PAGE_MAX_DEPTH),
    );
    set_eqtb_b32s1(INT_BASE + INT_PAR__vbadness, save_vbadness);
    set_eqtb_b32s1(DIMEN_BASE + DIMEN_PAR__vfuzz, save_vfuzz);

    if last_glue() != MAX_HALFWORD {
        delete_glue_ref(last_glue());
    }

    // 1026: Start a new current page
    set_page_contents(EMPTY);
    set_page_tail(PAGE_HEAD);
    set_mem_b32s1(PAGE_HEAD, TEX_NULL);
    set_last_glue(MAX_HALFWORD);
    set_last_penalty(0);
    set_last_kern(0);
    set_last_node_type(-1);
    set_page_so_far(7, 0);
    PAGE_MAX_DEPTH = 0;

    if hold_q != HOLD_HEAD {
        set_mem_b32s1(PAGE_HEAD, mem_b32s1(HOLD_HEAD));
        set_page_tail(hold_q);
    }

    // 1054: Delete page-insertion nodes
    let mut r = mem_b32s1(PAGE_INS_HEAD);
    while r != PAGE_INS_HEAD {
        let next = mem_b32s1(r);
        free_node(r, PAGE_INS_NODE_SIZE);
        r = next;
    }
    set_mem_b32s1(PAGE_INS_HEAD, PAGE_INS_HEAD);

    // Handle extended marks after new-page setup
    if sa_root(MARK_VAL) != TEX_NULL {
        if do_marks(FIRE_UP_DONE, 0, sa_root(MARK_VAL)) {
            set_sa_root(MARK_VAL, TEX_NULL);
        }
    }

    if cur_mark(TOP_MARK_CODE) != TEX_NULL && cur_mark(FIRST_MARK_CODE) == TEX_NULL {
        set_cur_mark(FIRST_MARK_CODE, cur_mark(TOP_MARK_CODE));
        set_mem_b32s0(
            cur_mark(TOP_MARK_CODE),
            mem_b32s0(cur_mark(TOP_MARK_CODE)) + 1,
        );
    }

    if eqtb_b32s1(LOCAL_BASE + LOCAL__output_routine) != TEX_NULL {
        if dead_cycles() >= eqtb_b32s1(INT_BASE + INT_PAR__max_dead_cycles) {
            // 1059: Too many dead cycles
            error_here_with_diagnostic(c"Output loop---".as_ptr());
            print_int(dead_cycles());
            print_cstr(c" consecutive dead cycles".as_ptr());
            capture_to_diagnostic(std::ptr::null_mut());
            set_help_ptr(3);
            set_help_line(
                2,
                c"I've concluded that your \\output is awry; it never does a".as_ptr(),
            );
            set_help_line(
                1,
                c"\\shipout, so I'm shipping \\box255 out myself. Next time".as_ptr(),
            );
            set_help_line(
                0,
                c"increase \\maxdeadcycles if you want me to be more patient!".as_ptr(),
            );
            error();
        } else {
            // 1060: Fire up the user's output routine
            set_output_active(true);
            set_dead_cycles(dead_cycles() + 1);
            push_nest();
            set_cur_list_mode(-VMODE);
            set_cur_list_aux_b32s1(IGNORE_DEPTH); // prev_depth
            set_cur_list_mode_line(-line());
            begin_token_list(eqtb_b32s1(LOCAL_BASE + LOCAL__output_routine), OUTPUT_TEXT);
            new_save_level(OUTPUT_GROUP);
            normal_paragraph();
            scan_left_brace();
            return;
        }
    }

    // 1058: Default output routine
    if mem_b32s1(PAGE_HEAD) != TEX_NULL {
        if mem_b32s1(CONTRIB_HEAD) == TEX_NULL {
            if nest_ptr() == 0 {
                set_cur_list_tail(page_tail());
            } else {
                set_nest_tail(0, page_tail());
            }
        } else {
            set_mem_b32s1(page_tail(), mem_b32s1(CONTRIB_HEAD));
        }
        set_mem_b32s1(CONTRIB_HEAD, mem_b32s1(PAGE_HEAD));
        set_mem_b32s1(PAGE_HEAD, TEX_NULL);
        set_page_tail(PAGE_HEAD);
    }

    flush_node_list(disc_ptr(LAST_BOX_CODE));
    set_disc_ptr(LAST_BOX_CODE, TEX_NULL);
    ship_out(eqtb_b32s1(BOX_BASE + 255));
    set_eqtb_b32s1(BOX_BASE + 255, TEX_NULL);
}

#[no_mangle]
pub unsafe extern "C" fn build_page() {
    if mem_b32s1(CONTRIB_HEAD) == TEX_NULL || output_active() {
        return;
    }

    'outer: loop {
        let p = mem_b32s1(CONTRIB_HEAD);

        // 1031: Update last_glue, last_penalty, last_kern, last_node_type
        if last_glue() != MAX_HALFWORD {
            delete_glue_ref(last_glue());
        }
        set_last_penalty(0);
        set_last_kern(0);
        let nt = mem_b16s1(p) as i32;
        set_last_node_type(nt + 1);

        if nt == GLUE_NODE {
            let gp = mem_b32s0(p + 1); // GLUE_NODE_glue_ptr
            set_last_glue(gp);
            set_mem_b32s1(gp, mem_b32s1(gp) + 1); // GLUE_SPEC_ref_count++
        } else {
            set_last_glue(MAX_HALFWORD);
            if nt == PENALTY_NODE {
                set_last_penalty(mem_b32s1(p + 1)); // PENALTY_NODE_penalty
            } else if nt == KERN_NODE {
                set_last_kern(mem_b32s1(p + 1)); // BOX_width
            }
        }

        // 1032/1035: Dispatch on node type
        let mut pi: i32 = INF_PENALTY;
        let mut need_contribute = false;
        let mut need_update_heights = false;
        let mut need_done1 = false;

        match nt {
            HLIST_NODE | VLIST_NODE | RULE_NODE => {
                if page_contents() < BOX_THERE {
                    // 1036: Initialize page, insert \topskip, continue loop
                    if page_contents() == EMPTY {
                        freeze_page_specs(BOX_THERE);
                    } else {
                        set_page_contents(BOX_THERE);
                    }
                    let q = new_skip_param(GLUE_PAR__top_skip as i16);
                    // temp_ptr = glue_ptr(q) after new_skip_param
                    if mem_b32s1(temp_ptr() + 1) > mem_b32s1(p + 3) {
                        // BOX_width(temp_ptr) > BOX_height(p)
                        set_mem_b32s1(temp_ptr() + 1, mem_b32s1(temp_ptr() + 1) - mem_b32s1(p + 3));
                    } else {
                        set_mem_b32s1(temp_ptr() + 1, 0);
                    }
                    set_mem_b32s1(q, p); // LLIST_link(q) = p
                    set_mem_b32s1(CONTRIB_HEAD, q);
                    if mem_b32s1(CONTRIB_HEAD) == TEX_NULL {
                        break 'outer;
                    }
                    continue 'outer;
                } else {
                    // 1037: Move box/rule to current page
                    set_page_so_far(1, page_so_far(1) + page_so_far(7) + mem_b32s1(p + 3));
                    set_page_so_far(7, mem_b32s1(p + 2)); // BOX_depth
                    need_contribute = true;
                }
            }
            WHATSIT_NODE => {
                // 1401
                let st = mem_b16s0(p) as i32;
                if st == PIC_NODE || st == PDF_NODE {
                    set_page_so_far(1, page_so_far(1) + page_so_far(7) + mem_b32s1(p + 3));
                    set_page_so_far(7, mem_b32s1(p + 2));
                }
                need_contribute = true;
            }
            GLUE_NODE => {
                if page_contents() < BOX_THERE {
                    need_done1 = true;
                } else if (mem_b16s1(page_tail()) as i32) < MATH_NODE {
                    // is_non_discardable_node(page_tail)
                    pi = 0;
                } else {
                    need_update_heights = true;
                }
            }
            KERN_NODE => {
                if page_contents() < BOX_THERE {
                    need_done1 = true;
                } else if mem_b32s1(p) == TEX_NULL {
                    // LLIST_link(p) == TEX_NULL — kern at list end; wait
                    return;
                } else if mem_b16s1(mem_b32s1(p)) as i32 == GLUE_NODE {
                    pi = 0;
                } else {
                    need_update_heights = true;
                }
            }
            PENALTY_NODE => {
                if page_contents() < BOX_THERE {
                    need_done1 = true;
                } else {
                    pi = mem_b32s1(p + 1); // PENALTY_NODE_penalty
                }
            }
            MARK_NODE => {
                need_contribute = true;
            }
            INS_NODE => {
                // 1043: Append insertion to current page
                if page_contents() == EMPTY {
                    freeze_page_specs(INSERTS_ONLY);
                }
                let n = mem_b16s0(p) as u8; // NODE_subtype(p)
                let mut r = PAGE_INS_HEAD;
                while n as u16 >= mem_b16s0(mem_b32s1(r)) {
                    r = mem_b32s1(r);
                }

                if mem_b16s0(r) as u8 != n {
                    // 1044: Create page-insertion node
                    let q2 = get_node(PAGE_INS_NODE_SIZE);
                    set_mem_b32s1(q2, mem_b32s1(r));
                    set_mem_b32s1(r, q2);
                    r = q2;
                    set_mem_b16s0(r, n as u16); // NODE_subtype = n
                    set_mem_b16s1(r, INSERTING); // NODE_type = INSERTING
                    ensure_vbox(n);
                    if eqtb_b32s1(BOX_BASE + n as i32) == TEX_NULL {
                        set_mem_b32s1(r + 3, 0); // BOX_height(r) = 0
                    } else {
                        let br = eqtb_b32s1(BOX_BASE + n as i32);
                        set_mem_b32s1(r + 3, mem_b32s1(br + 3) + mem_b32s1(br + 2));
                    }
                    set_mem_b32s0(r + 2, TEX_NULL); // best_ins_ptr = TEX_NULL
                    let q2 = eqtb_b32s1(SKIP_BASE + n as i32); // SKIP_REG(n)
                    let h = if eqtb_b32s1(COUNT_BASE + n as i32) == 1000 {
                        mem_b32s1(r + 3)
                    } else {
                        x_over_n(mem_b32s1(r + 3), 1000) * eqtb_b32s1(COUNT_BASE + n as i32)
                    };
                    set_page_so_far(0, page_so_far(0) - h - mem_b32s1(q2 + 1)); // BOX_width
                    let stretch_ord = mem_b16s1(q2) as usize; // GLUE_SPEC_stretch_order
                    set_page_so_far(
                        2 + stretch_ord,
                        page_so_far(2 + stretch_ord) + mem_b32s1(q2 + 2),
                    );
                    set_page_so_far(6, page_so_far(6) + mem_b32s1(q2 + 3)); // GLUE_SPEC_shrink
                    if mem_b16s0(q2) as i32 != NORMAL && mem_b32s1(q2 + 3) != 0 {
                        error_here_with_diagnostic(
                            c"Infinite glue shrinkage inserted from ".as_ptr(),
                        );
                        print_esc_cstr(c"skip".as_ptr());
                        print_int(n as i32);
                        capture_to_diagnostic(std::ptr::null_mut());
                        set_help_ptr(3);
                        set_help_line(
                            2,
                            c"The correction glue for page breaking with insertions".as_ptr(),
                        );
                        set_help_line(
                            1,
                            c"must have finite shrinkability. But you may proceed,".as_ptr(),
                        );
                        set_help_line(
                            0,
                            c"since the offensive shrinkability has been made finite.".as_ptr(),
                        );
                        error();
                    }
                }

                if mem_b16s1(r) as i32 == SPLIT_UP {
                    set_insert_penalties(
                        insert_penalties() + mem_b32s1(p + 1), // INSERTION_NODE_float_cost
                    );
                } else {
                    set_mem_b32s1(r + 2, p); // PAGE_INS_NODE_last_ins_ptr = p
                    let delta = page_so_far(0) - page_so_far(1) - page_so_far(7) + page_so_far(6);
                    let count_n = eqtb_b32s1(COUNT_BASE + n as i32);
                    let h = if count_n == 1000 {
                        mem_b32s1(p + 3) // BOX_height(p)
                    } else {
                        x_over_n(mem_b32s1(p + 3), 1000) * count_n
                    };

                    if (h <= 0 || h <= delta)
                        && mem_b32s1(p + 3) + mem_b32s1(r + 3) <= eqtb_b32s1(SCALED_BASE + n as i32)
                    {
                        set_page_so_far(0, page_so_far(0) - h);
                        set_mem_b32s1(r + 3, mem_b32s1(r + 3) + mem_b32s1(p + 3));
                    } else {
                        // 1045: Split the insertion
                        let w = if count_n <= 0 {
                            MAX_HALFWORD
                        } else {
                            let mut w = page_so_far(0) - page_so_far(1) - page_so_far(7);
                            if count_n != 1000 {
                                w = x_over_n(w, count_n) * 1000;
                            }
                            w
                        };
                        let max_h = eqtb_b32s1(SCALED_BASE + n as i32) - mem_b32s1(r + 3);
                        let w = if w > max_h { max_h } else { w };
                        let q2 = vert_break(mem_b32s0(p + 4), w, mem_b32s1(p + 2));
                        // BOX_height(r) += best_height_plus_depth
                        set_mem_b32s1(r + 3, mem_b32s1(r + 3) + best_height_plus_depth());
                        if count_n != 1000 {
                            set_best_height_plus_depth(
                                x_over_n(best_height_plus_depth(), 1000) * count_n,
                            );
                        }
                        set_page_so_far(0, page_so_far(0) - best_height_plus_depth());
                        set_mem_b16s1(r, SPLIT_UP as u16); // NODE_type = SPLIT_UP
                        set_mem_b32s1(r + 1, q2); // broken_ptr = q
                        set_mem_b32s0(r + 1, p); // broken_ins = p
                        if q2 == TEX_NULL {
                            set_insert_penalties(insert_penalties() + EJECT_PENALTY);
                        } else if mem_b16s1(q2) as i32 == PENALTY_NODE {
                            set_insert_penalties(insert_penalties() + mem_b32s1(q2 + 1));
                        }
                    }
                }
                need_contribute = true;
            }
            _ => confusion(c"page".as_ptr()),
        }

        // 1040: Break check — reached for GLUE/KERN/PENALTY that didn't goto contribute/done1
        if !need_contribute && !need_update_heights && !need_done1 {
            if pi < INF_PENALTY {
                // 1042: Compute badness b of current page
                let b = if page_so_far(1) < page_so_far(0) {
                    if page_so_far(3) != 0 || page_so_far(4) != 0 || page_so_far(5) != 0 {
                        0
                    } else {
                        badness(page_so_far(0) - page_so_far(1), page_so_far(2))
                    }
                } else if page_so_far(1) - page_so_far(0) > page_so_far(6) {
                    AWFUL_BAD
                } else {
                    badness(page_so_far(1) - page_so_far(0), page_so_far(6))
                };

                let mut c = if b < AWFUL_BAD {
                    if pi <= EJECT_PENALTY {
                        pi
                    } else if b < INF_BAD {
                        b + pi + insert_penalties()
                    } else {
                        100_000 // DEPLORABLE
                    }
                } else {
                    b
                };

                if insert_penalties() >= 10_000 {
                    c = MAX_HALFWORD;
                }

                if c <= LEAST_PAGE_COST {
                    BEST_PAGE_BREAK = p;
                    BEST_SIZE = page_so_far(0);
                    LEAST_PAGE_COST = c;
                    let mut r = mem_b32s1(PAGE_INS_HEAD);
                    while r != PAGE_INS_HEAD {
                        // best_ins_ptr = last_ins_ptr
                        set_mem_b32s0(r + 2, mem_b32s1(r + 2));
                        r = mem_b32s1(r);
                    }
                }

                if c == AWFUL_BAD || pi <= EJECT_PENALTY {
                    fire_up(p);
                    if output_active() {
                        return;
                    }
                    if mem_b32s1(CONTRIB_HEAD) == TEX_NULL {
                        break 'outer;
                    }
                    continue 'outer;
                }
            }

            // After break check: decide contribute vs update_heights
            if nt < GLUE_NODE || nt > KERN_NODE {
                need_contribute = true;
            } else {
                need_update_heights = true;
            }
        }

        // 1039: update_heights — update page measurements for glue/kern
        if need_update_heights {
            let q = if nt == KERN_NODE {
                p
            } else {
                let gp = mem_b32s0(p + 1); // GLUE_NODE_glue_ptr
                let stretch_ord = mem_b16s1(gp) as usize;
                set_page_so_far(
                    2 + stretch_ord,
                    page_so_far(2 + stretch_ord) + mem_b32s1(gp + 2),
                );
                set_page_so_far(6, page_so_far(6) + mem_b32s1(gp + 3));
                if mem_b16s0(gp) as i32 != NORMAL && mem_b32s1(gp + 3) != 0 {
                    error_here_with_diagnostic(
                        c"Infinite glue shrinkage found on current page".as_ptr(),
                    );
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(4);
                    set_help_line(
                        3,
                        c"The page about to be output contains some infinitely".as_ptr(),
                    );
                    set_help_line(
                        2,
                        c"shrinkable glue, e.g., `\\vss' or `\\vskip 0pt minus 1fil'.".as_ptr(),
                    );
                    set_help_line(
                        1,
                        c"Such glue doesn't belong there; but you can safely proceed,".as_ptr(),
                    );
                    set_help_line(
                        0,
                        c"since the offensive shrinkability has been made finite.".as_ptr(),
                    );
                    error();
                    let rr = new_spec(gp);
                    set_mem_b16s0(rr, NORMAL as u16); // GLUE_SPEC_shrink_order = NORMAL
                    delete_glue_ref(gp);
                    set_mem_b32s0(p + 1, rr); // GLUE_NODE_glue_ptr = rr
                    rr
                } else {
                    gp
                }
            };
            set_page_so_far(1, page_so_far(1) + page_so_far(7) + mem_b32s1(q + 1)); // BOX_width
            set_page_so_far(7, 0);
            need_contribute = true;
        }

        // 1038/1033: contribute — check depth limit, link p into page, goto done
        if need_contribute {
            if page_so_far(7) > PAGE_MAX_DEPTH {
                set_page_so_far(1, page_so_far(1) + page_so_far(7) - PAGE_MAX_DEPTH);
                set_page_so_far(7, PAGE_MAX_DEPTH);
            }
            set_mem_b32s1(page_tail(), p);
            set_page_tail(p);
            set_mem_b32s1(CONTRIB_HEAD, mem_b32s1(p));
            set_mem_b32s1(p, TEX_NULL);
            if mem_b32s1(CONTRIB_HEAD) == TEX_NULL {
                break 'outer;
            }
            continue 'outer;
        }

        // 1034: done1 — recycle node p (glue/kern/penalty before any box)
        if need_done1 {
            set_mem_b32s1(CONTRIB_HEAD, mem_b32s1(p));
            set_mem_b32s1(p, TEX_NULL);
            if eqtb_b32s1(INT_BASE + INT_PAR__saving_vdiscards) <= 0 {
                flush_node_list(p);
            } else {
                if disc_ptr(LAST_BOX_CODE) == TEX_NULL {
                    set_disc_ptr(LAST_BOX_CODE, p);
                } else {
                    set_mem_b32s1(disc_ptr(COPY_CODE), p);
                }
                set_disc_ptr(COPY_CODE, p);
            }
            if mem_b32s1(CONTRIB_HEAD) == TEX_NULL {
                break 'outer;
            }
            continue 'outer;
        }

        // done: re-evaluate while condition
        if mem_b32s1(CONTRIB_HEAD) == TEX_NULL {
            break 'outer;
        }
    }

    // After loop: update nest tail to point at contribution list head
    if nest_ptr() == 0 {
        set_cur_list_tail(CONTRIB_HEAD);
    } else {
        set_nest_tail(0, CONTRIB_HEAD);
    }
}
