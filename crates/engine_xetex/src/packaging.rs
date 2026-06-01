//! Box/node packing and list utilities — `hpack`/`vpackage`/`vsplit`/
//! `vert_break`/`append_to_vlist`/`prune_page_top`/`do_marks` and the
//! node-recycle utilities `copy_node_list`/`flush_node_list`/`just_copy`/
//! `just_reverse`. Relocated from `scan.rs` (= xetex0.c's box/node-packing
//! group) by workstream 02 (module decomposition).
//!
//! Transitional home (Decision 3): this is node-layer code destined for the
//! `node/` tree, which depends on the typed-accessor design of workstream 03
//! (not yet built). Parked flat here until then.

use crate::consts::{
    BOX_NODE_SIZE, DISC_NODE, GLUE_NODE, GLUE_SPEC_SIZE, HLIST_NODE, KERN_NODE, LIGATURE_NODE,
    MATH_NODE, MEDIUM_NODE_SIZE, MIN_HALFWORD, PENALTY_NODE, RULE_NODE, RULE_NODE_SIZE,
    SMALL_NODE_SIZE,
};
use crate::globals::{
    mem_b16s0, mem_b16s1, mem_b16s3, mem_b32s0, mem_b32s1, mem_word, set_mem_b16s0, set_mem_b16s1,
    set_mem_b32s0, set_mem_b32s1, set_mem_ptr, set_mem_word,
};
use crate::shipout::new_edge_global;

// Shared node-recycle infrastructure stays in scan.rs (also used by
// prune_page_top and the relocated font-loading group); import it here.
use crate::scan::{cnl_get_avail, cnl_get_node, node_recycle_abort, NodeRecycleAbort};

// ---------------------------------------------------------------------------
// flush_node_list (node-type constants — xetex_format.h / xetex-constants.h)
// ---------------------------------------------------------------------------
const VLIST_NODE_FNL: u16 = 1;
const INS_NODE_FNL: u16 = 3;
const MARK_NODE_FNL: u16 = 4;
const ADJUST_NODE_FNL: u16 = 5;
const WHATSIT_NODE_FNL: u16 = 8;
const UNSET_NODE_FNL: u16 = 13;
const STYLE_NODE_FNL: u16 = 14;
const CHOICE_NODE_FNL: u16 = 15;
const ORD_NOAD_FNL: u16 = 16;
const OP_NOAD_FNL: u16 = 17;
const BIN_NOAD_FNL: u16 = 18;
const REL_NOAD_FNL: u16 = 19;
const OPEN_NOAD_FNL: u16 = 20;
const CLOSE_NOAD_FNL: u16 = 21;
const PUNCT_NOAD_FNL: u16 = 22;
const INNER_NOAD_FNL: u16 = 23;
const RADICAL_NOAD_FNL: u16 = 24;
const FRACTION_NOAD_FNL: u16 = 25;
const UNDER_NOAD_FNL: u16 = 26;
const OVER_NOAD_FNL: u16 = 27;
const ACCENT_NOAD_FNL: u16 = 28;
const VCENTER_NOAD_FNL: u16 = 29;
const LEFT_NOAD_FNL: u16 = 30;
const RIGHT_NOAD_FNL: u16 = 31;
const MARGIN_KERN_NODE_FNL: u16 = 40; // checked below
                                      // Whatsit subtypes (xetex_format.h:457–467).
const OPEN_NODE_FNL: u16 = 0;
const WRITE_NODE_FNL: u16 = 1;
const CLOSE_NODE_FNL: u16 = 2;
const SPECIAL_NODE_FNL: u16 = 3;
const LANGUAGE_NODE_FNL: u16 = 4;
const PDF_SAVE_POS_NODE_FNL: u16 = 21;
const NATIVE_WORD_NODE_FNL: u16 = 40;
const NATIVE_WORD_NODE_AT_FNL: u16 = 41;
const GLYPH_NODE_FNL: u16 = 42;
const PIC_NODE_FNL: u16 = 43;
const PDF_NODE_FNL: u16 = 44;
// Node sizes (xetex-constants.h).
const INS_NODE_SIZE_FNL: i32 = 5;
const OPEN_NODE_SIZE_FNL: i32 = 3;
const WRITE_NODE_SIZE_FNL: i32 = 2;
const GLYPH_NODE_SIZE_FNL: i32 = 5;
const PIC_NODE_SIZE_FNL: i32 = 9;
const MARGIN_KERN_NODE_SIZE_FNL: i32 = 3;
const STYLE_NODE_SIZE_FNL: i32 = 3;
const NOAD_SIZE_FNL: i32 = 4;
const RADICAL_NOAD_SIZE_FNL: i32 = 5;
const ACCENT_NOAD_SIZE_FNL: i32 = 5;
const FRACTION_NOAD_SIZE_FNL: i32 = 6;
const SUB_BOX_FNL: i32 = 2; // xetex-constants.h:283
                            // sizeof(memory_word) = 8 on 64-bit platforms.
const MEMORY_WORD_SIZE_FNL: i32 = 8;

// ---------------------------------------------------------------------------
// copy_node_list (node-type constants — xetex_format.h / xetex-constants.h)
// ---------------------------------------------------------------------------
const VLIST_NODE_CNL: u16 = 1;
const INS_NODE_CNL: u16 = 3;
const UNSET_NODE_CNL: u16 = 13;
const WHATSIT_NODE_CNL: u16 = 8;
const MARGIN_KERN_NODE_CNL: u16 = 40;
const MARK_NODE_CNL: u16 = 4;
const ADJUST_NODE_CNL: u16 = 5;
// Whatsit subtypes.
const OPEN_NODE_CNL: u16 = 0;
const WRITE_NODE_CNL: u16 = 1;
const SPECIAL_NODE_CNL: u16 = 3;
const CLOSE_NODE_CNL: u16 = 2;
const LANGUAGE_NODE_CNL: u16 = 4;
const NATIVE_WORD_NODE_CNL: u16 = 40;
const NATIVE_WORD_NODE_AT_CNL: u16 = 41;
const GLYPH_NODE_CNL: u16 = 42;
const PIC_NODE_CNL: u16 = 43;
const PDF_NODE_CNL: u16 = 44;
const PDF_SAVE_POS_NODE_CNL: u16 = 21;
// Node sizes.
const INS_NODE_SIZE_CNL: i32 = 5;
const OPEN_NODE_SIZE_CNL: i32 = 3;
const WRITE_NODE_SIZE_CNL: i32 = 2;
const GLYPH_NODE_SIZE_CNL: i32 = 5;
const PIC_NODE_SIZE_CNL: i32 = 9;
const MARGIN_KERN_NODE_SIZE_CNL: i32 = 3;
const MEMORY_WORD_SIZE_CNL: i32 = 8;
// SYNCTEX_FIELD_SIZE = 1, so SYNCTEX_{tag,line}(p, BOX_NODE_SIZE) = mem[p+7].b32.{s0,s1}

// ---- moved verbatim from scan.rs (workstream 02) ----

// translated from xetex-xetex0.c:8029 (just_copy)
#[no_mangle]
pub unsafe extern "C" fn just_copy(p: i32, h: i32, t: i32) {
    use crate::globals::hi_mem_min;
    use crate::mem::TEX_NULL;

    extern "C" {}

    const OPEN_NODE_SIZE: i32 = 3; // xetex-constants.h:118
    const WRITE_NODE_SIZE: i32 = 2; // xetex-constants.h:112
    const GLYPH_NODE_SIZE: i32 = 5; // xetex-constants.h:126
    const PIC_NODE_SIZE: i32 = 9; // xetex-constants.h:133
    const MEM_WORD_BYTES: i32 = 8; // sizeof(memory_word)
    const SYNCTEX_FIELD_SIZE: i32 = 1; // xetex-constants.h:102
    const VLIST_NODE: u16 = 1;
    const WHATSIT_NODE: u16 = 8;
    const OPEN_NODE: u16 = 0;
    const WRITE_NODE: u16 = 1;
    const CLOSE_NODE: u16 = 2;
    const SPECIAL_NODE: u16 = 3;
    const LANGUAGE_NODE: u16 = 4;
    const NATIVE_WORD_NODE: u16 = 40;
    const NATIVE_WORD_NODE_AT: u16 = 41;
    const GLYPH_NODE: u16 = 42;
    const PIC_NODE: u16 = 43;
    const PDF_NODE: u16 = 44;
    const PDF_SAVE_POS_NODE: u16 = 21;

    let mut p = p;
    let mut h = h;

    while p != TEX_NULL {
        let mut words: i32 = 1;

        // Determine r and words for this node. None = skip linking (goto not_found).
        let r_opt: Option<i32> = if p >= hi_mem_min() {
            // char node
            Some(crate::mem::get_avail_global())
        } else {
            match mem_b16s1(p) {
                HLIST_NODE | VLIST_NODE => {
                    let r = crate::mem::get_node_global(BOX_NODE_SIZE);
                    // SYNCTEX_tag(r, BOX_NODE_SIZE) = SYNCTEX_tag(p, BOX_NODE_SIZE)
                    set_mem_b32s0(
                        r + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                        mem_b32s0(p + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE),
                    );
                    // SYNCTEX_line(r, BOX_NODE_SIZE) = SYNCTEX_line(p, BOX_NODE_SIZE)
                    set_mem_b32s1(
                        r + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                        mem_b32s1(p + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE),
                    );
                    set_mem_word(r + 6, mem_word(p + 6));
                    set_mem_word(r + 5, mem_word(p + 5));
                    words = 5;
                    set_mem_b32s1(r + 5, TEX_NULL); // BOX_list_ptr(r) = TEX_NULL
                    Some(r)
                }
                RULE_NODE => {
                    words = RULE_NODE_SIZE;
                    Some(crate::mem::get_node_global(RULE_NODE_SIZE))
                }
                LIGATURE_NODE => {
                    let r = crate::mem::get_avail_global();
                    set_mem_word(r, mem_word(p + 1)); // mem[r] = mem[p + 1]
                    words = 0; // skip the copy loop (goto found)
                    Some(r)
                }
                KERN_NODE | MATH_NODE => {
                    words = MEDIUM_NODE_SIZE;
                    Some(crate::mem::get_node_global(MEDIUM_NODE_SIZE))
                }
                GLUE_NODE => {
                    let r = crate::mem::get_node_global(MEDIUM_NODE_SIZE);
                    // GLUE_SPEC_ref_count(GLUE_NODE_glue_ptr(p))++
                    let gp = mem_b32s0(p + 1); // GLUE_NODE_glue_ptr(p)
                    set_mem_b32s1(gp, mem_b32s1(gp) + 1); // GLUE_SPEC_ref_count(gp)++
                                                          // SYNCTEX fields
                    set_mem_b32s0(
                        r + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                        mem_b32s0(p + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE),
                    );
                    set_mem_b32s1(
                        r + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                        mem_b32s1(p + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE),
                    );
                    set_mem_b32s0(r + 1, gp); // GLUE_NODE_glue_ptr(r) = gp
                    set_mem_b32s1(r + 1, TEX_NULL); // GLUE_NODE_leader_ptr(r) = TEX_NULL
                    Some(r)
                }
                WHATSIT_NODE => {
                    match mem_b16s0(p) {
                        OPEN_NODE => {
                            words = OPEN_NODE_SIZE;
                            Some(crate::mem::get_node_global(OPEN_NODE_SIZE))
                        }
                        WRITE_NODE | SPECIAL_NODE => {
                            let r = crate::mem::get_node_global(WRITE_NODE_SIZE);
                            // TOKEN_LIST_ref_count(WRITE_NODE_tokens(p))++
                            let toks = mem_b32s1(p + 1); // WRITE_NODE_tokens(p)
                            set_mem_b32s0(toks, mem_b32s0(toks) + 1);
                            words = WRITE_NODE_SIZE;
                            Some(r)
                        }
                        CLOSE_NODE | LANGUAGE_NODE => {
                            words = SMALL_NODE_SIZE;
                            Some(crate::mem::get_node_global(SMALL_NODE_SIZE))
                        }
                        NATIVE_WORD_NODE | NATIVE_WORD_NODE_AT => {
                            let sz = mem_b16s3(p + 4) as i32; // NATIVE_NODE_size(p)
                            let r = crate::mem::get_node_global(sz);
                            let mut w = sz;
                            while w > 0 {
                                w -= 1;
                                set_mem_word(r + w, mem_word(p + w));
                            }
                            // NATIVE_NODE_glyph_info_ptr(r) = NULL
                            set_mem_ptr(r + 5, std::ptr::null_mut());
                            // NATIVE_NODE_glyph_count(r) = 0
                            set_mem_b16s0(r + 4, 0);
                            crate::mem::copy_native_glyph_info_global(p, r);
                            words = 0; // already copied, skip outer loop
                            Some(r)
                        }
                        GLYPH_NODE => {
                            words = GLYPH_NODE_SIZE;
                            Some(crate::mem::get_node_global(GLYPH_NODE_SIZE))
                        }
                        PIC_NODE | PDF_NODE => {
                            let path_len = mem_b16s1(p + 4) as i32; // PIC_NODE_path_len
                            words =
                                PIC_NODE_SIZE + (path_len + MEM_WORD_BYTES - 1) / MEM_WORD_BYTES;
                            Some(crate::mem::get_node_global(words))
                        }
                        PDF_SAVE_POS_NODE => Some(crate::mem::get_node_global(SMALL_NODE_SIZE)),
                        _ => {
                            crate::errors::confusion(c"ext2".as_ptr());
                        }
                    }
                }
                _ => None, // goto not_found: skip copy and linking
            }
        };

        if let Some(r) = r_opt {
            // Copy words from p to r (skip if words==0)
            let mut w = words;
            while w > 0 {
                w -= 1;
                set_mem_word(r + w, mem_word(p + w));
            }
            // found: link r into h
            set_mem_b32s1(h, r); // LLIST_link(h) = r
            h = r;
        }

        // not_found: advance p
        p = mem_b32s1(p); // LLIST_link(p)
    }

    set_mem_b32s1(h, t); // LLIST_link(h) = t
}
// translated from xetex-xetex0.c:8155 (just_reverse)
#[no_mangle]
pub unsafe extern "C" fn just_reverse(p: i32) {
    use crate::globals::{
        avail, cur_dir, lr_problems, lr_ptr, set_avail, set_cur_dir, set_lr_problems, set_lr_ptr,
        set_temp_ptr, temp_ptr,
    };
    use crate::mem::TEX_NULL;

    extern "C" {}

    const TEMP_HEAD: i32 = 4_999_996; // MEM_TOP - 3
    const L_CODE: i32 = 4; // xetex-constants.h:142
    const R_CODE: i32 = 8; // xetex-constants.h:143

    let mut p = p;
    let mut m = MIN_HALFWORD;
    let mut n = MIN_HALFWORD;

    let mut q = if mem_b32s1(TEMP_HEAD) == TEX_NULL {
        just_copy(mem_b32s1(p), TEMP_HEAD, TEX_NULL);
        mem_b32s1(TEMP_HEAD)
    } else {
        let q = mem_b32s1(p);
        set_mem_b32s1(p, TEX_NULL);
        flush_node_list_global(mem_b32s1(TEMP_HEAD));
        q
    };

    let t = new_edge_global(cur_dir(), 0);
    let mut l = t;
    set_cur_dir(1 - cur_dir());

    'done: loop {
        while q != TEX_NULL {
            if q >= crate::globals::hi_mem_min() {
                // char node do-while
                loop {
                    p = q;
                    q = mem_b32s1(p);
                    set_mem_b32s1(p, l);
                    l = p;
                    if q < crate::globals::hi_mem_min() {
                        break;
                    }
                }
            } else {
                p = q;
                q = mem_b32s1(p);

                if mem_b16s1(p) == MATH_NODE {
                    let subtype = mem_b16s0(p) as i32;
                    if subtype & 1 != 0 {
                        // odd subtype = end math node
                        let expected = L_CODE * (subtype / L_CODE) + 3;
                        if mem_b32s0(lr_ptr()) != expected {
                            set_mem_b16s1(p, KERN_NODE); // NODE_type(p) = KERN_NODE
                            set_lr_problems(lr_problems() + 1);
                        } else {
                            // pop LR_ptr
                            set_temp_ptr(lr_ptr());
                            set_lr_ptr(mem_b32s1(temp_ptr()));
                            set_mem_b32s1(temp_ptr(), avail());
                            set_avail(temp_ptr());

                            if n > MIN_HALFWORD {
                                n -= 1;
                                set_mem_b16s0(p, mem_b16s0(p) - 1);
                            } else if m > MIN_HALFWORD {
                                m -= 1;
                                set_mem_b16s1(p, KERN_NODE);
                            } else {
                                // found edge end: set t's link and free p
                                set_mem_b32s1(t + 1, mem_b32s1(p + 1));
                                set_mem_b32s1(t, q);
                                crate::mem::free_node_global(p, MEDIUM_NODE_SIZE);
                                break 'done;
                            }
                        }
                    } else {
                        // even subtype = begin math node
                        // push LR_ptr
                        set_temp_ptr(crate::mem::get_avail_global());
                        set_mem_b32s0(temp_ptr(), L_CODE * (subtype / L_CODE) + 3);
                        set_mem_b32s1(temp_ptr(), lr_ptr());
                        set_lr_ptr(temp_ptr());

                        if n > MIN_HALFWORD || (subtype / R_CODE) != cur_dir() as i32 {
                            n += 1;
                            set_mem_b16s0(p, mem_b16s0(p) + 1);
                        } else {
                            set_mem_b16s1(p, KERN_NODE);
                            m += 1;
                        }
                    }
                }

                set_mem_b32s1(p, l);
                l = p;
            }
        }
        break; // goto done (past dead code)
    }

    // done:
    set_mem_b32s1(TEMP_HEAD, l);
}
/// translated from xetex-xetex0.c:765 (flush_node_list)
///
/// Phase-C: threaded to `&mut MemState`. Touches the `mem` arena directly,
/// recurses on itself, and calls the threaded [`crate::mem::free_node`] /
/// [`crate::mem::delete_glue_ref`] / [`crate::mem::delete_token_ref`]. The two
/// impossible corrupt-node arms cannot call the `with_state`-bearing
/// `confusion()` under the borrow, so they return `Err(message)` (the `01b` §5
/// sentinel) and [`flush_node_list_global`] performs the abort after the borrow
/// drops. Recursive-call arguments are precomputed into locals so the immutable
/// arena read does not overlap the `&mut mem` reborrow.
pub fn flush_node_list(
    mem: &mut crate::state::MemState,
    mut p: i32,
) -> Result<(), &'static std::ffi::CStr> {
    while p != crate::mem::TEX_NULL {
        let q = mem.b32s1(p);
        if p >= mem.hi_mem_min {
            // is_char_node(p): free to avail list
            mem.set_b32s1(p, mem.avail);
            mem.avail = p;
        } else {
            // Goto-done pattern: break 'done skips free_node(p, SMALL_NODE_SIZE) at end.
            'done: {
                match mem.b16s1(p) {
                    HLIST_NODE | VLIST_NODE_FNL | UNSET_NODE_FNL => {
                        let c = mem.b32s1(p + 5);
                        flush_node_list(mem, c)?;
                        crate::mem::free_node(mem, p, BOX_NODE_SIZE);
                        break 'done;
                    }
                    RULE_NODE => {
                        crate::mem::free_node(mem, p, RULE_NODE_SIZE);
                        break 'done;
                    }
                    INS_NODE_FNL => {
                        let c = mem.b32s0(p + 4);
                        flush_node_list(mem, c)?;
                        let g = mem.b32s1(p + 4);
                        crate::mem::delete_glue_ref(mem, g);
                        crate::mem::free_node(mem, p, INS_NODE_SIZE_FNL);
                        break 'done;
                    }
                    WHATSIT_NODE_FNL => {
                        match mem.b16s0(p) {
                            OPEN_NODE_FNL => {
                                crate::mem::free_node(mem, p, OPEN_NODE_SIZE_FNL);
                            }
                            WRITE_NODE_FNL | SPECIAL_NODE_FNL => {
                                let t = mem.b32s1(p + 1);
                                crate::mem::delete_token_ref(mem, t);
                                crate::mem::free_node(mem, p, WRITE_NODE_SIZE_FNL);
                                break 'done;
                            }
                            CLOSE_NODE_FNL | LANGUAGE_NODE_FNL => {
                                crate::mem::free_node(mem, p, SMALL_NODE_SIZE);
                            }
                            NATIVE_WORD_NODE_FNL | NATIVE_WORD_NODE_AT_FNL => {
                                let glyph_ptr = mem.ptr(p + 5);
                                if !glyph_ptr.is_null() {
                                    // SAFETY: glyph_ptr was xmalloc'd by the native-word shaper.
                                    unsafe { libc::free(glyph_ptr) };
                                    mem.set_ptr(p + 5, std::ptr::null_mut());
                                    mem.set_b16s0(p + 4, 0);
                                }
                                let sz = mem.b16s3(p + 4) as i32;
                                crate::mem::free_node(mem, p, sz);
                            }
                            GLYPH_NODE_FNL => {
                                crate::mem::free_node(mem, p, GLYPH_NODE_SIZE_FNL);
                            }
                            PIC_NODE_FNL | PDF_NODE_FNL => {
                                let path_len = mem.b16s1(p + 4) as i32;
                                let sz = PIC_NODE_SIZE_FNL
                                    + (path_len + MEMORY_WORD_SIZE_FNL - 1) / MEMORY_WORD_SIZE_FNL;
                                crate::mem::free_node(mem, p, sz);
                            }
                            PDF_SAVE_POS_NODE_FNL => {
                                crate::mem::free_node(mem, p, SMALL_NODE_SIZE);
                            }
                            _ => {
                                return Err(c"ext3");
                            }
                        }
                        break 'done;
                    }
                    GLUE_NODE => {
                        let spec = mem.b32s0(p + 1);
                        if mem.b32s1(spec) == crate::mem::TEX_NULL {
                            crate::mem::free_node(mem, spec, GLUE_SPEC_SIZE);
                        } else {
                            mem.set_b32s1(spec, mem.b32s1(spec) - 1);
                        }
                        if mem.b32s1(p + 1) != crate::mem::TEX_NULL {
                            let c = mem.b32s1(p + 1);
                            flush_node_list(mem, c)?;
                        }
                        crate::mem::free_node(mem, p, MEDIUM_NODE_SIZE);
                        break 'done;
                    }
                    KERN_NODE | MATH_NODE | PENALTY_NODE => {
                        crate::mem::free_node(mem, p, MEDIUM_NODE_SIZE);
                        break 'done;
                    }
                    MARGIN_KERN_NODE_FNL => {
                        crate::mem::free_node(mem, p, MARGIN_KERN_NODE_SIZE_FNL);
                        break 'done;
                    }
                    LIGATURE_NODE => {
                        let c = mem.b32s1(p + 1);
                        flush_node_list(mem, c)?;
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    MARK_NODE_FNL => {
                        let t = mem.b32s1(p + 1);
                        crate::mem::delete_token_ref(mem, t);
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    DISC_NODE => {
                        let c1 = mem.b32s0(p + 1);
                        flush_node_list(mem, c1)?;
                        let c2 = mem.b32s1(p + 1);
                        flush_node_list(mem, c2)?;
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    ADJUST_NODE_FNL => {
                        let c = mem.b32s1(p + 1);
                        flush_node_list(mem, c)?;
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    STYLE_NODE_FNL => {
                        crate::mem::free_node(mem, p, STYLE_NODE_SIZE_FNL);
                        break 'done;
                    }
                    CHOICE_NODE_FNL => {
                        let c1 = mem.b32s0(p + 1);
                        flush_node_list(mem, c1)?;
                        let c2 = mem.b32s1(p + 1);
                        flush_node_list(mem, c2)?;
                        let c3 = mem.b32s0(p + 2);
                        flush_node_list(mem, c3)?;
                        let c4 = mem.b32s1(p + 2);
                        flush_node_list(mem, c4)?;
                        crate::mem::free_node(mem, p, STYLE_NODE_SIZE_FNL);
                        break 'done;
                    }
                    ORD_NOAD_FNL | OP_NOAD_FNL | BIN_NOAD_FNL | REL_NOAD_FNL | OPEN_NOAD_FNL
                    | CLOSE_NOAD_FNL | PUNCT_NOAD_FNL | INNER_NOAD_FNL | RADICAL_NOAD_FNL
                    | OVER_NOAD_FNL | UNDER_NOAD_FNL | VCENTER_NOAD_FNL | ACCENT_NOAD_FNL => {
                        if mem.b32s1(p + 1) >= SUB_BOX_FNL {
                            let c = mem.b32s0(p + 1);
                            flush_node_list(mem, c)?;
                        }
                        if mem.b32s1(p + 2) >= SUB_BOX_FNL {
                            let c = mem.b32s0(p + 2);
                            flush_node_list(mem, c)?;
                        }
                        if mem.b32s1(p + 3) >= SUB_BOX_FNL {
                            let c = mem.b32s0(p + 3);
                            flush_node_list(mem, c)?;
                        }
                        let ntype = mem.b16s1(p);
                        if ntype == RADICAL_NOAD_FNL {
                            crate::mem::free_node(mem, p, RADICAL_NOAD_SIZE_FNL);
                        } else if ntype == ACCENT_NOAD_FNL {
                            crate::mem::free_node(mem, p, ACCENT_NOAD_SIZE_FNL);
                        } else {
                            crate::mem::free_node(mem, p, NOAD_SIZE_FNL);
                        }
                        break 'done;
                    }
                    LEFT_NOAD_FNL | RIGHT_NOAD_FNL => {
                        crate::mem::free_node(mem, p, NOAD_SIZE_FNL);
                        break 'done;
                    }
                    FRACTION_NOAD_FNL => {
                        let c1 = mem.b32s0(p + 2);
                        flush_node_list(mem, c1)?;
                        let c2 = mem.b32s0(p + 3);
                        flush_node_list(mem, c2)?;
                        crate::mem::free_node(mem, p, FRACTION_NOAD_SIZE_FNL);
                        break 'done;
                    }
                    _ => {
                        return Err(c"flushing");
                    }
                }
                crate::mem::free_node(mem, p, SMALL_NODE_SIZE);
            }
        }
        p = q;
    }
    Ok(())
}
/// Transitional bridge-down wrapper for legacy `flush_node_list` callers
/// (`01b` §3, §5). Runs the threaded list disposal under a single borrow, then
/// — after the borrow drops — fires the cold `confusion()` abort if a corrupt
/// node was encountered. `confusion()` never returns, so the `Err` is only ever
/// observed here, never by real callers.
pub unsafe fn flush_node_list_global(p: i32) {
    if let Err(msg) = crate::state::with_state_mut(|st| flush_node_list(&mut st.mem, p)) {
        crate::errors::confusion(msg.as_ptr());
    }
}
/// translated from xetex-xetex0.c:953 (copy_node_list)
///
/// Phase-C: threaded to `&mut MemState`. Recurses on itself and bottoms out on
/// the threaded `get_avail`/`get_node`/`copy_native_glyph_info`; allocation
/// failure and the two impossible corrupt-node `confusion` arms return the
/// `01b` §5 [`NodeRecycleAbort`] sentinel, which [`copy_node_list_global`]
/// fires after the borrow drops. Recursive-call arguments are precomputed into
/// locals to avoid overlapping the `&mut mem` reborrow. Legacy callers use
/// [`copy_node_list_global`].
pub fn copy_node_list(
    mem: &mut crate::state::MemState,
    mut p: i32,
) -> Result<i32, NodeRecycleAbort> {
    use crate::mem::TEX_NULL;
    let h = cnl_get_avail!(mem);
    let mut q = h;
    while p != TEX_NULL {
        let mut words = 1i32;
        let mut r = 0i32;
        if p >= mem.hi_mem_min {
            r = cnl_get_avail!(mem);
        } else {
            match mem.b16s1(p) {
                HLIST_NODE | VLIST_NODE_CNL | UNSET_NODE_CNL => {
                    r = cnl_get_node!(mem, BOX_NODE_SIZE);
                    // SYNCTEX_tag(r, BOX_NODE_SIZE) = SYNCTEX_tag(p, BOX_NODE_SIZE)
                    mem.set_b32s0(r + 7, mem.b32s0(p + 7));
                    // SYNCTEX_line(r, BOX_NODE_SIZE) = SYNCTEX_line(p, BOX_NODE_SIZE)
                    mem.set_b32s1(r + 7, mem.b32s1(p + 7));
                    mem.set_word(r + 6, mem.word(p + 6));
                    mem.set_word(r + 5, mem.word(p + 5));
                    let src = mem.b32s1(p + 5);
                    let copied = copy_node_list(mem, src)?;
                    mem.set_b32s1(r + 5, copied);
                    words = 5;
                }
                RULE_NODE => {
                    r = cnl_get_node!(mem, RULE_NODE_SIZE);
                    words = RULE_NODE_SIZE - 1;
                }
                INS_NODE_CNL => {
                    r = cnl_get_node!(mem, INS_NODE_SIZE_CNL);
                    mem.set_word(r + 4, mem.word(p + 4));
                    // GLUE_SPEC_ref_count(mem[p+4].b32.s1)++ = mem[mem[p+4].b32.s1].b32.s1++
                    let spec = mem.b32s1(p + 4);
                    mem.set_b32s1(spec, mem.b32s1(spec) + 1);
                    let src = mem.b32s0(p + 4);
                    let copied = copy_node_list(mem, src)?;
                    mem.set_b32s0(r + 4, copied);
                    words = INS_NODE_SIZE_CNL - 1;
                }
                WHATSIT_NODE_CNL => {
                    match mem.b16s0(p) {
                        OPEN_NODE_CNL => {
                            r = cnl_get_node!(mem, OPEN_NODE_SIZE_CNL);
                            words = OPEN_NODE_SIZE_CNL;
                        }
                        WRITE_NODE_CNL | SPECIAL_NODE_CNL => {
                            // mem[mem[p+1].b32.s1].b32.s0++ (token list ref count)
                            let tok = mem.b32s1(p + 1);
                            mem.set_b32s0(tok, mem.b32s0(tok) + 1);
                            r = cnl_get_node!(mem, WRITE_NODE_SIZE_CNL);
                            words = WRITE_NODE_SIZE_CNL;
                        }
                        CLOSE_NODE_CNL | LANGUAGE_NODE_CNL => {
                            r = cnl_get_node!(mem, SMALL_NODE_SIZE);
                            words = SMALL_NODE_SIZE;
                        }
                        NATIVE_WORD_NODE_CNL | NATIVE_WORD_NODE_AT_CNL => {
                            let sz = mem.b16s3(p + 4) as i32;
                            r = cnl_get_node!(mem, sz);
                            let mut w = sz;
                            while w > 0 {
                                w -= 1;
                                mem.set_word(r + w, mem.word(p + w));
                            }
                            mem.set_ptr(r + 5, std::ptr::null_mut());
                            mem.set_b16s0(r + 4, 0);
                            crate::mem::copy_native_glyph_info(mem, p, r);
                            words = 0;
                        }
                        GLYPH_NODE_CNL => {
                            r = cnl_get_node!(mem, GLYPH_NODE_SIZE_CNL);
                            words = GLYPH_NODE_SIZE_CNL;
                        }
                        PIC_NODE_CNL | PDF_NODE_CNL => {
                            let path_len = mem.b16s1(p + 4) as i32;
                            let sz = PIC_NODE_SIZE_CNL
                                + (path_len + MEMORY_WORD_SIZE_CNL - 1) / MEMORY_WORD_SIZE_CNL;
                            r = cnl_get_node!(mem, sz);
                            words = sz;
                        }
                        PDF_SAVE_POS_NODE_CNL => {
                            r = cnl_get_node!(mem, SMALL_NODE_SIZE);
                        }
                        _ => {
                            return Err(NodeRecycleAbort::Confusion(c"ext2"));
                        }
                    }
                }
                GLUE_NODE => {
                    r = cnl_get_node!(mem, MEDIUM_NODE_SIZE);
                    // GLUE_SPEC_ref_count(mem[p+1].b32.s0)++ = mem[mem[p+1].b32.s0].b32.s1++
                    let spec = mem.b32s0(p + 1);
                    mem.set_b32s1(spec, mem.b32s1(spec) + 1);
                    mem.set_b32s0(r + 2, mem.b32s0(p + 2));
                    mem.set_b32s1(r + 2, mem.b32s1(p + 2));
                    mem.set_b32s0(r + 1, mem.b32s0(p + 1));
                    let src = mem.b32s1(p + 1);
                    let copied = copy_node_list(mem, src)?;
                    mem.set_b32s1(r + 1, copied);
                }
                KERN_NODE | MATH_NODE | PENALTY_NODE => {
                    r = cnl_get_node!(mem, MEDIUM_NODE_SIZE);
                    words = MEDIUM_NODE_SIZE;
                }
                MARGIN_KERN_NODE_CNL => {
                    r = cnl_get_node!(mem, MARGIN_KERN_NODE_SIZE_CNL);
                    words = MARGIN_KERN_NODE_SIZE_CNL;
                }
                LIGATURE_NODE => {
                    r = cnl_get_node!(mem, SMALL_NODE_SIZE);
                    mem.set_word(r + 1, mem.word(p + 1));
                    let src = mem.b32s1(p + 1);
                    let copied = copy_node_list(mem, src)?;
                    mem.set_b32s1(r + 1, copied);
                }
                DISC_NODE => {
                    r = cnl_get_node!(mem, SMALL_NODE_SIZE);
                    let src0 = mem.b32s0(p + 1);
                    let copied0 = copy_node_list(mem, src0)?;
                    mem.set_b32s0(r + 1, copied0);
                    let src1 = mem.b32s1(p + 1);
                    let copied1 = copy_node_list(mem, src1)?;
                    mem.set_b32s1(r + 1, copied1);
                }
                MARK_NODE_CNL => {
                    r = cnl_get_node!(mem, SMALL_NODE_SIZE);
                    // mem[mem[p+1].b32.s1].b32.s0++ (token list ref count)
                    let tok = mem.b32s1(p + 1);
                    mem.set_b32s0(tok, mem.b32s0(tok) + 1);
                    words = SMALL_NODE_SIZE;
                }
                ADJUST_NODE_CNL => {
                    r = cnl_get_node!(mem, SMALL_NODE_SIZE);
                    let src = mem.b32s1(p + 1);
                    let copied = copy_node_list(mem, src)?;
                    mem.set_b32s1(r + 1, copied);
                }
                _ => {
                    return Err(NodeRecycleAbort::Confusion(c"copying"));
                }
            }
        }
        let mut w = words;
        while w > 0 {
            w -= 1;
            mem.set_word(r + w, mem.word(p + w));
        }
        mem.set_b32s1(q, r);
        q = r;
        p = mem.b32s1(p);
    }
    mem.set_b32s1(q, TEX_NULL);
    let result = mem.b32s1(h);
    let av = mem.avail;
    mem.set_b32s1(h, av);
    mem.avail = h;
    Ok(result)
}
/// Transitional bridge-down wrapper for legacy `copy_node_list` callers.
pub unsafe fn copy_node_list_global(p: i32) -> i32 {
    match crate::state::with_state_mut(|st| copy_node_list(&mut st.mem, p)) {
        Ok(r) => r,
        Err(e) => node_recycle_abort(e),
    }
}
