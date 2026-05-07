const MAX_HLIST_STACK: i16 = 512;
const TEX_NULL: i32 = -0x0FFF_FFFF;
const HLIST_NODE: i32 = 0;
const VLIST_NODE: i32 = 1;
const RULE_NODE: i32 = 2;
const DELTA_NODE: i32 = RULE_NODE;
const LIGATURE_NODE: i32 = 6;
const INS_NODE: i32 = 3;
const MARK_NODE: i32 = 4;
const ADJUST_NODE: i32 = 5;
const DISC_NODE: i32 = 7;
const WHATSIT_NODE: i32 = 8;
const MATH_NODE: i32 = 9;
const GLUE_NODE: i32 = 10;
const KERN_NODE: i32 = 11;
const PENALTY_NODE: i32 = 12;
const LANGUAGE_NODE: u16 = 4;
const GLYPH_NODE: u16 = 42;
const PIC_NODE: u16 = 43;
const PDF_NODE: u16 = 44;
const NORMAL: u16 = 0;
const HOLD_HEAD: i32 = 4_999_995;
const ACTIVE_LIST: i32 = 4_999_992;
const LAST_ACTIVE: i32 = ACTIVE_LIST;
const NATIVE_NODE_SIZE: i32 = 6;
const SMALL_NODE_SIZE: i32 = 2;
const PASSIVE_NODE_SIZE: i32 = 2;
const ACTIVE_NODE_SIZE_NORMAL: i16 = 3;
const ACTIVE_NODE_SIZE_EXTENDED: i16 = 5;
const DELTA_NODE_SIZE: i32 = 7;
const TOO_BIG_CHAR: i32 = 65_536;
const LIG_TAG: u16 = 1;
const NON_ADDRESS: i32 = 0;
const NATIVE_WORD_NODE: u16 = 40;
const NATIVE_WORD_NODE_AT: u16 = 41;
const MIN_TRIE_OP: i32 = 0;
const HYPH_PRIME: i32 = 607;
const LC_CODE_BASE: i32 = 3_370_281;
const INT_BASE: i32 = 7_826_729;
const INT_PAR_PRETOLERANCE: i32 = 0;
const INT_PAR_TOLERANCE: i32 = 1;
const INT_PAR_LINE_PENALTY: i32 = 2;
const INT_PAR_HYPHEN_PENALTY: i32 = 3;
const INT_PAR_EX_HYPHEN_PENALTY: i32 = 4;
const INT_PAR_DOUBLE_HYPHEN_DEMERITS: i32 = 14;
const INT_PAR_FINAL_HYPHEN_DEMERITS: i32 = 15;
const INT_PAR_ADJ_DEMERITS: i32 = 16;
const INT_PAR_LOOSENESS: i32 = 19;
const INT_PAR_UC_HYPH: i32 = 38;
const INT_PAR_LAST_LINE_FIT: i32 = 62;
const INT_PAR_XETEX_USE_GLYPH_METRICS: i32 = 72;
const GLUE_BASE: i32 = 2_254_340;
const LOCAL_BASE: i32 = 2_254_871;
const ETEX_PEN_BASE: i32 = 2_255_140;
const DIMEN_BASE: i32 = 8_941_180;
const TEMP_HEAD: i32 = 4_999_996;
const ADJUST_HEAD: i32 = 4_999_994;
const PRE_ADJUST_HEAD: i32 = 4_999_985;
const GLUE_PAR_LEFT_SKIP: i16 = 7;
const GLUE_PAR_RIGHT_SKIP: i16 = 8;
const GLUE_PAR_PAR_FILL_SKIP: i16 = 14;
const INT_PAR_CLUB_PENALTY: i32 = 5;
const INT_PAR_WIDOW_PENALTY: i32 = 6;
const INT_PAR_DISPLAY_WIDOW_PENALTY: i32 = 7;
const INT_PAR_HANG_AFTER: i32 = 41;
const INT_PAR_BROKEN_PENALTY: i32 = 8;
const INT_PAR_INTER_LINE_PENALTY: i32 = 13;
const INT_PAR_XETEX_PROTRUDE_CHARS: i32 = 68;
const INT_PAR_TEXXET: i32 = 69;
const LOCAL_PAR_SHAPE: i32 = 0;
const DIMEN_PAR_HSIZE: i32 = 3;
const DIMEN_PAR_HANG_INDENT: i32 = 17;
const DIMEN_PAR_EMERGENCY_STRETCH: i32 = 20;
const ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES: i32 = 0;
const ETEX_PENALTIES_PAR_CLUB_PENALTIES: i32 = 1;
const ETEX_PENALTIES_PAR_WIDOW_PENALTIES: i32 = 2;
const ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES: i32 = 3;
const L_CODE: u16 = 4;
const END_M_CODE: u16 = 3;
const EXPLICIT: u16 = 1;
const SPACE_ADJUSTMENT: u16 = 3;
const EXACTLY: i16 = 0;
const ADDITIONAL: i16 = 1;
const UNHYPHENATED: i16 = 0;
const HYPHENATED: i16 = 1;
const VERY_LOOSE_FIT: usize = 0;
const TIGHT_FIT: usize = 3;
const DECENT_FIT: usize = 2;
const LOOSE_FIT: usize = 1;
const MAX_HALFWORD: i32 = 0x3FFF_FFFF;
const AWFUL_BAD: i32 = MAX_HALFWORD;
const INF_BAD: i32 = 10_000;
const INF_PENALTY: i32 = 10_000;
const EJECT_PENALTY: i32 = -10_000;

extern "C" {
    fn char_pw(p: i32, side: i16) -> i32;
    fn init_trie();
    fn prev_rightmost(s: i32, e: i32) -> i32;
    fn measure_native_node(node: *mut std::ffi::c_void, use_glyph_metrics: bool);
}

// translated from xetex-linebreak.c:77 (get_native_usv)
#[no_mangle]
pub unsafe extern "C" fn get_native_usv(p: i32, i: i32) -> i32 {
    let text = crate::globals::mem_as_u16_ptr(p + NATIVE_NODE_SIZE);
    let c = *text.add(i as usize);

    if (0xD800..0xDC00).contains(&c) {
        return 0x10000 + (c as i32 - 0xD800) * 0x400
            + *text.add(i as usize + 1) as i32
            - 0xDC00;
    }

    c as i32
}

#[inline]
fn is_char_node(p: i32) -> bool {
    p >= crate::globals::hi_mem_min()
}

#[inline]
fn font_character_width(f: i32, c: u16) -> i32 {
    let info = crate::globals::font_info_b16(
        crate::globals::char_base_entry(f) + unsafe { crate::scan::effective_char(true, f, c) },
    );
    crate::globals::font_info_b32s1(crate::globals::width_base_entry(f) + info.s3 as i32)
}

#[inline]
fn active_node_break_node(p: i32) -> i32 {
    crate::globals::mem_b32s1(p + 1)
}

#[inline]
fn set_active_node_break_node(p: i32, v: i32) {
    crate::globals::set_mem_b32s1(p + 1, v);
}

#[inline]
fn active_node_line_number(p: i32) -> i32 {
    crate::globals::mem_b32s0(p + 1)
}

#[inline]
fn set_active_node_line_number(p: i32, v: i32) {
    crate::globals::set_mem_b32s0(p + 1, v);
}

#[inline]
fn active_node_fitness(p: i32) -> u16 {
    crate::globals::mem_b16s0(p)
}

#[inline]
fn set_active_node_fitness(p: i32, v: usize) {
    crate::globals::set_mem_b16s0(p, v as u16);
}

#[inline]
fn active_node_total_demerits(p: i32) -> i32 {
    crate::globals::mem_b32s1(p + 2)
}

#[inline]
fn set_active_node_total_demerits(p: i32, v: i32) {
    crate::globals::set_mem_b32s1(p + 2, v);
}

#[inline]
fn active_node_shortfall(p: i32) -> i32 {
    crate::globals::mem_b32s1(p + 3)
}

#[inline]
fn set_active_node_shortfall(p: i32, v: i32) {
    crate::globals::set_mem_b32s1(p + 3, v);
}

#[inline]
fn active_node_glue(p: i32) -> i32 {
    crate::globals::mem_b32s1(p + 4)
}

#[inline]
fn set_active_node_glue(p: i32, v: i32) {
    crate::globals::set_mem_b32s1(p + 4, v);
}

#[inline]
fn node_type(p: i32) -> i32 {
    crate::globals::mem_b16s1(p) as i32
}

#[inline]
fn node_subtype(p: i32) -> u16 {
    crate::globals::mem_b16s0(p)
}

#[inline]
fn box_width(p: i32) -> i32 {
    crate::globals::mem_b32s1(p + 1)
}

#[inline]
fn set_box_width(p: i32, v: i32) {
    crate::globals::set_mem_b32s1(p + 1, v);
}

#[inline]
fn glue_ptr(p: i32) -> i32 {
    crate::globals::mem_b32s0(p + 1)
}

#[inline]
fn set_glue_ptr(p: i32, v: i32) {
    crate::globals::set_mem_b32s0(p + 1, v);
}

#[inline]
fn glue_leader_ptr(p: i32) -> i32 {
    crate::globals::mem_b32s1(p + 1)
}

#[inline]
fn glue_spec_stretch_order(p: i32) -> u16 {
    crate::globals::mem_b16s1(p)
}

#[inline]
fn glue_spec_shrink_order(p: i32) -> u16 {
    crate::globals::mem_b16s0(p)
}

#[inline]
fn glue_spec_stretch(p: i32) -> i32 {
    crate::globals::mem_b32s1(p + 2)
}

#[inline]
fn set_glue_spec_stretch(p: i32, v: i32) {
    crate::globals::set_mem_b32s1(p + 2, v);
}

#[inline]
fn glue_spec_shrink(p: i32) -> i32 {
    crate::globals::mem_b32s1(p + 3)
}

#[inline]
fn penalty_node_penalty(p: i32) -> i32 {
    crate::globals::mem_b32s1(p + 1)
}

#[inline]
fn set_penalty_node_penalty(p: i32, v: i32) {
    crate::globals::set_mem_b32s1(p + 1, v);
}

#[inline]
fn language_node_what_lang(p: i32) -> u8 {
    crate::globals::mem_b32s1(p + 1) as u8
}

#[inline]
fn language_node_what_lhm(p: i32) -> i32 {
    crate::globals::mem_b16s1(p + 1) as i32
}

#[inline]
fn language_node_what_rhm(p: i32) -> i32 {
    crate::globals::mem_b16s0(p + 1) as i32
}

#[inline]
fn ligature_node_lig_font(p: i32) -> i32 {
    crate::globals::mem_b16s1(p + 1) as i32
}

#[inline]
fn ligature_node_lig_char(p: i32) -> u16 {
    crate::globals::mem_b16s0(p + 1)
}

#[inline]
fn ligature_node_lig_ptr(p: i32) -> i32 {
    crate::globals::mem_b32s1(p + 1)
}

#[inline]
fn disc_pre_break(p: i32) -> i32 {
    crate::globals::mem_b32s0(p + 1)
}

#[inline]
fn disc_replace_count(p: i32) -> i32 {
    crate::globals::mem_b16s0(p) as i32
}

#[inline]
fn native_node_len(p: i32) -> i32 {
    crate::globals::mem_b16s1(p + 4) as i32
}

#[inline]
fn set_native_node_len(p: i32, v: i32) {
    crate::globals::set_mem_b16s1(p + 4, v as u16);
}

#[inline]
fn native_node_font(p: i32) -> i32 {
    crate::globals::mem_b16s2(p + 4) as i32
}

#[inline]
unsafe fn copy_native_tail(q: i32, ha: i32, start: i32) {
    let dst = crate::globals::mem_as_u16_ptr_mut(q + NATIVE_NODE_SIZE);
    let src = crate::globals::mem_as_u16_ptr(ha + NATIVE_NODE_SIZE);
    for i in start..native_node_len(ha) {
        *dst.add((i - start) as usize) = *src.add(i as usize);
    }
}

#[inline]
fn is_non_discardable_node(p: i32) -> bool {
    (crate::globals::mem_b16s1(p) as i32) < MATH_NODE
}

#[inline]
fn is_empty_box(p: i32) -> bool {
    use crate::globals::mem_b32s1;

    crate::globals::mem_b16s1(p) as i32 == HLIST_NODE
        && mem_b32s1(p + 1) == 0
        && mem_b32s1(p + 3) == 0
        && mem_b32s1(p + 2) == 0
        && mem_b32s1(p + 5) == TEX_NULL
}

#[inline]
fn ignorable_protrusion_node(p: i32) -> bool {
    use crate::globals::{mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1};

    !is_char_node(p)
        && match mem_b16s1(p) as i32 {
            INS_NODE | MARK_NODE | ADJUST_NODE | PENALTY_NODE => true,
            DISC_NODE => mem_b32s0(p + 1) == TEX_NULL
                && mem_b32s1(p + 1) == TEX_NULL
                && mem_b16s0(p) == 0,
            MATH_NODE => mem_b32s1(p + 1) == 0,
            KERN_NODE => mem_b32s1(p + 1) == 0 || mem_b16s0(p) == NORMAL,
            GLUE_NODE => mem_b32s0(p + 1) == 0,
            HLIST_NODE => mem_b32s1(p + 1) == 0
                && mem_b32s1(p + 3) == 0
                && mem_b32s1(p + 2) == 0
                && mem_b32s1(p + 5) == TEX_NULL,
            _ => false,
        }
}

// translated from xetex-linebreak.c:2827 (push_node)
#[no_mangle]
pub unsafe extern "C" fn push_node(p: i32) {
    use crate::globals::{hlist_stack_level, set_hlist_stack_entry, set_hlist_stack_level};

    let level = hlist_stack_level();
    if level > MAX_HLIST_STACK {
        crate::errors::pdf_error(c"push_node".as_ptr(), c"stack overflow".as_ptr());
    }

    set_hlist_stack_entry(level, p);
    set_hlist_stack_level(level + 1);
}

// translated from xetex-linebreak.c:2837 (pop_node)
#[no_mangle]
pub unsafe extern "C" fn pop_node() -> i32 {
    use crate::globals::{hlist_stack_entry, hlist_stack_level, set_hlist_stack_level};

    let level = hlist_stack_level() - 1;
    set_hlist_stack_level(level);
    if level < 0 {
        crate::errors::pdf_error(c"pop_node".as_ptr(), c"stack underflow (internal error)".as_ptr());
    }

    hlist_stack_entry(level)
}

// translated from xetex-linebreak.c:2692 (total_pw)
#[no_mangle]
pub unsafe extern "C" fn total_pw(q: i32, p: i32) -> i32 {
    use crate::globals::{first_p, global_prev_p, mem_b16s1, mem_b32s0, mem_b32s1};

    let mut l = if mem_b32s1(q + 1) == TEX_NULL {
        first_p()
    } else {
        mem_b32s1(mem_b32s1(q + 1) + 1)
    };

    let mut r = prev_rightmost(global_prev_p(), p);
    if p != TEX_NULL && mem_b16s1(p) as i32 == DISC_NODE && mem_b32s0(p + 1) != TEX_NULL {
        r = mem_b32s0(p + 1);
        while mem_b32s1(r) != TEX_NULL {
            r = mem_b32s1(r);
        }
    } else {
        r = find_protchar_right(l, r);
    }

    if l != TEX_NULL && mem_b16s1(l) as i32 == DISC_NODE {
        if mem_b32s1(l + 1) != TEX_NULL {
            l = mem_b32s1(l + 1);
        } else {
            let mut n = crate::globals::mem_b16s0(l);
            l = mem_b32s1(l);
            while n > 0 {
                if mem_b32s1(l) != TEX_NULL {
                    l = mem_b32s1(l);
                }
                n -= 1;
            }
            l = find_protchar_left(l, true);
        }
    } else {
        l = find_protchar_left(l, true);
    }

    char_pw(l, 0) + char_pw(r, 1)
}

// translated from xetex-linebreak.c:2731 (find_protchar_left)
#[no_mangle]
pub unsafe extern "C" fn find_protchar_left(mut l: i32, d: bool) -> i32 {
    use crate::globals::{hlist_stack_level, mem_b16s1, mem_b32s1, set_hlist_stack_level};

    if is_empty_box(l) {
        l = mem_b32s1(l);
    } else if d {
        while mem_b32s1(l) != TEX_NULL && !(is_char_node(l) || is_non_discardable_node(l)) {
            l = mem_b32s1(l);
        }
    }

    set_hlist_stack_level(0);
    let mut run = true;
    loop {
        let t = l;
        while run && mem_b16s1(l) as i32 == HLIST_NODE && mem_b32s1(l + 5) != TEX_NULL {
            push_node(l);
            l = mem_b32s1(l + 5);
        }

        while run && ignorable_protrusion_node(l) {
            while mem_b32s1(l) == TEX_NULL && hlist_stack_level() > 0 {
                l = pop_node();
            }

            if mem_b32s1(l) != TEX_NULL {
                l = mem_b32s1(l);
            } else if hlist_stack_level() == 0 {
                run = false;
            }
        }

        if t == l {
            break;
        }
    }

    l
}

// translated from xetex-linebreak.c:2778 (find_protchar_right)
#[no_mangle]
pub unsafe extern "C" fn find_protchar_right(mut l: i32, mut r: i32) -> i32 {
    use crate::globals::{hlist_stack_level, mem_b16s1, mem_b32s1, set_hlist_stack_level};

    if r == TEX_NULL {
        return TEX_NULL;
    }

    set_hlist_stack_level(0);
    let mut run = true;
    loop {
        let t = r;
        while run && mem_b16s1(r) as i32 == HLIST_NODE && mem_b32s1(r + 5) != TEX_NULL {
            push_node(l);
            push_node(r);
            l = mem_b32s1(r + 5);
            r = l;
            while mem_b32s1(r) != TEX_NULL {
                r = mem_b32s1(r);
            }
        }

        while run && ignorable_protrusion_node(r) {
            while r == l && hlist_stack_level() > 0 {
                r = pop_node();
                l = pop_node();
            }

            if r != l && r != TEX_NULL {
                r = prev_rightmost(l, r);
            } else if r == l && hlist_stack_level() == 0 {
                run = false;
            }
        }

        if t == r {
            break;
        }
    }

    r
}

// translated from xetex-linebreak.c:2377 (finite_shrink)
#[no_mangle]
pub unsafe extern "C" fn finite_shrink(p: i32) -> i32 {
    use crate::globals::{
        no_shrink_error_yet, set_help_line, set_help_ptr, set_mem_b16s0, set_no_shrink_error_yet,
    };

    if no_shrink_error_yet() {
        set_no_shrink_error_yet(false);
        crate::output::error_here_with_diagnostic(
            c"Infinite glue shrinkage found in a paragraph".as_ptr(),
        );
        crate::output::capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(5);
        set_help_line(4, c"The paragraph just ended includes some glue that has".as_ptr());
        set_help_line(3, c"infinite shrinkability, e.g., `\\hskip 0pt minus 1fil'.".as_ptr());
        set_help_line(2, c"Such glue doesn't belong there---it allows a paragraph".as_ptr());
        set_help_line(1, c"of any length to fit on one line. But it's safe to proceed,".as_ptr());
        set_help_line(0, c"since the offensive shrinkability has been made finite.".as_ptr());
        crate::errors::error();
    }

    let q = crate::mem::new_spec(p);
    set_mem_b16s0(q, NORMAL);
    crate::mem::delete_glue_ref(p);
    q
}

// translated from xetex-linebreak.c:2402 (reconstitute)
#[no_mangle]
pub unsafe extern "C" fn reconstitute(mut j: i16, n: i16, mut bchar: i32, mut hchar: i32) -> i16 {
    use crate::globals::{
        bchar_label_entry, char_base_entry, cur_l, cur_q, cur_r, font_info_b16, font_info_b32s1,
        hf, hyf_entry, init_lft, init_lig, init_list, kern_base_entry, lft_hit,
        lig_kern_base_entry, lig_stack, ligature_present, mem_b16s0, mem_b32s1, rt_hit,
        set_cur_l, set_cur_q, set_cur_r, set_hyphen_passed, set_lft_hit, set_lig_stack,
        set_ligature_present, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1,
        set_rt_hit,
    };
    use crate::mem::{free_node, get_avail, new_kern, new_lig_item, new_ligature};

    #[inline]
    fn odd(v: u8) -> bool {
        v % 2 == 1
    }

    #[inline]
    unsafe fn update_right(mut j: i16, n: i16, bchar: i32, hchar: i32) -> i32 {
        if j < n {
            set_cur_r(crate::globals::hu_entry((j + 1) as usize));
        } else {
            set_cur_r(bchar);
        }
        if odd(hyf_entry(j as usize)) {
            hchar
        } else {
            TOO_BIG_CHAR
        }
    }

    set_hyphen_passed(0);
    let mut t = HOLD_HEAD;
    let mut w = 0;
    set_mem_b32s1(HOLD_HEAD, TEX_NULL);
    set_cur_l(crate::globals::hu_entry(j as usize));
    set_cur_q(t);

    if j == 0 {
        set_ligature_present(init_lig());
        let mut p = init_list();
        if ligature_present() {
            set_lft_hit(init_lft());
        }
        while p > TEX_NULL {
            set_mem_b32s1(t, get_avail());
            t = mem_b32s1(t);
            set_mem_b16s1(t, hf() as u16);
            set_mem_b16s0(t, mem_b16s0(p));
            p = mem_b32s1(p);
        }
    } else if cur_l() < TOO_BIG_CHAR {
        set_mem_b32s1(t, get_avail());
        t = mem_b32s1(t);
        set_mem_b16s1(t, hf() as u16);
        set_mem_b16s0(t, cur_l() as u16);
    }

    set_lig_stack(TEX_NULL);
    let mut cur_rh = update_right(j, n, bchar, hchar);

    loop {
    'continue_: loop {
        let mut k;
        let mut q;

        if cur_l() == TOO_BIG_CHAR {
            k = bchar_label_entry(hf());
            if k == NON_ADDRESS {
                break 'continue_;
            }
            q = font_info_b16(k);
        } else {
            q = font_info_b16(
                char_base_entry(hf()) + crate::scan::effective_char(true, hf(), cur_l() as u16),
            );
            if q.s1 % 4 != LIG_TAG {
                break 'continue_;
            }
            k = lig_kern_base_entry(hf()) + q.s0 as i32;
            q = font_info_b16(k);
            if q.s3 > 128 {
                k = lig_kern_base_entry(hf()) + 256 * q.s1 as i32 + q.s0 as i32 + 32768 - 256 * 128;
                q = font_info_b16(k);
            }
        }

        let mut test_char = if cur_rh < TOO_BIG_CHAR { cur_rh } else { cur_r() };

        loop {
            if q.s2 as i32 == test_char && q.s3 <= 128 {
                if cur_rh < TOO_BIG_CHAR {
                    set_hyphen_passed(j);
                    hchar = TOO_BIG_CHAR;
                    cur_rh = TOO_BIG_CHAR;
                    continue 'continue_;
                }

                if hchar < TOO_BIG_CHAR && odd(hyf_entry(j as usize)) {
                    set_hyphen_passed(j);
                    hchar = TOO_BIG_CHAR;
                }

                if q.s1 < 128 {
                    if cur_l() == TOO_BIG_CHAR {
                        set_lft_hit(true);
                    }
                    if j == n && lig_stack() == TEX_NULL {
                        set_rt_hit(true);
                    }

                    match q.s1 {
                        1 | 5 => {
                            set_cur_l(q.s0 as i32);
                            set_ligature_present(true);
                        }
                        2 | 6 => {
                            set_cur_r(q.s0 as i32);
                            if lig_stack() > TEX_NULL {
                                set_mem_b16s0(lig_stack(), cur_r() as u16);
                            } else {
                                set_lig_stack(new_lig_item(cur_r() as u16));
                                if j == n {
                                    bchar = TOO_BIG_CHAR;
                                } else {
                                    let p = get_avail();
                                    set_mem_b32s1(lig_stack() + 1, p);
                                    set_mem_b16s0(p, crate::globals::hu_entry((j + 1) as usize) as u16);
                                    set_mem_b16s1(p, hf() as u16);
                                }
                            }
                        }
                        3 => {
                            set_cur_r(q.s0 as i32);
                            let p = lig_stack();
                            set_lig_stack(new_lig_item(cur_r() as u16));
                            set_mem_b32s1(lig_stack(), p);
                        }
                        7 | 11 => {
                            if ligature_present() {
                                let p = new_ligature(hf(), cur_l() as u16, mem_b32s1(cur_q()));
                                if lft_hit() {
                                    set_mem_b16s0(p, 2);
                                    set_lft_hit(false);
                                }
                                set_mem_b32s1(cur_q(), p);
                                t = p;
                                set_ligature_present(false);
                            }
                            set_cur_q(t);
                            set_cur_l(q.s0 as i32);
                            set_ligature_present(true);
                        }
                        _ => {
                            set_cur_l(q.s0 as i32);
                            set_ligature_present(true);
                            if lig_stack() > TEX_NULL {
                                if mem_b32s1(lig_stack() + 1) > TEX_NULL {
                                    set_mem_b32s1(t, mem_b32s1(lig_stack() + 1));
                                    t = mem_b32s1(t);
                                    j += 1;
                                }
                                let p = lig_stack();
                                set_lig_stack(mem_b32s1(p));
                                free_node(p, SMALL_NODE_SIZE);
                                if lig_stack() == TEX_NULL {
                                    cur_rh = update_right(j, n, bchar, hchar);
                                } else {
                                    set_cur_r(mem_b16s0(lig_stack()) as i32);
                                }
                            } else if j == n {
                                break 'continue_;
                            } else {
                                set_mem_b32s1(t, get_avail());
                                t = mem_b32s1(t);
                                set_mem_b16s1(t, hf() as u16);
                                set_mem_b16s0(t, cur_r() as u16);
                                j += 1;
                                cur_rh = update_right(j, n, bchar, hchar);
                            }
                        }
                    }

                    if q.s1 > 4 && q.s1 != 7 {
                        break 'continue_;
                    }
                    continue 'continue_;
                }

                w = font_info_b32s1(kern_base_entry(hf()) + 256 * q.s1 as i32 + q.s0 as i32);
                break 'continue_;
            }

            if q.s3 >= 128 {
                if cur_rh == TOO_BIG_CHAR {
                    break 'continue_;
                }
                cur_rh = TOO_BIG_CHAR;
                continue 'continue_;
            }

            k += q.s3 as i32 + 1;
            q = font_info_b16(k);
            test_char = if cur_rh < TOO_BIG_CHAR { cur_rh } else { cur_r() };
        }
    }

    if ligature_present() {
        let p = new_ligature(hf(), cur_l() as u16, mem_b32s1(cur_q()));
        if lft_hit() {
            set_mem_b16s0(p, 2);
            set_lft_hit(false);
        }
        if rt_hit() && lig_stack() == TEX_NULL {
            set_mem_b16s0(p, mem_b16s0(p) + 1);
            set_rt_hit(false);
        }
        set_mem_b32s1(cur_q(), p);
        t = p;
        set_ligature_present(false);
    }

    if w != 0 {
        set_mem_b32s1(t, new_kern(w));
        t = mem_b32s1(t);
        w = 0;
        set_mem_b32s0(t + 2, 0);
    }

    if lig_stack() > TEX_NULL {
        set_cur_q(t);
        set_cur_l(mem_b16s0(lig_stack()) as i32);
        set_ligature_present(true);
        if mem_b32s1(lig_stack() + 1) > TEX_NULL {
            set_mem_b32s1(t, mem_b32s1(lig_stack() + 1));
            t = mem_b32s1(t);
            j += 1;
        }
        let p = lig_stack();
        set_lig_stack(mem_b32s1(p));
        free_node(p, SMALL_NODE_SIZE);
        if lig_stack() == TEX_NULL {
            cur_rh = update_right(j, n, bchar, hchar);
        } else {
            set_cur_r(mem_b16s0(lig_stack()) as i32);
        }
        continue;
    }

    return j;
    }
}

#[inline]
unsafe fn set_native_metrics(p: i32) {
    measure_native_node(
        crate::globals::mem_node_addr(p),
        crate::globals::eqtb_b32s1(INT_BASE + INT_PAR_XETEX_USE_GLYPH_METRICS) > 0,
    );
}

// translated from xetex-linebreak.c:90 (line_break)
#[no_mangle]
pub unsafe extern "C" fn line_break(d: bool) {
    use crate::globals::{
        active_width, background, cur_lang, cur_list_head, cur_list_mode_line, cur_list_prev_graf,
        cur_list_tail, eqtb_b32s1, first_p, font_bchar_entry, font_in_short_display, global_prev_p,
        hc_entry, hi_mem_min, hn, hu_entry, hyph_index, hyph_start, hyphen_char_entry,
        l_hyf, last_special_line, mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1,
        minimal_demerits, passive, r_hyf, set_active_width, set_active_node_size,
        set_background, set_best_bet, set_best_line, set_cur_lang, set_cur_list_tail,
        set_cur_p, set_disc_width, set_do_last_line_fit, set_easy_line, set_fill_width,
        set_final_pass, set_first_indent, set_first_p, set_first_width, set_font_in_short_display,
        set_global_prev_p, set_ha, set_hb, set_hc_entry, set_hf, set_hn, set_hu_entry,
        set_hyf_bchar, set_hyf_char, set_hyph_index, set_init_l_hyf, set_init_r_hyf,
        set_l_hyf, set_last_special_line, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0,
        set_mem_b32s1, set_minimal_demerits, set_minimum_demerits, set_no_shrink_error_yet,
        set_pack_begin_line, set_passive, set_r_hyf, set_second_indent, set_second_width,
        set_threshold, set_xtx_ligature_present, trie_not_ready, trie_trc_entry, trie_trl_entry,
        trie_tro_entry,
    };

    fn int_par(n: i32) -> i32 {
        crate::globals::eqtb_b32s1(INT_BASE + n)
    }

    fn dimen_par(n: i32) -> i32 {
        crate::globals::eqtb_b32s1(DIMEN_BASE + n)
    }

    fn glue_par(n: i16) -> i32 {
        crate::globals::eqtb_b32s1(GLUE_BASE + n as i32)
    }

    fn lc_code(c: i32) -> i32 {
        crate::globals::eqtb_b32s1(LC_CODE_BASE + c)
    }

    fn set_current_hyph_index() {
        let lang = crate::globals::cur_lang() as i32;
        let start = crate::globals::hyph_start();
        if crate::globals::trie_trc_entry(start + lang) != crate::globals::cur_lang() as u16 {
            crate::globals::set_hyph_index(0);
        } else {
            crate::globals::set_hyph_index(crate::globals::trie_trl_entry(start + lang));
        }
    }

    fn add_active_width(k: usize, v: i32) {
        crate::globals::set_active_width(k, crate::globals::active_width(k) + v);
    }

    unsafe fn add_node_width_to_active(p: i32, confusion_name: *const libc::c_char) {
        if is_char_node(p) {
            let f = mem_b16s1(p) as i32;
            add_active_width(1, font_character_width(f, mem_b16s0(p)));
            return;
        }

        match node_type(p) {
            LIGATURE_NODE => {
                let f = ligature_node_lig_font(p);
                set_xtx_ligature_present(true);
                add_active_width(1, font_character_width(f, ligature_node_lig_char(p)));
            }
            HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                add_active_width(1, box_width(p));
            }
            WHATSIT_NODE => {
                let subtype = node_subtype(p);
                if subtype == NATIVE_WORD_NODE
                    || subtype == NATIVE_WORD_NODE_AT
                    || subtype == GLYPH_NODE
                    || subtype == PIC_NODE
                    || subtype == PDF_NODE
                {
                    add_active_width(1, box_width(p));
                } else {
                    crate::errors::confusion(confusion_name);
                }
            }
            _ => crate::errors::confusion(confusion_name),
        }
    }

    set_pack_begin_line(cur_list_mode_line());
    set_mem_b32s1(TEMP_HEAD, mem_b32s1(cur_list_head()));

    let tail = cur_list_tail();
    if is_char_node(tail) || node_type(tail) != GLUE_NODE {
        let p = crate::mem::new_penalty(INF_PENALTY);
        set_mem_b32s1(tail, p);
        set_cur_list_tail(p);
    } else {
        set_mem_b16s1(tail, PENALTY_NODE as u16);
        crate::mem::delete_glue_ref(glue_ptr(tail));
        crate::scan::flush_node_list(glue_leader_ptr(tail));
        set_penalty_node_penalty(tail, INF_PENALTY);
    }

    let last_line_fill = crate::mem::new_param_glue(GLUE_PAR_PAR_FILL_SKIP);
    set_mem_b32s1(cur_list_tail(), last_line_fill);
    set_init_l_hyf(cur_list_prev_graf() / 0x0400000);
    set_init_r_hyf((cur_list_prev_graf() / 65_536) % 64);
    let init_cur_lang = (cur_list_prev_graf() % 65_536) as u8;

    crate::mem::pop_nest();
    set_no_shrink_error_yet(true);

    if glue_spec_shrink_order(glue_par(GLUE_PAR_LEFT_SKIP)) != NORMAL
        && glue_spec_shrink(glue_par(GLUE_PAR_LEFT_SKIP)) != 0
    {
        let p = finite_shrink(glue_par(GLUE_PAR_LEFT_SKIP));
        crate::globals::set_eqtb_b32s1(GLUE_BASE + GLUE_PAR_LEFT_SKIP as i32, p);
    }
    if glue_spec_shrink_order(glue_par(GLUE_PAR_RIGHT_SKIP)) != NORMAL
        && glue_spec_shrink(glue_par(GLUE_PAR_RIGHT_SKIP)) != 0
    {
        let p = finite_shrink(glue_par(GLUE_PAR_RIGHT_SKIP));
        crate::globals::set_eqtb_b32s1(GLUE_BASE + GLUE_PAR_RIGHT_SKIP as i32, p);
    }

    let q = glue_par(GLUE_PAR_LEFT_SKIP);
    let r = glue_par(GLUE_PAR_RIGHT_SKIP);
    set_background(1, box_width(q) + box_width(r));
    for k in 2..=5 {
        set_background(k, 0);
    }
    set_background(2 + glue_spec_stretch_order(q) as usize, glue_spec_stretch(q));
    set_background(
        2 + glue_spec_stretch_order(r) as usize,
        background(2 + glue_spec_stretch_order(r) as usize) + glue_spec_stretch(r),
    );
    set_background(6, glue_spec_shrink(q) + glue_spec_shrink(r));

    set_do_last_line_fit(false);
    set_active_node_size(ACTIVE_NODE_SIZE_NORMAL);
    if int_par(INT_PAR_LAST_LINE_FIT) > 0 {
        let q = glue_ptr(last_line_fill);
        if glue_spec_stretch(q) > 0 && glue_spec_stretch_order(q) > NORMAL {
            if background(3) == 0 && background(4) == 0 && background(5) == 0 {
                set_do_last_line_fit(true);
                set_active_node_size(ACTIVE_NODE_SIZE_EXTENDED);
                set_fill_width(0, 0);
                set_fill_width(1, 0);
                set_fill_width(2, 0);
                set_fill_width((glue_spec_stretch_order(q) - 1) as usize, glue_spec_stretch(q));
            }
        }
    }

    set_minimum_demerits(AWFUL_BAD);
    for fit in [TIGHT_FIT, DECENT_FIT, LOOSE_FIT, VERY_LOOSE_FIT] {
        set_minimal_demerits(fit, AWFUL_BAD);
    }

    if eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE) == TEX_NULL {
        if dimen_par(DIMEN_PAR_HANG_INDENT) == 0 {
            set_last_special_line(0);
            set_second_width(dimen_par(DIMEN_PAR_HSIZE));
            set_second_indent(0);
        } else {
            set_last_special_line(int_par(INT_PAR_HANG_AFTER).abs());
            if int_par(INT_PAR_HANG_AFTER) < 0 {
                set_first_width(dimen_par(DIMEN_PAR_HSIZE) - dimen_par(DIMEN_PAR_HANG_INDENT).abs());
                set_first_indent(if dimen_par(DIMEN_PAR_HANG_INDENT) >= 0 {
                    dimen_par(DIMEN_PAR_HANG_INDENT)
                } else {
                    0
                });
                set_second_width(dimen_par(DIMEN_PAR_HSIZE));
                set_second_indent(0);
            } else {
                set_first_width(dimen_par(DIMEN_PAR_HSIZE));
                set_first_indent(0);
                set_second_width(dimen_par(DIMEN_PAR_HSIZE) - dimen_par(DIMEN_PAR_HANG_INDENT).abs());
                set_second_indent(if dimen_par(DIMEN_PAR_HANG_INDENT) >= 0 {
                    dimen_par(DIMEN_PAR_HANG_INDENT)
                } else {
                    0
                });
            }
        }
    } else {
        let par_shape = eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE);
        set_last_special_line(mem_b32s0(par_shape) - 1);
        set_second_width(mem_b32s1(par_shape + 2 * (last_special_line() + 1)));
        set_second_indent(mem_b32s1(par_shape + 2 * last_special_line() + 1));
    }

    if int_par(INT_PAR_LOOSENESS) == 0 {
        set_easy_line(last_special_line());
    } else {
        set_easy_line(MAX_HALFWORD);
    }

    set_threshold(int_par(INT_PAR_PRETOLERANCE));
    let mut second_pass;
    if crate::globals::threshold() >= 0 {
        second_pass = false;
        set_final_pass(false);
    } else {
        set_threshold(int_par(INT_PAR_TOLERANCE));
        second_pass = true;
        set_final_pass(dimen_par(DIMEN_PAR_EMERGENCY_STRETCH) <= 0);
    }

    'passes: loop {
        if crate::globals::threshold() > INF_BAD {
            set_threshold(INF_BAD);
        }

        if second_pass {
            if trie_not_ready() {
                init_trie();
            }
            set_cur_lang(init_cur_lang);
            set_l_hyf(crate::globals::init_l_hyf());
            set_r_hyf(crate::globals::init_r_hyf());
            set_current_hyph_index();
        }

        let q = crate::mem::get_node(crate::globals::active_node_size());
        set_mem_b16s1(q, UNHYPHENATED as u16);
        set_active_node_fitness(q, DECENT_FIT);
        set_mem_b32s1(q, LAST_ACTIVE);
        set_active_node_break_node(q, TEX_NULL);
        set_active_node_line_number(q, cur_list_prev_graf() + 1);
        set_active_node_total_demerits(q, 0);
        set_mem_b32s1(ACTIVE_LIST, q);
        if crate::globals::do_last_line_fit() {
            set_active_node_shortfall(q, 0);
            set_active_node_glue(q, 0);
        }

        for k in 1..=6 {
            set_active_width(k, background(k));
        }
        set_passive(TEX_NULL);
        set_font_in_short_display(0);
        set_cur_p(mem_b32s1(TEMP_HEAD));
        let mut auto_breaking = true;
        let mut prev_p = crate::globals::cur_p();
        set_global_prev_p(prev_p);
        set_first_p(prev_p);

        while crate::globals::cur_p() != TEX_NULL && mem_b32s1(ACTIVE_LIST) != LAST_ACTIVE {
            let mut cur = crate::globals::cur_p();

            if is_char_node(cur) {
                prev_p = cur;
                set_global_prev_p(cur);
                loop {
                    let f = mem_b16s1(cur) as i32;
                    add_active_width(1, font_character_width(f, mem_b16s0(cur)));
                    cur = mem_b32s1(cur);
                    set_cur_p(cur);
                    if !is_char_node(cur) {
                        break;
                    }
                }
            }

            cur = crate::globals::cur_p();
            match node_type(cur) {
                HLIST_NODE | VLIST_NODE | RULE_NODE => {
                    add_active_width(1, box_width(cur));
                }
                WHATSIT_NODE => {
                    let subtype = node_subtype(cur);
                    if subtype == LANGUAGE_NODE {
                        set_cur_lang(language_node_what_lang(cur));
                        set_l_hyf(language_node_what_lhm(cur));
                        set_r_hyf(language_node_what_rhm(cur));
                        set_current_hyph_index();
                    } else if subtype == NATIVE_WORD_NODE
                        || subtype == NATIVE_WORD_NODE_AT
                        || subtype == GLYPH_NODE
                        || subtype == PIC_NODE
                        || subtype == PDF_NODE
                    {
                        add_active_width(1, box_width(cur));
                    }
                }
                GLUE_NODE => {
                    if auto_breaking {
                        if is_char_node(prev_p)
                            || is_non_discardable_node(prev_p)
                            || (node_type(prev_p) == KERN_NODE && node_subtype(prev_p) != EXPLICIT)
                        {
                            try_break(0, UNHYPHENATED);
                        }
                    }

                    let mut q = glue_ptr(cur);
                    if glue_spec_shrink_order(q) != NORMAL && glue_spec_shrink(q) != 0 {
                        q = finite_shrink(q);
                        set_glue_ptr(cur, q);
                    }
                    add_active_width(1, box_width(q));
                    add_active_width(2 + glue_spec_stretch_order(q) as usize, glue_spec_stretch(q));
                    add_active_width(6, glue_spec_shrink(q));

                    if second_pass && auto_breaking {
                        let mut prev_s = cur;
                        let mut s = mem_b32s1(prev_s);

                        'hyphen_probe: {
                            if s == TEX_NULL {
                                break 'hyphen_probe;
                            }

                            let mut c: i32;
                            let mut l: i32 = 0;

                            loop {
                                if is_char_node(s) {
                                    c = mem_b16s0(s) as i32;
                                    set_hf(mem_b16s1(s) as i32);
                                } else if node_type(s) == LIGATURE_NODE {
                                    if ligature_node_lig_ptr(s) == TEX_NULL {
                                        prev_s = s;
                                        s = mem_b32s1(prev_s);
                                        continue;
                                    }
                                    q = ligature_node_lig_ptr(s);
                                    c = mem_b16s0(q) as i32;
                                    set_hf(mem_b16s1(q) as i32);
                                } else if node_type(s) == KERN_NODE && node_subtype(s) == NORMAL {
                                    prev_s = s;
                                    s = mem_b32s1(prev_s);
                                    continue;
                                } else if node_type(s) == MATH_NODE && node_subtype(s) >= L_CODE {
                                    prev_s = s;
                                    s = mem_b32s1(prev_s);
                                    continue;
                                } else if node_type(s) == WHATSIT_NODE {
                                    if node_subtype(s) == NATIVE_WORD_NODE
                                        || node_subtype(s) == NATIVE_WORD_NODE_AT
                                    {
                                        l = 0;
                                        while l < native_node_len(s) {
                                            c = get_native_usv(s, l);
                                            if lc_code(c) != 0 {
                                                set_hf(native_node_font(s));
                                                prev_s = s;
                                                if lc_code(c) == c || int_par(INT_PAR_UC_HYPH) > 0 {
                                                    break;
                                                } else {
                                                    break 'hyphen_probe;
                                                }
                                            }
                                            if c >= 65_536 {
                                                l += 1;
                                            }
                                            l += 1;
                                        }
                                        if l < native_node_len(s) {
                                            break;
                                        }
                                    }

                                    if node_subtype(s) == LANGUAGE_NODE {
                                        set_cur_lang(language_node_what_lang(s));
                                        set_l_hyf(language_node_what_lhm(s));
                                        set_r_hyf(language_node_what_rhm(s));
                                        set_current_hyph_index();
                                    }
                                    prev_s = s;
                                    s = mem_b32s1(prev_s);
                                    continue;
                                } else {
                                    break 'hyphen_probe;
                                }

                                if hyph_index() == 0 || c > 255 {
                                    set_hc_entry(0, lc_code(c));
                                } else if trie_trc_entry(hyph_index() + c) != c as u16 {
                                    set_hc_entry(0, 0);
                                } else {
                                    set_hc_entry(0, trie_tro_entry(hyph_index() + c));
                                }

                                if hc_entry(0) != 0 {
                                    if hc_entry(0) == c || int_par(INT_PAR_UC_HYPH) > 0 {
                                        break;
                                    } else {
                                        break 'hyphen_probe;
                                    }
                                }

                                prev_s = s;
                                s = mem_b32s1(prev_s);
                            }

                            set_hyf_char(hyphen_char_entry(crate::globals::hf()));
                            if crate::globals::hyf_char() < 0 || crate::globals::hyf_char() > TOO_BIG_CHAR - 1 {
                                break 'hyphen_probe;
                            }
                            set_ha(prev_s);
                            if l_hyf() + r_hyf() > crate::scan::max_hyphenatable_length() {
                                break 'hyphen_probe;
                            }

                            if crate::globals::ha() != TEX_NULL
                                && crate::globals::ha() < hi_mem_min()
                                && node_type(crate::globals::ha()) == WHATSIT_NODE
                                && (node_subtype(crate::globals::ha()) == NATIVE_WORD_NODE
                                    || node_subtype(crate::globals::ha()) == NATIVE_WORD_NODE_AT)
                            {
                                s = mem_b32s1(crate::globals::ha());
                                loop {
                                    if !is_char_node(s) {
                                        match node_type(s) {
                                            LIGATURE_NODE => {}
                                            KERN_NODE => {
                                                if node_subtype(s) != NORMAL {
                                                    break;
                                                }
                                            }
                                            WHATSIT_NODE | GLUE_NODE | PENALTY_NODE | INS_NODE | ADJUST_NODE
                                            | MARK_NODE => break,
                                            _ => break 'hyphen_probe,
                                        }
                                    }
                                    s = mem_b32s1(s);
                                }

                                set_hn(0);
                                'restart_native: loop {
                                    let len = native_node_len(crate::globals::ha());
                                    l = 0;
                                    while l < len {
                                        c = get_native_usv(crate::globals::ha(), l);
                                        if hyph_index() == 0 || c > 255 {
                                            set_hc_entry(0, lc_code(c));
                                        } else if trie_trc_entry(hyph_index() + c) != c as u16 {
                                            set_hc_entry(0, 0);
                                        } else {
                                            set_hc_entry(0, trie_tro_entry(hyph_index() + c));
                                        }

                                        if hc_entry(0) == 0 {
                                            if hn() > 0 {
                                                q = crate::scan::new_native_word_node(crate::globals::hf(), native_node_len(crate::globals::ha()) - l);
                                                set_mem_b16s0(q, node_subtype(crate::globals::ha()));
                                                copy_native_tail(q, crate::globals::ha(), l);
                                                set_native_metrics(q);
                                                set_mem_b32s1(q, mem_b32s1(crate::globals::ha()));
                                                set_mem_b32s1(crate::globals::ha(), q);
                                                set_native_node_len(crate::globals::ha(), l);
                                                set_native_metrics(crate::globals::ha());
                                                break 'restart_native;
                                            }
                                        } else if hn() == 0 && l > 0 {
                                            q = crate::scan::new_native_word_node(crate::globals::hf(), native_node_len(crate::globals::ha()) - l);
                                            set_mem_b16s0(q, node_subtype(crate::globals::ha()));
                                            copy_native_tail(q, crate::globals::ha(), l);
                                            set_native_metrics(q);
                                            set_mem_b32s1(q, mem_b32s1(crate::globals::ha()));
                                            set_mem_b32s1(crate::globals::ha(), q);
                                            set_native_node_len(crate::globals::ha(), l);
                                            set_native_metrics(crate::globals::ha());
                                            set_ha(mem_b32s1(crate::globals::ha()));
                                            continue 'restart_native;
                                        } else if hn() as i32 == crate::scan::max_hyphenatable_length() {
                                            break 'restart_native;
                                        } else {
                                            set_hn(hn() + 1);
                                            if c < 65_536 {
                                                set_hu_entry(hn() as usize, c);
                                                set_hc_entry(hn() as usize, hc_entry(0));
                                            } else {
                                                set_hu_entry(hn() as usize, (c - 65_536) / 1024 + 0xD800);
                                                set_hc_entry(hn() as usize, (hc_entry(0) - 65_536) / 1024 + 0xD800);
                                                set_hn(hn() + 1);
                                                set_hu_entry(hn() as usize, c % 1024 + 0xDC00);
                                                set_hc_entry(hn() as usize, hc_entry(0) % 1024 + 0xDC00);
                                                l += 1;
                                            }
                                            set_hyf_bchar(TOO_BIG_CHAR);
                                        }
                                        l += 1;
                                    }
                                    break 'restart_native;
                                }
                            } else {
                                set_hn(0);
                                'collect_word: loop {
                                    if is_char_node(s) {
                                        if mem_b16s1(s) as i32 != crate::globals::hf() {
                                            break;
                                        }
                                        set_hyf_bchar(mem_b16s0(s) as i32);
                                        c = crate::globals::hyf_bchar();
                                        if hyph_index() == 0 || c > 255 {
                                            set_hc_entry(0, lc_code(c));
                                        } else if trie_trc_entry(hyph_index() + c) != c as u16 {
                                            set_hc_entry(0, 0);
                                        } else {
                                            set_hc_entry(0, trie_tro_entry(hyph_index() + c));
                                        }
                                        if hc_entry(0) == 0
                                            || hc_entry(0) > crate::globals::max_hyph_char()
                                            || hn() as i32 == crate::scan::max_hyphenatable_length()
                                        {
                                            break;
                                        }
                                        set_hb(s);
                                        set_hn(hn() + 1);
                                        set_hu_entry(hn() as usize, c);
                                        set_hc_entry(hn() as usize, hc_entry(0));
                                        set_hyf_bchar(TOO_BIG_CHAR);
                                    } else if node_type(s) == LIGATURE_NODE {
                                        if ligature_node_lig_font(s) != crate::globals::hf() {
                                            break;
                                        }
                                        let mut j = hn();
                                        q = ligature_node_lig_ptr(s);
                                        if q > TEX_NULL {
                                            set_hyf_bchar(mem_b16s0(q) as i32);
                                        }
                                        while q > TEX_NULL {
                                            c = mem_b16s0(q) as i32;
                                            if hyph_index() == 0 || c > 255 {
                                                set_hc_entry(0, lc_code(c));
                                            } else if trie_trc_entry(hyph_index() + c) != c as u16 {
                                                set_hc_entry(0, 0);
                                            } else {
                                                set_hc_entry(0, trie_tro_entry(hyph_index() + c));
                                            }
                                            if hc_entry(0) == 0
                                                || hc_entry(0) > crate::globals::max_hyph_char()
                                                || j as i32 == crate::scan::max_hyphenatable_length()
                                            {
                                                break 'collect_word;
                                            }
                                            j += 1;
                                            set_hu_entry(j as usize, c);
                                            set_hc_entry(j as usize, hc_entry(0));
                                            q = mem_b32s1(q);
                                        }
                                        set_hb(s);
                                        set_hn(j);
                                        if odd(node_subtype(s) as u8) {
                                            set_hyf_bchar(font_bchar_entry(crate::globals::hf()));
                                        } else {
                                            set_hyf_bchar(TOO_BIG_CHAR);
                                        }
                                    } else if node_type(s) == KERN_NODE && node_subtype(s) == NORMAL {
                                        set_hb(s);
                                        set_hyf_bchar(font_bchar_entry(crate::globals::hf()));
                                    } else {
                                        break;
                                    }
                                    s = mem_b32s1(s);
                                }
                            }

                            if (hn() as i32) < l_hyf() + r_hyf() {
                                break 'hyphen_probe;
                            }

                            loop {
                                if !is_char_node(s) {
                                    match node_type(s) {
                                        LIGATURE_NODE => {}
                                        KERN_NODE => {
                                            if node_subtype(s) != NORMAL {
                                                break;
                                            }
                                        }
                                        WHATSIT_NODE | GLUE_NODE | PENALTY_NODE | INS_NODE | ADJUST_NODE
                                        | MARK_NODE => break,
                                        MATH_NODE => {
                                            if node_subtype(s) >= L_CODE {
                                                break;
                                            } else {
                                                break 'hyphen_probe;
                                            }
                                        }
                                        _ => break 'hyphen_probe,
                                    }
                                }
                                s = mem_b32s1(s);
                            }

                            hyphenate();
                        }
                    }
                }
                KERN_NODE => {
                    if node_subtype(cur) == EXPLICIT {
                        if !is_char_node(mem_b32s1(cur)) && auto_breaking && node_type(mem_b32s1(cur)) == GLUE_NODE {
                            try_break(0, UNHYPHENATED);
                        }
                        add_active_width(1, box_width(cur));
                    } else {
                        add_active_width(1, box_width(cur));
                    }
                }
                LIGATURE_NODE => {
                    let f = ligature_node_lig_font(cur);
                    set_xtx_ligature_present(true);
                    add_active_width(1, font_character_width(f, ligature_node_lig_char(cur)));
                }
                DISC_NODE => {
                    let mut s = disc_pre_break(cur);
                    set_disc_width(0);
                    if s == TEX_NULL {
                        try_break(int_par(INT_PAR_EX_HYPHEN_PENALTY), HYPHENATED);
                    } else {
                        set_disc_width(0);
                        while s != TEX_NULL {
                            let before = active_width(1);
                            add_node_width_to_active(s, c"disc3".as_ptr());
                            set_disc_width(crate::globals::disc_width() + active_width(1) - before);
                            set_active_width(1, before);
                            s = mem_b32s1(s);
                        }
                        add_active_width(1, crate::globals::disc_width());
                        try_break(int_par(INT_PAR_HYPHEN_PENALTY), HYPHENATED);
                        add_active_width(1, -crate::globals::disc_width());
                    }

                    let mut count = disc_replace_count(cur);
                    s = mem_b32s1(cur);
                    while count > 0 {
                        add_node_width_to_active(s, c"disc4".as_ptr());
                        count -= 1;
                        s = mem_b32s1(s);
                    }

                    prev_p = cur;
                    set_global_prev_p(cur);
                    set_cur_p(s);
                    continue;
                }
                MATH_NODE => {
                    if node_subtype(cur) < L_CODE {
                        auto_breaking = odd(node_subtype(cur) as u8);
                    }
                    if !is_char_node(mem_b32s1(cur)) && auto_breaking && node_type(mem_b32s1(cur)) == GLUE_NODE {
                        try_break(0, UNHYPHENATED);
                    }
                    add_active_width(1, box_width(cur));
                }
                PENALTY_NODE => {
                    try_break(penalty_node_penalty(cur), UNHYPHENATED);
                }
                MARK_NODE | INS_NODE | ADJUST_NODE => {}
                _ => crate::errors::confusion(c"paragraph".as_ptr()),
            }

            prev_p = cur;
            set_global_prev_p(cur);
            set_cur_p(mem_b32s1(cur));
        }

        if crate::globals::cur_p() == TEX_NULL {
            try_break(EJECT_PENALTY, HYPHENATED);
            if mem_b32s1(ACTIVE_LIST) != LAST_ACTIVE {
                let mut r = mem_b32s1(ACTIVE_LIST);
                let mut fewest_demerits = MAX_HALFWORD;
                loop {
                    if node_type(r) != DELTA_NODE && active_node_total_demerits(r) < fewest_demerits {
                        fewest_demerits = active_node_total_demerits(r);
                        set_best_bet(r);
                    }
                    r = mem_b32s1(r);
                    if r == LAST_ACTIVE {
                        break;
                    }
                }
                set_best_line(active_node_line_number(crate::globals::best_bet()));

                if int_par(INT_PAR_LOOSENESS) == 0 {
                    break 'passes;
                }

                let mut r = mem_b32s1(ACTIVE_LIST);
                let mut actual_looseness = 0;
                loop {
                    if node_type(r) != DELTA_NODE {
                        let line_diff = active_node_line_number(r) - crate::globals::best_line();
                        if (line_diff < actual_looseness && int_par(INT_PAR_LOOSENESS) <= line_diff)
                            || (line_diff > actual_looseness && int_par(INT_PAR_LOOSENESS) >= line_diff)
                        {
                            set_best_bet(r);
                            actual_looseness = line_diff;
                            fewest_demerits = active_node_total_demerits(r);
                        } else if line_diff == actual_looseness
                            && active_node_total_demerits(r) < fewest_demerits
                        {
                            set_best_bet(r);
                            fewest_demerits = active_node_total_demerits(r);
                        }
                    }
                    r = mem_b32s1(r);
                    if r == LAST_ACTIVE {
                        break;
                    }
                }

                set_best_line(active_node_line_number(crate::globals::best_bet()));
                if actual_looseness == int_par(INT_PAR_LOOSENESS) || crate::globals::final_pass() {
                    break 'passes;
                }
            }
        }

        let mut q = mem_b32s1(ACTIVE_LIST);
        while q != LAST_ACTIVE {
            set_cur_p(mem_b32s1(q));
            if node_type(q) == DELTA_NODE {
                crate::mem::free_node(q, DELTA_NODE_SIZE);
            } else {
                crate::mem::free_node(q, crate::globals::active_node_size());
            }
            q = crate::globals::cur_p();
        }

        q = passive();
        while q != TEX_NULL {
            set_cur_p(mem_b32s1(q));
            crate::mem::free_node(q, PASSIVE_NODE_SIZE);
            q = crate::globals::cur_p();
        }

        if !second_pass {
            set_threshold(int_par(INT_PAR_TOLERANCE));
            second_pass = true;
            set_final_pass(dimen_par(DIMEN_PAR_EMERGENCY_STRETCH) <= 0);
        } else {
            set_background(2, background(2) + dimen_par(DIMEN_PAR_EMERGENCY_STRETCH));
            set_final_pass(true);
        }
    }

    if crate::globals::do_last_line_fit() {
        if active_node_shortfall(crate::globals::best_bet()) == 0 {
            set_do_last_line_fit(false);
        } else {
            let q = crate::mem::new_spec(glue_ptr(last_line_fill));
            crate::mem::delete_glue_ref(glue_ptr(last_line_fill));
            set_box_width(
                q,
                box_width(q)
                    + active_node_shortfall(crate::globals::best_bet())
                    - active_node_glue(crate::globals::best_bet()),
            );
            set_glue_spec_stretch(q, 0);
            set_glue_ptr(last_line_fill, q);
        }
    }

    post_line_break(d);

    let mut q = mem_b32s1(ACTIVE_LIST);
    while q != ACTIVE_LIST {
        let next = mem_b32s1(q);
        if node_type(q) == DELTA_NODE {
            crate::mem::free_node(q, DELTA_NODE_SIZE);
        } else {
            crate::mem::free_node(q, crate::globals::active_node_size());
        }
        q = next;
    }

    q = passive();
    while q != TEX_NULL {
        let next = mem_b32s1(q);
        crate::mem::free_node(q, PASSIVE_NODE_SIZE);
        q = next;
    }

    set_pack_begin_line(0);
}

#[inline]
fn odd(v: u8) -> bool {
    v % 2 == 1
}

#[inline]
fn odd_i32(v: i32) -> bool {
    (v & 1) != 0
}

#[inline]
unsafe fn update_lr_stack(q: i32, lr_ptr: &mut i32) {
    use crate::globals::{
        avail, mem_b16s0, mem_b32s0, mem_b32s1, set_avail, set_mem_b32s0, set_mem_b32s1,
    };

    if odd_i32(mem_b16s0(q) as i32) {
        if *lr_ptr != TEX_NULL
            && mem_b32s0(*lr_ptr)
                == L_CODE as i32 * (mem_b16s0(q) as i32 / L_CODE as i32) + END_M_CODE as i32
        {
            let temp_ptr = *lr_ptr;
            *lr_ptr = mem_b32s1(temp_ptr);
            set_mem_b32s1(temp_ptr, avail());
            set_avail(temp_ptr);
        }
    } else {
        let temp_ptr = crate::mem::get_avail();
        set_mem_b32s0(
            temp_ptr,
            L_CODE as i32 * (mem_b16s0(q) as i32 / L_CODE as i32) + END_M_CODE as i32,
        );
        set_mem_b32s1(temp_ptr, *lr_ptr);
        *lr_ptr = temp_ptr;
    }
}

// translated from xetex-linebreak.c:971 (post_line_break)
#[no_mangle]
pub unsafe extern "C" fn post_line_break(d: bool) {
    use crate::globals::{
        adjust_tail, best_bet, best_line, cur_list_e_tex_aux, cur_list_prev_graf, cur_list_tail,
        eqtb_b32s1, first_indent, first_width, hi_mem_min, just_box, last_leftmost_char,
        last_rightmost_char, last_special_line, mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1,
        pre_adjust_tail, second_indent, second_width, semantic_pagination_enabled, set_adjust_tail,
        set_cur_list_e_tex_aux, set_cur_list_prev_graf, set_cur_list_tail, set_cur_p,
        set_just_box, set_mem_b16s0, set_mem_b32s0, set_mem_b32s1, set_pre_adjust_tail,
    };

    let mut lr_ptr = cur_list_e_tex_aux();

    let mut q = mem_b32s1(best_bet() + 1);
    set_cur_p(TEX_NULL);
    loop {
        let r = q;
        q = mem_b32s0(q + 1);
        set_mem_b32s0(r + 1, crate::globals::cur_p());
        set_cur_p(r);
        if q == TEX_NULL {
            break;
        }
    }

    let mut cur_line = cur_list_prev_graf() + 1;
    loop {
        if eqtb_b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
            q = mem_b32s1(TEMP_HEAD);

            if lr_ptr != TEX_NULL {
                let mut temp_ptr = lr_ptr;
                let mut r = q;
                loop {
                    let s = crate::mem::new_math(0, (mem_b32s0(temp_ptr) - 1) as i16);
                    set_mem_b32s1(s, r);
                    r = s;
                    temp_ptr = mem_b32s1(temp_ptr);
                    if temp_ptr == TEX_NULL {
                        break;
                    }
                }
                set_mem_b32s1(TEMP_HEAD, r);
            }

            while q != mem_b32s1(crate::globals::cur_p() + 1) {
                if q < hi_mem_min() && mem_b16s1(q) as i32 == MATH_NODE {
                    update_lr_stack(q, &mut lr_ptr);
                }
                q = mem_b32s1(q);
            }
        }

        q = mem_b32s1(crate::globals::cur_p() + 1);
        let mut disc_break = false;
        let mut post_disc_break = false;
        let mut glue_break = false;

        if q == TEX_NULL {
            q = TEMP_HEAD;
            while mem_b32s1(q) != TEX_NULL {
                q = mem_b32s1(q);
            }
        } else if mem_b16s1(q) as i32 == GLUE_NODE {
            crate::mem::delete_glue_ref(mem_b32s0(q + 1));
            let right_skip = eqtb_b32s1(GLUE_BASE + GLUE_PAR_RIGHT_SKIP as i32);
            set_mem_b32s0(q + 1, right_skip);
            set_mem_b16s0(q, GLUE_PAR_RIGHT_SKIP as u16 + 1);
            set_mem_b32s1(right_skip, mem_b32s1(right_skip) + 1);
            glue_break = true;
        } else if mem_b16s1(q) as i32 == DISC_NODE {
            let mut t = mem_b16s0(q);
            let mut r;

            if t == 0 {
                r = mem_b32s1(q);
            } else {
                r = q;
                while t > 1 {
                    r = mem_b32s1(r);
                    t -= 1;
                }

                let s = mem_b32s1(r);
                r = mem_b32s1(s);
                set_mem_b32s1(s, TEX_NULL);
                crate::scan::flush_node_list(mem_b32s1(q));
                set_mem_b16s0(q, 0);
            }

            if mem_b32s1(q + 1) != TEX_NULL {
                let mut s = mem_b32s1(q + 1);
                while mem_b32s1(s) != TEX_NULL {
                    s = mem_b32s1(s);
                }
                set_mem_b32s1(s, r);
                r = mem_b32s1(q + 1);
                set_mem_b32s1(q + 1, TEX_NULL);
                post_disc_break = true;
            }

            if mem_b32s0(q + 1) != TEX_NULL {
                let mut s = mem_b32s0(q + 1);
                set_mem_b32s1(q, s);
                while mem_b32s1(s) != TEX_NULL {
                    s = mem_b32s1(s);
                }
                set_mem_b32s0(q + 1, TEX_NULL);
                q = s;
            }

            set_mem_b32s1(q, r);
            disc_break = true;
        } else if mem_b16s1(q) as i32 == KERN_NODE {
            set_mem_b32s1(q + 1, 0);
        } else if mem_b16s1(q) as i32 == MATH_NODE {
            set_mem_b32s1(q + 1, 0);

            if eqtb_b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
                update_lr_stack(q, &mut lr_ptr);
            }
        }

        if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_PROTRUDE_CHARS) > 0 {
            let mut p;
            let ptmp;
            if disc_break && (is_char_node(q) || mem_b16s1(q) as i32 != DISC_NODE) {
                p = q;
                ptmp = p;
            } else {
                p = prev_rightmost(mem_b32s1(TEMP_HEAD), q);
                ptmp = p;
                p = find_protchar_right(mem_b32s1(TEMP_HEAD), p);
            }

            let w = char_pw(p, 1);
            if w != 0 {
                let k = crate::mem::new_margin_kern(-w, last_rightmost_char(), 1);
                set_mem_b32s1(k, mem_b32s1(ptmp));
                set_mem_b32s1(ptmp, k);
                if ptmp == q {
                    q = mem_b32s1(q);
                }
            }
        }

        if !glue_break {
            let r = crate::mem::new_param_glue(GLUE_PAR_RIGHT_SKIP);
            set_mem_b32s1(r, mem_b32s1(q));
            set_mem_b32s1(q, r);
            q = r;
        }

        if eqtb_b32s1(INT_BASE + INT_PAR_TEXXET) > 0 && lr_ptr != TEX_NULL {
            let mut s = TEMP_HEAD;
            let mut r = mem_b32s1(s);
            while r != q {
                s = r;
                r = mem_b32s1(s);
            }

            r = lr_ptr;
            while r != TEX_NULL {
                let temp_ptr = crate::mem::new_math(0, mem_b32s0(r) as i16);
                set_mem_b32s1(s, temp_ptr);
                s = temp_ptr;
                r = mem_b32s1(r);
            }
            set_mem_b32s1(s, q);
        }

        let r = mem_b32s1(q);
        set_mem_b32s1(q, TEX_NULL);
        q = mem_b32s1(TEMP_HEAD);
        set_mem_b32s1(TEMP_HEAD, r);

        if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_PROTRUDE_CHARS) > 0 {
            let p = find_protchar_left(q, false);
            let w = char_pw(p, 0);
            if w != 0 {
                let k = crate::mem::new_margin_kern(-w, last_leftmost_char(), 0);
                set_mem_b32s1(k, q);
                q = k;
            }
        }

        if eqtb_b32s1(GLUE_BASE + GLUE_PAR_LEFT_SKIP as i32) != 0 {
            let r = crate::mem::new_param_glue(GLUE_PAR_LEFT_SKIP);
            set_mem_b32s1(r, q);
            q = r;
        }

        let (cur_width, cur_indent) = if cur_line > last_special_line() {
            (second_width(), second_indent())
        } else if eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE) == TEX_NULL {
            (first_width(), first_indent())
        } else {
            (
                mem_b32s1(eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE) + 2 * cur_line),
                mem_b32s1(eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE) + 2 * cur_line - 1),
            )
        };

        set_adjust_tail(ADJUST_HEAD);
        set_pre_adjust_tail(PRE_ADJUST_HEAD);

        if semantic_pagination_enabled() {
            set_just_box(crate::scan::hpack(q, 0, ADDITIONAL));
        } else {
            set_just_box(crate::scan::hpack(q, cur_width, EXACTLY));
        }
        set_mem_b32s1(just_box() + 4, cur_indent);

        if PRE_ADJUST_HEAD != pre_adjust_tail() {
            set_mem_b32s1(cur_list_tail(), mem_b32s1(PRE_ADJUST_HEAD));
            set_cur_list_tail(pre_adjust_tail());
        }

        set_pre_adjust_tail(TEX_NULL);
        crate::scan::append_to_vlist(just_box());

        if ADJUST_HEAD != adjust_tail() {
            set_mem_b32s1(cur_list_tail(), mem_b32s1(ADJUST_HEAD));
            set_cur_list_tail(adjust_tail());
        }

        set_adjust_tail(TEX_NULL);

        if cur_line + 1 != best_line() {
            q = eqtb_b32s1(ETEX_PEN_BASE + ETEX_PENALTIES_PAR_INTER_LINE_PENALTIES);
            let mut pen;
            if q != TEX_NULL {
                let mut r = cur_line;
                if r > mem_b32s1(q + 1) {
                    r = mem_b32s1(q + 1);
                }
                pen = mem_b32s1(q + r + 1);
            } else {
                pen = eqtb_b32s1(INT_BASE + INT_PAR_INTER_LINE_PENALTY);
            }

            q = eqtb_b32s1(ETEX_PEN_BASE + ETEX_PENALTIES_PAR_CLUB_PENALTIES);
            if q != TEX_NULL {
                let mut r = cur_line - cur_list_prev_graf();
                if r > mem_b32s1(q + 1) {
                    r = mem_b32s1(q + 1);
                }
                pen += mem_b32s1(q + r + 1);
            } else if cur_line == cur_list_prev_graf() + 1 {
                pen += eqtb_b32s1(INT_BASE + INT_PAR_CLUB_PENALTY);
            }

            q = if d {
                eqtb_b32s1(ETEX_PEN_BASE + ETEX_PENALTIES_PAR_DISPLAY_WIDOW_PENALTIES)
            } else {
                eqtb_b32s1(ETEX_PEN_BASE + ETEX_PENALTIES_PAR_WIDOW_PENALTIES)
            };

            if q != TEX_NULL {
                let mut r = best_line() - cur_line - 1;
                if r > mem_b32s1(q + 1) {
                    r = mem_b32s1(q + 1);
                }
                pen += mem_b32s1(q + r + 1);
            } else if cur_line + 2 == best_line() {
                if d {
                    pen += eqtb_b32s1(INT_BASE + INT_PAR_DISPLAY_WIDOW_PENALTY);
                } else {
                    pen += eqtb_b32s1(INT_BASE + INT_PAR_WIDOW_PENALTY);
                }
            }

            if disc_break {
                pen += eqtb_b32s1(INT_BASE + INT_PAR_BROKEN_PENALTY);
            }

            if pen != 0 {
                let r = crate::mem::new_penalty(pen);
                set_mem_b32s1(cur_list_tail(), r);
                set_cur_list_tail(r);
            }
        }

        cur_line += 1;
        set_cur_p(mem_b32s0(crate::globals::cur_p() + 1));

        if crate::globals::cur_p() != TEX_NULL && !post_disc_break {
            let mut r = TEMP_HEAD;
            loop {
                q = mem_b32s1(r);

                if q == mem_b32s1(crate::globals::cur_p() + 1)
                    || is_char_node(q)
                    || is_non_discardable_node(q)
                    || (mem_b16s1(q) as i32 == KERN_NODE
                        && mem_b16s0(q) != EXPLICIT
                        && mem_b16s0(q) != SPACE_ADJUSTMENT)
                {
                    break;
                }

                r = q;

                if mem_b16s1(q) as i32 == MATH_NODE && eqtb_b32s1(INT_BASE + INT_PAR_TEXXET) > 0 {
                    update_lr_stack(q, &mut lr_ptr);
                }
            }

            if r != TEMP_HEAD {
                set_mem_b32s1(r, TEX_NULL);
                crate::scan::flush_node_list(mem_b32s1(TEMP_HEAD));
                set_mem_b32s1(TEMP_HEAD, q);
            }
        }

        if crate::globals::cur_p() == TEX_NULL {
            break;
        }
    }

    if cur_line != best_line() || mem_b32s1(TEMP_HEAD) != TEX_NULL {
        crate::errors::confusion(c"line breaking".as_ptr());
    }

    set_cur_list_prev_graf(best_line() - 1);
    set_cur_list_e_tex_aux(lr_ptr);
}

// translated from xetex-linebreak.c:978 (try_break)
#[no_mangle]
pub unsafe extern "C" fn try_break(mut pi: i32, break_type: i16) {
    use crate::globals::{
        active_node_size, active_width, background, best_pl_glue, best_pl_line, best_pl_short,
        best_place, break_width, cur_active_width, cur_p, disc_width, do_last_line_fit,
        easy_line, eqtb_b32s1, fill_width, final_pass, first_width, last_special_line,
        mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1, minimal_demerits, minimum_demerits,
        passive, semantic_pagination_enabled, set_active_width, set_best_pl_glue,
        set_best_pl_line, set_best_pl_short, set_best_place, set_break_width,
        set_cur_active_width, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1,
        set_minimal_demerits, set_minimum_demerits, set_passive, set_xtx_ligature_present,
        threshold,
    };

    if semantic_pagination_enabled() && cur_p() != TEX_NULL {
        return;
    }

    if pi.abs() >= INF_PENALTY {
        if pi > 0 {
            return;
        }
        pi = EJECT_PENALTY;
    }

    let mut no_break_yet = true;
    let mut prev_r = ACTIVE_LIST;
    let mut prev_prev_r = TEX_NULL;
    let mut old_l = 0;
    let mut line_width = 0;
    let mut g = 0;

    for k in 1..=6 {
        set_cur_active_width(k, active_width(k));
    }

    loop {
        let mut r = mem_b32s1(prev_r);

        if mem_b16s1(r) as i32 == DELTA_NODE {
            set_cur_active_width(1, cur_active_width(1) + mem_b32s1(r + 1));
            set_cur_active_width(2, cur_active_width(2) + mem_b32s1(r + 2));
            set_cur_active_width(3, cur_active_width(3) + mem_b32s1(r + 3));
            set_cur_active_width(4, cur_active_width(4) + mem_b32s1(r + 4));
            set_cur_active_width(5, cur_active_width(5) + mem_b32s1(r + 5));
            set_cur_active_width(6, cur_active_width(6) + mem_b32s1(r + 6));
            prev_prev_r = prev_r;
            prev_r = r;
            continue;
        }

        let l = active_node_line_number(r);
        if l > old_l {
            if minimum_demerits() < AWFUL_BAD && (old_l != easy_line() || r == LAST_ACTIVE) {
                if no_break_yet {
                    no_break_yet = false;
                    for k in 1..=6 {
                        set_break_width(k, background(k));
                    }
                    let mut s = cur_p();

                    if break_type > UNHYPHENATED {
                        if cur_p() != TEX_NULL {
                            let mut t = mem_b16s0(cur_p()) as i32;
                            let mut v = cur_p();
                            s = mem_b32s1(cur_p() + 1);

                            while t > 0 {
                                t -= 1;
                                v = mem_b32s1(v);
                                if is_char_node(v) {
                                    let f = mem_b16s1(v) as i32;
                                    set_break_width(
                                        1,
                                        break_width(1) - font_character_width(f, mem_b16s0(v)),
                                    );
                                } else {
                                    match mem_b16s1(v) as i32 {
                                        LIGATURE_NODE => {
                                            let f = mem_b16s1(v + 1) as i32;
                                            set_xtx_ligature_present(true);
                                            set_break_width(
                                                1,
                                                break_width(1)
                                                    - font_character_width(f, mem_b16s0(v + 1)),
                                            );
                                        }
                                        HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                                            set_break_width(1, break_width(1) - mem_b32s1(v + 1));
                                        }
                                        WHATSIT_NODE => {
                                            let sub = mem_b16s0(v);
                                            if sub == NATIVE_WORD_NODE
                                                || sub == NATIVE_WORD_NODE_AT
                                                || sub == GLYPH_NODE
                                                || sub == PIC_NODE
                                                || sub == PDF_NODE
                                            {
                                                set_break_width(1, break_width(1) - mem_b32s1(v + 1));
                                            } else {
                                                crate::errors::confusion(c"disc1a".as_ptr());
                                            }
                                        }
                                        _ => crate::errors::confusion(c"disc1".as_ptr()),
                                    }
                                }
                            }

                            while s != TEX_NULL {
                                if is_char_node(s) {
                                    let f = mem_b16s1(s) as i32;
                                    set_break_width(
                                        1,
                                        break_width(1) + font_character_width(f, mem_b16s0(s)),
                                    );
                                } else {
                                    match mem_b16s1(s) as i32 {
                                        LIGATURE_NODE => {
                                            let f = mem_b16s1(s + 1) as i32;
                                            set_xtx_ligature_present(true);
                                            set_break_width(
                                                1,
                                                break_width(1)
                                                    + font_character_width(f, mem_b16s0(s + 1)),
                                            );
                                        }
                                        HLIST_NODE | VLIST_NODE | RULE_NODE | KERN_NODE => {
                                            set_break_width(1, break_width(1) + mem_b32s1(s + 1));
                                        }
                                        WHATSIT_NODE => {
                                            let sub = mem_b16s0(s);
                                            if sub == NATIVE_WORD_NODE
                                                || sub == NATIVE_WORD_NODE_AT
                                                || sub == GLYPH_NODE
                                                || sub == PIC_NODE
                                                || sub == PDF_NODE
                                            {
                                                set_break_width(1, break_width(1) + mem_b32s1(s + 1));
                                            } else {
                                                crate::errors::confusion(c"disc2a".as_ptr());
                                            }
                                        }
                                        _ => crate::errors::confusion(c"disc2".as_ptr()),
                                    }
                                }

                                s = mem_b32s1(s);
                            }

                            set_break_width(1, break_width(1) + disc_width());
                            if mem_b32s1(cur_p() + 1) == TEX_NULL {
                                s = mem_b32s1(v);
                            }
                        }
                    }

                    while s != TEX_NULL {
                        if is_char_node(s) {
                            break;
                        }

                        match mem_b16s1(s) as i32 {
                            GLUE_NODE => {
                                let v = mem_b32s0(s + 1);
                                set_break_width(1, break_width(1) - mem_b32s1(v + 1));
                                set_break_width(
                                    2 + mem_b16s1(v) as usize,
                                    break_width(2 + mem_b16s1(v) as usize) - mem_b32s1(v + 2),
                                );
                                set_break_width(6, break_width(6) - mem_b32s1(v + 3));
                            }
                            PENALTY_NODE => {}
                            MATH_NODE => {
                                set_break_width(1, break_width(1) - mem_b32s1(s + 1));
                            }
                            KERN_NODE => {
                                if mem_b16s0(s) != EXPLICIT {
                                    break;
                                }
                                set_break_width(1, break_width(1) - mem_b32s1(s + 1));
                            }
                            _ => break,
                        }

                        s = mem_b32s1(s);
                    }
                }

                if mem_b16s1(prev_r) as i32 == DELTA_NODE {
                    for (k, off) in [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)] {
                        set_mem_b32s1(
                            prev_r + off,
                            mem_b32s1(prev_r + off) - cur_active_width(k) + break_width(k),
                        );
                    }
                } else if prev_r == ACTIVE_LIST {
                    for k in 1..=6 {
                        set_active_width(k, break_width(k));
                    }
                } else {
                    let q = crate::mem::get_node(DELTA_NODE_SIZE);
                    set_mem_b32s1(q, r);
                    set_mem_b16s1(q, DELTA_NODE as u16);
                    set_mem_b16s0(q, 0);
                    for (k, off) in [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)] {
                        set_mem_b32s1(q + off, break_width(k) - cur_active_width(k));
                    }
                    set_mem_b32s1(prev_r, q);
                    prev_prev_r = prev_r;
                    prev_r = q;
                }

                let adj = eqtb_b32s1(INT_BASE + INT_PAR_ADJ_DEMERITS).abs();
                if adj >= MAX_HALFWORD - minimum_demerits() {
                    set_minimum_demerits(AWFUL_BAD - 1);
                } else {
                    set_minimum_demerits(minimum_demerits() + adj);
                }

                for fit_class in VERY_LOOSE_FIT..=TIGHT_FIT {
                    if minimal_demerits(fit_class) <= minimum_demerits() {
                        let passive_node = crate::mem::get_node(PASSIVE_NODE_SIZE);
                        set_mem_b32s1(passive_node, passive());
                        set_passive(passive_node);
                        set_mem_b32s1(passive_node + 1, cur_p());
                        set_mem_b32s0(passive_node + 1, best_place(fit_class));

                        let q = crate::mem::get_node(active_node_size());
                        set_active_node_break_node(q, passive());
                        set_active_node_line_number(q, best_pl_line(fit_class) + 1);
                        set_active_node_fitness(q, fit_class);
                        set_mem_b16s1(q, break_type as u16);
                        set_active_node_total_demerits(q, minimal_demerits(fit_class));

                        if do_last_line_fit() {
                            set_active_node_shortfall(q, best_pl_short(fit_class));
                            set_active_node_glue(q, best_pl_glue(fit_class));
                        }

                        set_mem_b32s1(q, r);
                        set_mem_b32s1(prev_r, q);
                        prev_r = q;
                    }

                    set_minimal_demerits(fit_class, MAX_HALFWORD);
                }

                set_minimum_demerits(MAX_HALFWORD);

                if r != LAST_ACTIVE {
                    let q = crate::mem::get_node(DELTA_NODE_SIZE);
                    set_mem_b32s1(q, r);
                    set_mem_b16s1(q, DELTA_NODE as u16);
                    set_mem_b16s0(q, 0);
                    for (k, off) in [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)] {
                        set_mem_b32s1(q + off, cur_active_width(k) - break_width(k));
                    }
                    set_mem_b32s1(prev_r, q);
                    prev_prev_r = prev_r;
                    prev_r = q;
                }
            }

            if r == LAST_ACTIVE {
                return;
            }

            if l > easy_line() {
                line_width = crate::globals::second_width();
                old_l = MAX_HALFWORD - 1;
            } else {
                old_l = l;
                if l > last_special_line() {
                    line_width = crate::globals::second_width();
                } else if eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE) == TEX_NULL {
                    line_width = first_width();
                } else {
                    line_width = mem_b32s1(eqtb_b32s1(LOCAL_BASE + LOCAL_PAR_SHAPE) + 2 * l);
                }
            }
        }

        let mut artificial_demerits;
        let mut shortfall;
        if semantic_pagination_enabled() {
            line_width = cur_active_width(1);
            artificial_demerits = true;
            shortfall = 0;
        } else {
            artificial_demerits = false;
            shortfall = line_width - cur_active_width(1);
            if eqtb_b32s1(INT_BASE + INT_PAR_XETEX_PROTRUDE_CHARS) > 1 {
                shortfall += total_pw(r, cur_p());
            }
        }

        let mut b = 0;
        let mut fit_class = DECENT_FIT;
        let mut jumped_to_found = false;

        if shortfall > 0 {
            if cur_active_width(3) != 0 || cur_active_width(4) != 0 || cur_active_width(5) != 0 {
                let mut last_line_fit_found = false;
                if do_last_line_fit() && cur_p() == TEX_NULL {
                    if active_node_shortfall(r) != 0
                        && active_node_glue(r) > 0
                        && cur_active_width(3) == fill_width(0)
                        && cur_active_width(4) == fill_width(1)
                        && cur_active_width(5) == fill_width(2)
                    {
                        g = if active_node_shortfall(r) > 0 {
                            cur_active_width(2)
                        } else {
                            cur_active_width(6)
                        };

                        if g > 0 {
                            crate::globals::set_arith_error(false);
                            g = crate::scaled_math::fract(
                                g,
                                active_node_shortfall(r),
                                active_node_glue(r),
                                MAX_HALFWORD,
                            );
                            if eqtb_b32s1(INT_BASE + INT_PAR_LAST_LINE_FIT) < 1000 {
                                g = crate::scaled_math::fract(
                                    g,
                                    eqtb_b32s1(INT_BASE + INT_PAR_LAST_LINE_FIT),
                                    1000,
                                    MAX_HALFWORD,
                                );
                            }
                            if crate::globals::arith_error() {
                                g = if active_node_shortfall(r) > 0 {
                                    MAX_HALFWORD
                                } else {
                                    -MAX_HALFWORD
                                };
                            }

                            if g > 0 {
                                if g > shortfall {
                                    g = shortfall;
                                }
                                if g > 7_230_584 && cur_active_width(2) < 1_663_497 {
                                    b = INF_BAD;
                                    fit_class = VERY_LOOSE_FIT;
                                } else {
                                    b = crate::helpers::badness(g, cur_active_width(2));
                                    fit_class = if b > 12 {
                                        if b > 99 { VERY_LOOSE_FIT } else { LOOSE_FIT }
                                    } else {
                                        DECENT_FIT
                                    };
                                }
                                last_line_fit_found = true;
                                jumped_to_found = true;
                            } else if g < 0 {
                                if -g > cur_active_width(6) {
                                    g = -cur_active_width(6);
                                }
                                b = crate::helpers::badness(-g, cur_active_width(6));
                                fit_class = if b > 12 { TIGHT_FIT } else { DECENT_FIT };
                                last_line_fit_found = true;
                                jumped_to_found = true;
                            }
                        }
                    }
                }

                if !last_line_fit_found {
                    shortfall = 0;
                    b = 0;
                    fit_class = DECENT_FIT;
                }
            } else {
                if shortfall > 7_230_584 && cur_active_width(2) < 1_663_497 {
                    b = INF_BAD;
                    fit_class = VERY_LOOSE_FIT;
                } else {
                    b = crate::helpers::badness(shortfall, cur_active_width(2));
                    fit_class = if b > 12 {
                        if b > 99 { VERY_LOOSE_FIT } else { LOOSE_FIT }
                    } else {
                        DECENT_FIT
                    };
                }
            }
        } else {
            if -shortfall > cur_active_width(6) {
                b = INF_BAD + 1;
            } else {
                b = crate::helpers::badness(-shortfall, cur_active_width(6));
            }
            fit_class = if b > 12 { TIGHT_FIT } else { DECENT_FIT };
        }

        if do_last_line_fit() && !jumped_to_found {
            if cur_p() == TEX_NULL {
                shortfall = 0;
            }
            g = if shortfall > 0 {
                cur_active_width(2)
            } else if shortfall < 0 {
                cur_active_width(6)
            } else {
                0
            };
        }

        let node_r_stays_active;
        if b > INF_BAD || pi == EJECT_PENALTY {
            if final_pass()
                && minimum_demerits() == AWFUL_BAD
                && mem_b32s1(r) == LAST_ACTIVE
                && prev_r == ACTIVE_LIST
            {
                artificial_demerits = true;
            } else if b > threshold() {
                deactivate_node(r, &mut prev_r, &mut prev_prev_r);
                continue;
            }
            node_r_stays_active = false;
        } else {
            prev_r = r;
            if b > threshold() {
                continue;
            }
            node_r_stays_active = true;
        }

        let mut d = if artificial_demerits {
            0
        } else {
            let mut d = eqtb_b32s1(INT_BASE + INT_PAR_LINE_PENALTY) + b;
            if d.abs() >= 10_000 {
                d = 100_000_000;
            } else {
                d *= d;
            }

            if pi != 0 {
                if pi > 0 {
                    d += pi * pi;
                } else if pi > EJECT_PENALTY {
                    d -= pi * pi;
                }
            }

            if break_type == HYPHENATED && mem_b16s1(r) as i16 == HYPHENATED {
                if cur_p() != TEX_NULL {
                    d += eqtb_b32s1(INT_BASE + INT_PAR_DOUBLE_HYPHEN_DEMERITS);
                } else {
                    d += eqtb_b32s1(INT_BASE + INT_PAR_FINAL_HYPHEN_DEMERITS);
                }
            }

            if ((fit_class as i32) - active_node_fitness(r) as i32).abs() > 1 {
                d += eqtb_b32s1(INT_BASE + INT_PAR_ADJ_DEMERITS);
            }

            d
        };

        d += active_node_total_demerits(r);
        if d <= minimal_demerits(fit_class) {
            set_minimal_demerits(fit_class, d);
            set_best_place(fit_class, active_node_break_node(r));
            set_best_pl_line(fit_class, l);

            if do_last_line_fit() {
                set_best_pl_short(fit_class, shortfall);
                set_best_pl_glue(fit_class, g);
            }

            if d < minimum_demerits() {
                set_minimum_demerits(d);
            }
        }

        if node_r_stays_active {
            continue;
        }

        deactivate_node(r, &mut prev_r, &mut prev_prev_r);
    }

    unsafe fn deactivate_node(r: i32, prev_r: &mut i32, prev_prev_r: &mut i32) {
        use crate::globals::{
            active_node_size, active_width, cur_active_width, mem_b16s1, mem_b32s1,
            set_active_width, set_cur_active_width, set_mem_b32s1,
        };

        set_mem_b32s1(*prev_r, mem_b32s1(r));
        crate::mem::free_node(r, active_node_size());

        if *prev_r == ACTIVE_LIST {
            let r = mem_b32s1(ACTIVE_LIST);
            if mem_b16s1(r) as i32 == DELTA_NODE {
                for (k, off) in [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)] {
                    set_active_width(k, active_width(k) + mem_b32s1(r + off));
                    set_cur_active_width(k, active_width(k));
                }
                set_mem_b32s1(ACTIVE_LIST, mem_b32s1(r));
                crate::mem::free_node(r, DELTA_NODE_SIZE);
            }
        } else if mem_b16s1(*prev_r) as i32 == DELTA_NODE {
            let r = mem_b32s1(*prev_r);
            if r == LAST_ACTIVE {
                for (k, off) in [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)] {
                    set_cur_active_width(k, cur_active_width(k) - mem_b32s1(*prev_r + off));
                }
                set_mem_b32s1(*prev_prev_r, LAST_ACTIVE);
                crate::mem::free_node(*prev_r, DELTA_NODE_SIZE);
                *prev_r = *prev_prev_r;
            } else if mem_b16s1(r) as i32 == DELTA_NODE {
                for (k, off) in [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)] {
                    set_cur_active_width(k, cur_active_width(k) + mem_b32s1(r + off));
                    set_mem_b32s1(*prev_r + off, mem_b32s1(*prev_r + off) + mem_b32s1(r + off));
                }
                set_mem_b32s1(*prev_r, mem_b32s1(r));
                crate::mem::free_node(r, DELTA_NODE_SIZE);
            }
        }
    }
}

// translated from xetex-linebreak.c:2010 (hyphenate)
#[no_mangle]
pub unsafe extern "C" fn hyphenate() {
    use crate::globals::{
        avail, bchar_label_entry, cur_lang, cur_p, font_bchar_entry, ha, hb, hc_entry,
        hf, hn, hu_entry, hyf_bchar, hyf_char, hyf_distance_entry, hyf_entry, hyf_next_entry,
        hyf_num_entry, hyph_link_entry, hyph_list_entry, hyph_word_entry, hyphen_passed,
        l_hyf, max_hyph_char, mem_as_u16_ptr, mem_as_u16_ptr_mut, mem_b16s0, mem_b16s1,
        mem_b32s0, mem_b32s1, op_start_entry, r_hyf, set_avail, set_hc_entry, set_hn,
        set_hu_entry, set_hyf_entry, set_hyphen_passed, set_init_lft, set_init_lig,
        set_init_list, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0, set_mem_b32s1,
        str_pool_entry, str_start, trie_trc_entry, trie_trl_entry, trie_tro_entry,
    };

    let mut hn_v = hn();
    for j in 0..=hn_v {
        set_hyf_entry(j as usize, 0);
    }

    let mut h = hc_entry(1);
    hn_v += 1;
    set_hn(hn_v);
    set_hc_entry(hn_v as usize, cur_lang() as i32);

    for j in 2..=hn_v {
        h = (h + h + hc_entry(j as usize)) % HYPH_PRIME;
    }

    loop {
        let k = hyph_word_entry(h);
        if k == 0 {
            break;
        }

        if crate::string_pool::length(k) == hn_v as i32 {
            let mut j = 1;
            let mut u = str_start(k - TOO_BIG_CHAR);
            loop {
                if str_pool_entry(u) as i32 != hc_entry(j as usize) {
                    break;
                }
                j += 1;
                u += 1;
                if j > hn_v {
                    let mut s = hyph_list_entry(h);
                    while s != TEX_NULL {
                        set_hyf_entry(mem_b32s0(s) as usize, 1);
                        s = mem_b32s1(s);
                    }
                    hn_v -= 1;
                    set_hn(hn_v);
                    goto_found(hn_v);
                    return;
                }
            }
        }

        let link = hyph_link_entry(h);
        if link == 0 {
            break;
        }
        h = link as i32 - 1;
    }

    hn_v -= 1;
    set_hn(hn_v);
    if trie_trc_entry(cur_lang() as i32 + 1) != cur_lang() as u16 {
        return;
    }

    set_hc_entry(0, 0);
    set_hc_entry((hn_v + 1) as usize, 0);
    set_hc_entry((hn_v + 2) as usize, max_hyph_char());

    for j in 0..=(hn_v - r_hyf() as i16 + 1) {
        let mut z = trie_trl_entry(cur_lang() as i32 + 1) + hc_entry(j as usize);
        let mut l = j;
        while hc_entry(l as usize) == trie_trc_entry(z) as i32 {
            if trie_tro_entry(z) != MIN_TRIE_OP {
                let mut v = trie_tro_entry(z);
                loop {
                    v += op_start_entry(cur_lang() as usize);
                    let i = l - hyf_distance_entry(v);
                    if hyf_num_entry(v) as u8 > hyf_entry(i as usize) {
                        set_hyf_entry(i as usize, hyf_num_entry(v) as u8);
                    }
                    v = hyf_next_entry(v) as i32;
                    if v == MIN_TRIE_OP {
                        break;
                    }
                }
            }
            l += 1;
            z = trie_trl_entry(z) + hc_entry(l as usize);
        }
    }

    goto_found(hn_v);

    unsafe fn goto_found(mut hn_v: i16) {
        use crate::globals::{
            avail, bchar_label_entry, cur_p, font_bchar_entry, ha, hb, hf, hn, hu_entry,
            hyf_bchar, hyf_char, hyf_entry, hyphen_passed, l_hyf, max_hyph_char,
            mem_as_u16_ptr, mem_as_u16_ptr_mut, mem_b16s0, mem_b16s1, mem_b32s1, r_hyf,
            set_avail, set_hn, set_hu_entry, set_hyf_entry, set_hyphen_passed, set_init_lft,
            set_init_lig, set_init_list, set_mem_b16s0, set_mem_b16s1, set_mem_b32s0,
            set_mem_b32s1,
        };

        for j in 0..=(l_hyf() as i16 - 1) {
            set_hyf_entry(j as usize, 0);
        }
        for j in 0..=(r_hyf() as i16 - 1) {
            set_hyf_entry((hn_v - j) as usize, 0);
        }

        let mut any_hyphen = false;
        for j in l_hyf() as i16..=(hn_v - r_hyf() as i16) {
            if odd(hyf_entry(j as usize)) {
                any_hyphen = true;
                break;
            }
        }
        if !any_hyphen {
            return;
        }

        if ha() != TEX_NULL
            && !is_char_node(ha())
            && mem_b16s1(ha()) as i32 == WHATSIT_NODE
            && (mem_b16s0(ha()) == NATIVE_WORD_NODE || mem_b16s0(ha()) == NATIVE_WORD_NODE_AT)
        {
            let mut s = cur_p();
            while mem_b32s1(s) != ha() {
                s = mem_b32s1(s);
            }

            set_hyphen_passed(0);
            for j in l_hyf() as i16..=(hn_v - r_hyf() as i16) {
                if odd(hyf_entry(j as usize)) {
                    let q = crate::scan::new_native_word_node(hf(), (j - hyphen_passed()) as i32);
                    set_mem_b16s0(q, mem_b16s0(ha()));
                    for i in 0..=(j - hyphen_passed() - 1) {
                        *mem_as_u16_ptr_mut(q + NATIVE_NODE_SIZE).add(i as usize) =
                            *mem_as_u16_ptr(ha() + NATIVE_NODE_SIZE)
                                .add((i + hyphen_passed()) as usize);
                    }
                    set_native_metrics(q);
                    set_mem_b32s1(s, q);
                    s = q;

                    let q = crate::mem::new_disc();
                    set_mem_b32s0(
                        q + 1,
                        crate::scan::new_native_character(hf(), hyf_char() as u32),
                    );
                    set_mem_b32s1(s, q);
                    s = q;
                    set_hyphen_passed(j);
                }
            }

            hn_v = mem_b16s1(ha() + 4) as i16;
            set_hn(hn_v);
            let q = crate::scan::new_native_word_node(hf(), (hn_v - hyphen_passed()) as i32);
            set_mem_b16s0(q, mem_b16s0(ha()));
            for i in 0..=(hn_v - hyphen_passed() - 1) {
                *mem_as_u16_ptr_mut(q + NATIVE_NODE_SIZE).add(i as usize) =
                    *mem_as_u16_ptr(ha() + NATIVE_NODE_SIZE).add((i + hyphen_passed()) as usize);
            }
            set_native_metrics(q);
            set_mem_b32s1(s, q);
            s = q;
            let q = mem_b32s1(ha());
            set_mem_b32s1(s, q);
            set_mem_b32s1(ha(), TEX_NULL);
            crate::scan::flush_node_list(ha());
            return;
        }

        let q_final = mem_b32s1(hb());
        set_mem_b32s1(hb(), TEX_NULL);
        let r = mem_b32s1(ha());
        set_mem_b32s1(ha(), TEX_NULL);
        let bchar = hyf_bchar();

        let (mut s, mut j) = if is_char_node(ha()) {
            if mem_b16s1(ha()) as i32 != hf() {
                set_hu_entry(0, max_hyph_char());
                set_init_lig(false);
                set_init_list(TEX_NULL);
                (ha(), 0)
            } else {
                set_init_list(ha());
                set_init_lig(false);
                set_hu_entry(0, mem_b16s0(ha()) as i32);
                let mut s = cur_p();
                while mem_b32s1(s) != ha() {
                    s = mem_b32s1(s);
                }
                (s, 0)
            }
        } else if mem_b16s1(ha()) as i32 == LIGATURE_NODE {
            if mem_b16s1(ha() + 1) as i32 != hf() {
                set_hu_entry(0, max_hyph_char());
                set_init_lig(false);
                set_init_list(TEX_NULL);
                (ha(), 0)
            } else {
                set_init_list(mem_b32s1(ha() + 1));
                set_init_lig(true);
                set_init_lft(mem_b16s0(ha()) > 1);
                set_hu_entry(0, mem_b16s0(ha() + 1) as i32);
                if crate::globals::init_list() == TEX_NULL && crate::globals::init_lft() {
                    set_hu_entry(0, max_hyph_char());
                    set_init_lig(false);
                }
                crate::mem::free_node(ha(), SMALL_NODE_SIZE);
                let mut s = cur_p();
                while mem_b32s1(s) != ha() {
                    s = mem_b32s1(s);
                }
                (s, 0)
            }
        } else {
            if !is_char_node(r) && mem_b16s1(r) as i32 == LIGATURE_NODE && mem_b16s0(r) > 1 {
                set_hu_entry(0, max_hyph_char());
                set_init_lig(false);
                set_init_list(TEX_NULL);
                (ha(), 0)
            } else {
                set_init_list(TEX_NULL);
                (ha(), 1)
            }
        };

        crate::scan::flush_node_list(r);

        loop {
            let mut l = j;
            j = reconstitute(j, hn_v, bchar, hyf_char()) + 1;
            if hyphen_passed() == 0 {
                set_mem_b32s1(s, mem_b32s1(HOLD_HEAD));
                while mem_b32s1(s) > TEX_NULL {
                    s = mem_b32s1(s);
                }
                if odd(hyf_entry((j - 1) as usize)) {
                    l = j;
                    set_hyphen_passed(j - 1);
                    set_mem_b32s1(HOLD_HEAD, TEX_NULL);
                }
            }

            if hyphen_passed() > 0 {
                loop {
                    let r = crate::mem::get_node(SMALL_NODE_SIZE);
                    set_mem_b32s1(r, mem_b32s1(HOLD_HEAD));
                    set_mem_b16s1(r, DISC_NODE as u16);
                    let mut major_tail = r;
                    let mut r_count = 0;
                    while mem_b32s1(major_tail) > TEX_NULL {
                        major_tail = mem_b32s1(major_tail);
                        r_count += 1;
                    }

                    let mut i = hyphen_passed();
                    set_hyf_entry(i as usize, 0);
                    let mut minor_tail = TEX_NULL;
                    set_mem_b32s0(r + 1, TEX_NULL);
                    let hyf_node = crate::scan::new_character(hf(), hyf_char() as u16);
                    let mut saved_c = 0;
                    if hyf_node != TEX_NULL {
                        i += 1;
                        saved_c = hu_entry(i as usize);
                        set_hu_entry(i as usize, hyf_char());
                        set_mem_b32s1(hyf_node, avail());
                        set_avail(hyf_node);
                    }

                    while l <= i {
                        l = reconstitute(l, i, font_bchar_entry(hf()), TOO_BIG_CHAR) + 1;
                        if mem_b32s1(HOLD_HEAD) > TEX_NULL {
                            if minor_tail == TEX_NULL {
                                set_mem_b32s0(r + 1, mem_b32s1(HOLD_HEAD));
                            } else {
                                set_mem_b32s1(minor_tail, mem_b32s1(HOLD_HEAD));
                            }
                            minor_tail = mem_b32s1(HOLD_HEAD);
                            while mem_b32s1(minor_tail) > TEX_NULL {
                                minor_tail = mem_b32s1(minor_tail);
                            }
                        }
                    }

                    if hyf_node != TEX_NULL {
                        set_hu_entry(i as usize, saved_c);
                        l = i;
                        i -= 1;
                    }

                    minor_tail = TEX_NULL;
                    set_mem_b32s1(r + 1, TEX_NULL);
                    let mut c_loc = 0;
                    let mut c = 0;
                    if bchar_label_entry(hf()) != NON_ADDRESS {
                        l -= 1;
                        c = hu_entry(l as usize);
                        c_loc = l;
                        set_hu_entry(l as usize, max_hyph_char());
                    }

                    while l < j {
                        loop {
                            l = reconstitute(l, hn_v, bchar, TOO_BIG_CHAR) + 1;
                            if c_loc > 0 {
                                set_hu_entry(c_loc as usize, c);
                                c_loc = 0;
                            }
                            if mem_b32s1(HOLD_HEAD) > TEX_NULL {
                                if minor_tail == TEX_NULL {
                                    set_mem_b32s1(r + 1, mem_b32s1(HOLD_HEAD));
                                } else {
                                    set_mem_b32s1(minor_tail, mem_b32s1(HOLD_HEAD));
                                }
                                minor_tail = mem_b32s1(HOLD_HEAD);
                                while mem_b32s1(minor_tail) > TEX_NULL {
                                    minor_tail = mem_b32s1(minor_tail);
                                }
                            }
                            if l >= j {
                                break;
                            }
                        }

                        while l > j {
                            j = reconstitute(j, hn_v, bchar, TOO_BIG_CHAR) + 1;
                            set_mem_b32s1(major_tail, mem_b32s1(HOLD_HEAD));
                            while mem_b32s1(major_tail) > TEX_NULL {
                                major_tail = mem_b32s1(major_tail);
                                r_count += 1;
                            }
                        }
                    }

                    if r_count > 127 {
                        set_mem_b32s1(s, mem_b32s1(r));
                        set_mem_b32s1(r, TEX_NULL);
                        crate::scan::flush_node_list(r);
                    } else {
                        set_mem_b32s1(s, r);
                        set_mem_b16s0(r, r_count as u16);
                    }
                    s = major_tail;
                    set_hyphen_passed(j - 1);
                    set_mem_b32s1(HOLD_HEAD, TEX_NULL);

                    if !odd(hyf_entry((j - 1) as usize)) {
                        break;
                    }
                }
            }

            if j > hn_v {
                break;
            }
        }

        set_mem_b32s1(s, q_final);
        crate::mem::flush_list(crate::globals::init_list());
    }
}
