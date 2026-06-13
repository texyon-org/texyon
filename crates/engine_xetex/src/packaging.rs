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
    BOX_BASE, BOX_NODE_SIZE, DIMEN_BASE, DISC_NODE, GLUE_BASE, GLUE_NODE, GLUE_SPEC_SIZE,
    HLIST_NODE, INT_BASE, KERN_NODE, LIGATURE_NODE, MATH_NODE, MAX_HALFWORD, MEDIUM_NODE_SIZE,
    MIN_HALFWORD, PENALTY_NODE, RULE_NODE, RULE_NODE_SIZE, SMALL_NODE_SIZE,
};
use crate::errors::error;
use crate::ext::measure_native_node;
use crate::fonts::{effective_char, new_native_word_node};
use crate::globals::{
    active_width, cur_val, eqtb_b32s1, mem_b16s0, mem_b16s1, mem_b32s0, mem_b32s1,
    set_active_width, set_best_height_plus_depth, set_cur_val, set_eqtb_b32s1, set_help_line,
    set_help_ptr, set_mem_b16s0, set_mem_b32s0, set_mem_b32s1, temp_ptr,
};
use crate::mem::new_spec_global;
use crate::node::{
    AnyBox, GlueNode, GlueSpecNode, HListNode, KernNode, MathNode, Noad, NodePtr, RuleDimen,
    RuleNode, VListNode,
};
use crate::output::{capture_to_diagnostic, error_here_with_diagnostic, short_display, show_box};

// Shared node-recycle infrastructure stays in scan.rs (also used by
// prune_page_top and the relocated font-loading group); import it here.
use crate::scan::{cnl_get_avail, cnl_get_node, node_recycle_abort, NodeRecycleAbort};
use crate::state::{EqtbState, FontState, MemState, ScannerState, ShipoutState};
use crate::units::{MemIdx, MemPtr, NodeType, Scaled};

// Module-private consts shared with scan.rs (TEMP_HEAD/IGNORE_DEPTH still used
// there too, so duplicated rather than relocated — same value).
const TEMP_HEAD: i32 = 4_999_999 - 3;
const IGNORE_DEPTH: i32 = -65_536_000; // xetex_format.h:388

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

// ---------------------------------------------------------------------------
// prune_page_top
// ---------------------------------------------------------------------------

// xetex_format.h:159
const GLUE_PAR_SPLIT_TOP_SKIP_PPT: i16 = 10;
const VLIST_NODE_PPT: u16 = 1; // xetex_format.h:381
const WHATSIT_NODE_PPT: u16 = 8; // xetex_format.h:388
const MARK_NODE_PPT: u16 = 4; // xetex_format.h:384
const INS_NODE_PPT: u16 = 3; // xetex_format.h:383
const VSPLIT_CODE_PPT: usize = 3; // xetex_format.h:483

// ---------------------------------------------------------------------------
// do_marks
// ---------------------------------------------------------------------------

// xetex-constants.h:381–384
const VSPLIT_INIT_DM: i16 = 0;
const FIRE_UP_INIT_DM: i16 = 1;
const FIRE_UP_DONE_DM: i16 = 2;
const DESTROY_MARKS_DM: i16 = 3;
// xetex_format.h:685, 689
const TOP_MARK_CODE_DM: i16 = 0;
const SPLIT_BOT_MARK_CODE_DM: i16 = 4;
// xetex-constants.h:134, 123
const INDEX_NODE_SIZE_DM: i32 = 33;
const MARK_CLASS_NODE_SIZE_DM: i32 = 4;

// ---------------------------------------------------------------------------
// hpack
// ---------------------------------------------------------------------------

// Node type constants for hpack
const WHATSIT_NODE_HP: u16 = 8;
// whatsit subtypes
const NATIVE_WORD_NODE_HP: u16 = 40;
const NATIVE_WORD_NODE_AT_HP: u16 = 41;
// GLYPH_NODE = 42, PIC/PDF = 43/44: contribute width/height/depth
const A_LEADERS_HP: u16 = 100;
// glue orders
const NORMAL_HP: usize = 0;
const FIL_HP: usize = 1;
const FILL_HP: usize = 2;
const FILLL_HP: usize = 3;
const STRETCHING_HP: u16 = 1;
const SHRINKING_HP: u16 = 2;
// Bidi
const BEFORE_HP: u32 = 0;
const L_CODE_HP: u16 = 4;
const EXPLICIT_HP: u16 = 1;
// INT/DIMEN params
const INT_PAR_TEXXET_HP: i32 = 69;
const INT_PAR_HBADNESS_HP: i32 = 26;
const DIMEN_PAR_HFUZZ_HP: i32 = 8;
const DIMEN_PAR_OVERFULL_RULE_HP: i32 = 16;
const NATIVE_NODE_SIZE_HP: i32 = 6;

#[derive(Copy, Clone)]
enum PackDiag {
    HUnderfull { badness: i32, loose: bool },
    HOverfull { excess: i32 },
    HTight { badness: i32 },
    LrProblem { missing: i32, extra: i32 },
    VUnderfull { badness: i32, loose: bool },
    VOverfull { excess: i32 },
    VTight { badness: i32 },
}

#[derive(Copy, Clone)]
struct PackOutcome {
    r: i32,
    diags: [Option<PackDiag>; 2],
}

fn push_pack_diag(diags: &mut [Option<PackDiag>; 2], diag: PackDiag) {
    if diags[0].is_none() {
        diags[0] = Some(diag);
    } else {
        diags[1] = Some(diag);
    }
}

// ---------------------------------------------------------------------------
// vpackage
// ---------------------------------------------------------------------------

// xetex-constants.h:132, 173–175, 182, 275, 281, 422
const NORMAL_VP: usize = 0;
const FIL_VP: usize = 1;
const FILL_VP: usize = 2;
const FILLL_VP: usize = 3;
const STRETCHING_VP: u16 = 1;
const SHRINKING_VP: u16 = 2;
const A_LEADERS_VP: u16 = 100;
// xetex_format.h:381, 383–384, 466–467
const PIC_NODE_VP: u16 = 43;
const PDF_NODE_VP: u16 = 44;
// xetex_format.h:71, 106, 131
const INT_PAR_XETEX_UPWARDS_VP: i32 = 71;
const INT_PAR_VBADNESS_VP: i32 = 27;
const DIMEN_PAR_VFUZZ_VP: i32 = 9;

// ---- moved verbatim from scan.rs (workstream 02) ----

// translated from xetex-xetex0.c:8029 (just_copy)
pub fn just_copy(
    mem: &mut crate::state::MemState,
    p: i32,
    h: i32,
    t: i32,
) -> Result<(), NodeRecycleAbort> {
    use crate::mem::TEX_NULL;

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
        let r_opt: Option<i32> = if p >= mem.hi_mem_min {
            // char node
            Some(cnl_get_avail!(mem))
        } else {
            match mem.b16s1(p) {
                HLIST_NODE | VLIST_NODE => {
                    let r = cnl_get_node!(mem, BOX_NODE_SIZE);
                    // SYNCTEX_tag(r, BOX_NODE_SIZE) = SYNCTEX_tag(p, BOX_NODE_SIZE)
                    mem.set_b32s0(
                        r + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                        mem.b32s0(p + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE),
                    );
                    // SYNCTEX_line(r, BOX_NODE_SIZE) = SYNCTEX_line(p, BOX_NODE_SIZE)
                    mem.set_b32s1(
                        r + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                        mem.b32s1(p + BOX_NODE_SIZE - SYNCTEX_FIELD_SIZE),
                    );
                    mem.set_word(r + 6, mem.word(p + 6));
                    mem.set_word(r + 5, mem.word(p + 5));
                    words = 5;
                    mem.set_b32s1(r + 5, TEX_NULL); // BOX_list_ptr(r) = TEX_NULL
                    Some(r)
                }
                RULE_NODE => {
                    words = RULE_NODE_SIZE;
                    Some(cnl_get_node!(mem, RULE_NODE_SIZE))
                }
                LIGATURE_NODE => {
                    let r = cnl_get_avail!(mem);
                    mem.set_word(r, mem.word(p + 1)); // mem[r] = mem[p + 1]
                    words = 0; // skip the copy loop (goto found)
                    Some(r)
                }
                KERN_NODE | MATH_NODE => {
                    words = MEDIUM_NODE_SIZE;
                    Some(cnl_get_node!(mem, MEDIUM_NODE_SIZE))
                }
                GLUE_NODE => {
                    let r = cnl_get_node!(mem, MEDIUM_NODE_SIZE);
                    // GLUE_SPEC_ref_count(GLUE_NODE_glue_ptr(p))++
                    let gp = mem.b32s0(p + 1); // GLUE_NODE_glue_ptr(p)
                    let spec =
                        NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(gp)).unwrap();
                    spec.set_ref_count(mem, spec.ref_count(mem) + 1);
                    // SYNCTEX fields
                    mem.set_b32s0(
                        r + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                        mem.b32s0(p + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE),
                    );
                    mem.set_b32s1(
                        r + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                        mem.b32s1(p + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE),
                    );
                    mem.set_b32s0(r + 1, gp); // GLUE_NODE_glue_ptr(r) = gp
                    mem.set_b32s1(r + 1, TEX_NULL); // GLUE_NODE_leader_ptr(r) = TEX_NULL
                    Some(r)
                }
                WHATSIT_NODE => {
                    match mem.b16s0(p) {
                        OPEN_NODE => {
                            words = OPEN_NODE_SIZE;
                            Some(cnl_get_node!(mem, OPEN_NODE_SIZE))
                        }
                        WRITE_NODE | SPECIAL_NODE => {
                            let r = cnl_get_node!(mem, WRITE_NODE_SIZE);
                            // TOKEN_LIST_ref_count(WRITE_NODE_tokens(p))++
                            let toks = mem.b32s1(p + 1); // WRITE_NODE_tokens(p)
                            mem.set_b32s0(toks, mem.b32s0(toks) + 1);
                            words = WRITE_NODE_SIZE;
                            Some(r)
                        }
                        CLOSE_NODE | LANGUAGE_NODE => {
                            words = SMALL_NODE_SIZE;
                            Some(cnl_get_node!(mem, SMALL_NODE_SIZE))
                        }
                        NATIVE_WORD_NODE | NATIVE_WORD_NODE_AT => {
                            let sz = mem.b16s3(p + 4) as i32; // NATIVE_NODE_size(p)
                            let r = cnl_get_node!(mem, sz);
                            let mut w = sz;
                            while w > 0 {
                                w -= 1;
                                mem.set_word(r + w, mem.word(p + w));
                            }
                            // NATIVE_NODE_glyph_info_ptr(r) = NULL
                            mem.set_ptr(r + 5, std::ptr::null_mut());
                            // NATIVE_NODE_glyph_count(r) = 0
                            mem.set_b16s0(r + 4, 0);
                            crate::mem::copy_native_glyph_info(mem, p, r);
                            words = 0; // already copied, skip outer loop
                            Some(r)
                        }
                        GLYPH_NODE => {
                            words = GLYPH_NODE_SIZE;
                            Some(cnl_get_node!(mem, GLYPH_NODE_SIZE))
                        }
                        PIC_NODE | PDF_NODE => {
                            let path_len = mem.b16s1(p + 4) as i32; // PIC_NODE_path_len
                            words =
                                PIC_NODE_SIZE + (path_len + MEM_WORD_BYTES - 1) / MEM_WORD_BYTES;
                            Some(cnl_get_node!(mem, words))
                        }
                        PDF_SAVE_POS_NODE => Some(cnl_get_node!(mem, SMALL_NODE_SIZE)),
                        _ => {
                            return Err(NodeRecycleAbort::Confusion(c"ext2"));
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
                mem.set_word(r + w, mem.word(p + w));
            }
            // found: link r into h
            mem.set_b32s1(h, r); // LLIST_link(h) = r
            h = r;
        }

        // not_found: advance p
        p = mem.b32s1(p); // LLIST_link(p)
    }

    mem.set_b32s1(h, t); // LLIST_link(h) = t
    Ok(())
}

/// Transitional bridge-down wrapper for legacy `just_copy` callers (`01b` §3,
/// §5). Runs the threaded copy under a single `mem` borrow, then performs the
/// deferred cold abort (impossible corrupt-node `confusion`, or arena
/// exhaustion `overflow`/`runaway`) after the borrow drops.
pub unsafe fn just_copy_global(p: i32, h: i32, t: i32) {
    if let Err(e) = crate::state::with_state_mut(|st| just_copy(&mut st.mem, p, h, t)) {
        node_recycle_abort(e);
    }
}
// translated from xetex-xetex0.c:8155 (just_reverse)
//
// Phase-C: threaded to `&mut MemState` + `&mut ScannerState`. The mem arena is
// touched directly; the `cur_dir`/`LR_ptr`/`LR_problems` scanner scalars are
// read/written as `scanner` fields; `temp_ptr` stays the raw global accessor
// (safe under the borrow per `01b` §4). Calls the threaded `just_copy`,
// `flush_node_list`, `new_edge`, `get_avail`, and `free_node`; arena exhaustion
// and the corrupt-node `confusion` propagate the `01b` §5 [`NodeRecycleAbort`]
// sentinel, run by [`just_reverse_global`] after the borrow drops. FFI-free
// (no C++ measurer / `new_native_character` / print / scanner re-entry).
pub fn just_reverse(
    mem: &mut crate::state::MemState,
    scanner: &mut crate::state::ScannerState,
    p: i32,
) -> Result<(), NodeRecycleAbort> {
    use crate::globals::{set_temp_ptr, temp_ptr};
    use crate::mem::TEX_NULL;

    const TEMP_HEAD: i32 = 4_999_996; // MEM_TOP - 3
    const L_CODE: i32 = 4; // xetex-constants.h:142
    const R_CODE: i32 = 8; // xetex-constants.h:143

    let mut p = p;
    let mut m = MIN_HALFWORD;
    let mut n = MIN_HALFWORD;

    let mut q = if mem.b32s1(TEMP_HEAD) == TEX_NULL {
        let src = mem.b32s1(p);
        just_copy(mem, src, TEMP_HEAD, TEX_NULL)?;
        mem.b32s1(TEMP_HEAD)
    } else {
        let q = mem.b32s1(p);
        mem.set_b32s1(p, TEX_NULL);
        let head = mem.b32s1(TEMP_HEAD);
        flush_node_list(mem, head).map_err(NodeRecycleAbort::Confusion)?;
        q
    };

    let t = crate::shipout::new_edge(mem, scanner.cur_dir, 0);
    if t == crate::mem::ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    let mut l = t;
    scanner.cur_dir = 1 - scanner.cur_dir;

    'done: loop {
        while q != TEX_NULL {
            if q >= mem.hi_mem_min {
                // char node do-while
                loop {
                    p = q;
                    q = mem.b32s1(p);
                    mem.set_b32s1(p, l);
                    l = p;
                    if q < mem.hi_mem_min {
                        break;
                    }
                }
            } else {
                p = q;
                q = mem.b32s1(p);

                if mem.b16s1(p) == MATH_NODE {
                    let subtype = mem.b16s0(p) as i32;
                    if subtype & 1 != 0 {
                        // odd subtype = end math node
                        let expected = L_CODE * (subtype / L_CODE) + 3;
                        if mem.b32s0(scanner.LR_ptr) != expected {
                            mem.set_b16s1(p, KERN_NODE); // NODE_type(p) = KERN_NODE
                            scanner.LR_problems += 1;
                        } else {
                            // pop LR_ptr
                            set_temp_ptr(scanner.LR_ptr);
                            scanner.LR_ptr = mem.b32s1(temp_ptr());
                            mem.set_b32s1(temp_ptr(), mem.avail);
                            mem.avail = temp_ptr();

                            if n > MIN_HALFWORD {
                                n -= 1;
                                mem.set_b16s0(p, mem.b16s0(p) - 1);
                            } else if m > MIN_HALFWORD {
                                m -= 1;
                                mem.set_b16s1(p, KERN_NODE);
                            } else {
                                // found edge end: set t's link and free p
                                mem.set_b32s1(t + 1, mem.b32s1(p + 1));
                                mem.set_b32s1(t, q);
                                crate::mem::free_node(mem, p, MEDIUM_NODE_SIZE);
                                break 'done;
                            }
                        }
                    } else {
                        // even subtype = begin math node
                        // push LR_ptr
                        let tp = cnl_get_avail!(mem);
                        set_temp_ptr(tp);
                        mem.set_b32s0(temp_ptr(), L_CODE * (subtype / L_CODE) + 3);
                        mem.set_b32s1(temp_ptr(), scanner.LR_ptr);
                        scanner.LR_ptr = temp_ptr();

                        if n > MIN_HALFWORD || (subtype / R_CODE) != scanner.cur_dir as i32 {
                            n += 1;
                            mem.set_b16s0(p, mem.b16s0(p) + 1);
                        } else {
                            mem.set_b16s1(p, KERN_NODE);
                            m += 1;
                        }
                    }
                }

                mem.set_b32s1(p, l);
                l = p;
            }
        }
        break; // goto done (past dead code)
    }

    // done:
    mem.set_b32s1(TEMP_HEAD, l);
    Ok(())
}

/// Transitional bridge-down wrapper for legacy `just_reverse` callers (`01b`
/// §3, §5). Runs the threaded reversal under one `mem`+`scanner` borrow, then
/// performs the deferred cold abort after the borrow drops.
pub unsafe fn just_reverse_global(p: i32) {
    if let Err(e) = crate::state::with_state_mut(|st| just_reverse(&mut st.mem, &mut st.scanner, p))
    {
        node_recycle_abort(e);
    }
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
                        // SAFETY: the `NODE_type` match arm proves this is a glue node.
                        let glue =
                            unsafe { NodePtr::<GlueNode>::from_idx_unchecked(MemIdx::new(p)) };
                        let spec = glue.glue_spec(mem).expect("glue node has a glue_spec");
                        if spec.ref_count(mem) == crate::mem::TEX_NULL {
                            crate::mem::free_node(mem, spec.idx().0, GLUE_SPEC_SIZE);
                        } else {
                            spec.set_ref_count(mem, spec.ref_count(mem) - 1);
                        }
                        let leader = glue.leader_ptr(mem);
                        if leader.0 != crate::mem::TEX_NULL {
                            flush_node_list(mem, leader.0)?;
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
                        let p_noad = unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(p)) };
                        if p_noad.nucleus_type(mem) >= SUB_BOX_FNL {
                            let c = p_noad.nucleus_field(mem);
                            flush_node_list(mem, c)?;
                        }
                        if p_noad.supscr_type(mem) >= SUB_BOX_FNL {
                            let c = p_noad.supscr_field(mem);
                            flush_node_list(mem, c)?;
                        }
                        if p_noad.subscr_type(mem) >= SUB_BOX_FNL {
                            let c = p_noad.subscr_field(mem);
                            flush_node_list(mem, c)?;
                        }
                        let ntype = p_noad.noad_type(mem).as_raw();
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
                        let p_noad = unsafe { NodePtr::<Noad>::from_idx_unchecked(MemIdx::new(p)) };
                        let c1 = p_noad.supscr_field(mem);
                        flush_node_list(mem, c1)?;
                        let c2 = p_noad.subscr_field(mem);
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
                    let spec =
                        NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(mem.b32s1(p + 4)))
                            .unwrap();
                    spec.set_ref_count(mem, spec.ref_count(mem) + 1);
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
                    let spec =
                        NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(mem.b32s0(p + 1)))
                            .unwrap();
                    spec.set_ref_count(mem, spec.ref_count(mem) + 1);
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

// ---- moved verbatim from scan.rs (workstream 02) ----

// translated from xetex-xetex0.c:8020 (append_to_vlist)
//
// Phase-C: threaded to `&mut MemState` + `&EqtbState`. The mem arena is touched
// directly; baseline-skip parameters are read as `eqtb` fields; the `cur_list`
// aux/tail scalars and `temp_ptr` stay raw global accessors (safe under the
// borrow per `01b` §4). The two glue constructors `new_param_glue`/
// `new_skip_param` are threaded; arena exhaustion propagates the `01b` §5
// [`NodeRecycleAbort::Overflow`] sentinel, run by [`append_to_vlist_global`]
// after the borrow drops. FFI-free (no measurer / print / scanner re-entry).
pub fn append_to_vlist(
    mem: &mut crate::state::MemState,
    eqtb: &crate::state::EqtbState,
    b: AnyBox,
) -> Result<(), NodeRecycleAbort> {
    const INT_PAR_XETEX_UPWARDS: i32 = 71; // xetex_format.h:106
    const GLUE_PAR_BASELINE_SKIP: i32 = 1; // xetex_format.h:150
    const GLUE_PAR_LINE_SKIP: i16 = 0; // xetex_format.h:149
    const DIMEN_PAR_LINE_SKIP_LIMIT: i32 = 2; // xetex_format.h:124

    let upwards = eqtb.b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS) > 0;
    let b_idx = b.idx().0;
    if crate::globals::cur_list_aux_b32s1() > IGNORE_DEPTH {
        let baseline_spec = eqtb.b32s1(GLUE_BASE + GLUE_PAR_BASELINE_SKIP);
        let d = mem.b32s1(baseline_spec + 1)
            - crate::globals::cur_list_aux_b32s1()
            - if upwards {
                b.depth(mem).0
            } else {
                b.height(mem).0
            };
        let p = if d < eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_LINE_SKIP_LIMIT) {
            let p = crate::mem::new_param_glue(mem, eqtb, GLUE_PAR_LINE_SKIP);
            if p == crate::mem::ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow);
            }
            p
        } else {
            let p = crate::mem::new_skip_param(mem, eqtb, GLUE_PAR_BASELINE_SKIP as i16);
            if p == crate::mem::ALLOC_FAILED {
                return Err(NodeRecycleAbort::Overflow);
            }
            mem.set_b32s1(crate::globals::temp_ptr() + 1, d);
            p
        };
        mem.set_b32s1(crate::globals::cur_list_tail(), p);
        crate::globals::set_cur_list_tail(p);
    }
    mem.set_b32s1(crate::globals::cur_list_tail(), b_idx);
    crate::globals::set_cur_list_tail(b_idx);
    crate::globals::set_cur_list_aux_b32s1(if upwards {
        b.height(mem).0
    } else {
        b.depth(mem).0
    });
    Ok(())
}

/// Transitional bridge-down wrapper for legacy `append_to_vlist` callers (`01b`
/// §3, §5). Runs the threaded append under one `mem`+`eqtb` borrow, then runs
/// the deferred arena-exhaustion `overflow` after the borrow drops.
pub unsafe fn append_to_vlist_global(b: i32) {
    if let Err(e) = crate::state::with_state_mut(|st| {
        let s = &mut *st;
        let b = s
            .mem
            .classify_box(MemIdx::new(b))
            .expect("valid node type for append_to_vlist")
            .expect("append_to_vlist receives an hlist or vlist node");
        append_to_vlist(&mut s.mem, &s.eqtb, b)
    }) {
        node_recycle_abort(e);
    }
}
// translated from xetex-xetex0.c:7725 (vert_break)
pub unsafe fn vert_break(p: i32, h: i32, d: i32) -> i32 {
    use crate::mem::TEX_NULL;

    const EJECT_PENALTY: i32 = -10_000; // xetex-constants.h:336
    const INF_PENALTY: i32 = 10_000; // xetex-constants.h:338
    const INF_BAD: i32 = 10_000; // xetex-constants.h:337
    const PIC_NODE: u16 = 43; // xetex_format.h:466
    const PDF_NODE: u16 = 44; // xetex_format.h:467
    const NORMAL: u16 = 0;

    let mut p = p;
    let mut prev_p = p;
    let mut least_cost = MAX_HALFWORD;
    let mut best_place = TEX_NULL;
    let mut prev_dp: i32 = 0;
    for k in 1..=6usize {
        set_active_width(k, 0);
    }

    enum Next {
        NotFound,
        UpdateHeights,
        CheckPi(i32),
    }

    'done: loop {
        let next = if p == TEX_NULL {
            Next::CheckPi(EJECT_PENALTY)
        } else {
            match mem_b16s1(p) {
                0..=2 => {
                    set_active_width(1, active_width(1) + prev_dp + mem_b32s1(p + 3));
                    prev_dp = mem_b32s1(p + 2);
                    Next::NotFound
                }
                8 => {
                    let sub = mem_b16s0(p);
                    if sub == PIC_NODE || sub == PDF_NODE {
                        set_active_width(1, active_width(1) + prev_dp + mem_b32s1(p + 3));
                        prev_dp = mem_b32s1(p + 2);
                    }
                    Next::NotFound
                }
                10 => {
                    // GLUE_NODE
                    if mem_b16s1(prev_p) < MATH_NODE {
                        // is_non_discardable_node
                        Next::CheckPi(0)
                    } else {
                        Next::UpdateHeights
                    }
                }
                11 => {
                    // KERN_NODE
                    let link = mem_b32s1(p);
                    let t = if link == TEX_NULL {
                        PENALTY_NODE
                    } else {
                        mem_b16s1(link)
                    };
                    if t == GLUE_NODE {
                        Next::CheckPi(0)
                    } else {
                        Next::UpdateHeights
                    }
                }
                12 => Next::CheckPi(mem_b32s1(p + 1)), // PENALTY_NODE
                3 | 4 => Next::NotFound,
                _ => crate::errors::confusion(c"vertbreak".as_ptr()),
            }
        };

        let do_update = match next {
            Next::NotFound => false,
            Next::UpdateHeights => true,
            Next::CheckPi(pi) => {
                if pi < INF_PENALTY {
                    let b = if active_width(1) < h {
                        if active_width(3) != 0 || active_width(4) != 0 || active_width(5) != 0 {
                            0
                        } else {
                            crate::helpers::badness(h - active_width(1), active_width(2))
                        }
                    } else if active_width(1) - h > active_width(6) {
                        MAX_HALFWORD
                    } else {
                        crate::helpers::badness(active_width(1) - h, active_width(6))
                    };
                    let b = if b < MAX_HALFWORD {
                        if pi <= EJECT_PENALTY {
                            pi
                        } else if b < INF_BAD {
                            b + pi
                        } else {
                            100_000
                        }
                    } else {
                        b
                    };
                    if b <= least_cost {
                        best_place = p;
                        least_cost = b;
                        set_best_height_plus_depth(active_width(1) + prev_dp);
                    }
                    if b == MAX_HALFWORD || pi <= EJECT_PENALTY {
                        break 'done;
                    }
                }
                // reached only when pi >= INF_PENALTY, or (pi < INF_PENALTY and did not break);
                // in the TEX_NULL case pi == EJECT_PENALTY so we always break above — p is valid.
                let nt = mem_b16s1(p);
                (GLUE_NODE..=KERN_NODE).contains(&nt)
            }
        };

        if do_update {
            let q = if mem_b16s1(p) == KERN_NODE {
                p
            } else {
                let q = mem_b32s0(p + 1);
                let qi = mem_b16s1(q) as usize;
                set_active_width(2 + qi, active_width(2 + qi) + mem_b32s1(q + 2));
                set_active_width(6, active_width(6) + mem_b32s1(q + 3));
                if mem_b16s0(q) != NORMAL && mem_b32s1(q + 3) != 0 {
                    error_here_with_diagnostic(
                        c"Infinite glue shrinkage found in box being split".as_ptr(),
                    );
                    capture_to_diagnostic(std::ptr::null_mut());
                    set_help_ptr(4);
                    set_help_line(
                        3,
                        c"The box you are \\vsplitting contains some infinitely".as_ptr(),
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
                    let r = new_spec_global(q);
                    set_mem_b16s0(r, NORMAL); // GLUE_SPEC_shrink_order(r) = NORMAL
                    crate::mem::delete_glue_ref_global(q);
                    set_mem_b32s0(p + 1, r);
                    r
                } else {
                    q
                }
            };
            set_active_width(1, active_width(1) + prev_dp + mem_b32s1(q + 1));
            prev_dp = 0;
        }

        // not_found:
        if prev_dp > d {
            set_active_width(1, active_width(1) + prev_dp - d);
            prev_dp = d;
        }
        prev_p = p;
        p = mem_b32s1(prev_p);
    }
    best_place
}
// translated from xetex-xetex0.c:7727 (vsplit)
pub unsafe fn vsplit(n: i32, h: i32) -> i32 {
    use crate::globals::{
        cur_mark, cur_ptr, disc_ptr, sa_root, set_cur_mark, set_disc_ptr, set_sa_root,
    };
    use crate::mem::TEX_NULL;

    const DIMEN_PAR_SPLIT_MAX_DEPTH: i32 = 6; // xetex_format.h:128
    const INT_PAR_SAVING_VDISCARDS: i32 = 63; // xetex_format.h:98
    const EXACTLY: i16 = 0; // xetex-constants.h:256
    const ADDITIONAL: i16 = 1; // xetex-constants.h:264
    const VSPLIT_CODE: usize = 3; // xetex_format.h:483
    const MARK_VAL: usize = 7; // xetex-constants.h:300
    const VLIST_NODE: u16 = 1; // xetex_format.h:381
    const MARK_NODE: u16 = 4; // xetex_format.h:384
    const SPLIT_FIRST_MARK_CODE: usize = 3; // xetex_format.h:688
    const SPLIT_BOT_MARK_CODE: usize = 4; // xetex_format.h:689

    set_cur_val(n);
    let v = if cur_val() < 256 {
        eqtb_b32s1(BOX_BASE + cur_val())
    } else {
        crate::save::find_sa_element_global(4, cur_val(), false);
        if cur_ptr() == TEX_NULL {
            TEX_NULL
        } else {
            mem_b32s1(cur_ptr() + 1)
        }
    };

    flush_node_list_global(disc_ptr(VSPLIT_CODE));
    set_disc_ptr(VSPLIT_CODE, TEX_NULL);

    if sa_root(MARK_VAL) != TEX_NULL && do_marks_global(0, 0, sa_root(MARK_VAL)) {
        set_sa_root(MARK_VAL, TEX_NULL);
    }

    if cur_mark(SPLIT_FIRST_MARK_CODE) != TEX_NULL {
        crate::mem::delete_token_ref_global(cur_mark(SPLIT_FIRST_MARK_CODE));
        set_cur_mark(SPLIT_FIRST_MARK_CODE, TEX_NULL);
        crate::mem::delete_token_ref_global(cur_mark(SPLIT_BOT_MARK_CODE));
        set_cur_mark(SPLIT_BOT_MARK_CODE, TEX_NULL);
    }

    if v == TEX_NULL {
        return TEX_NULL;
    }

    if mem_b16s1(v) != VLIST_NODE {
        error_here_with_diagnostic(c"".as_ptr());
        crate::output::print_esc_cstr(c"vsplit".as_ptr());
        crate::output::print_cstr(c" needs a ".as_ptr());
        crate::output::print_esc_cstr(c"vbox".as_ptr());
        capture_to_diagnostic(std::ptr::null_mut());
        set_help_ptr(2);
        set_help_line(1, c"The box you are trying to split is an \\hbox.".as_ptr());
        set_help_line(
            0,
            c"I can't split such a box, so I'll leave it alone.".as_ptr(),
        );
        error();
        return TEX_NULL;
    }

    let split_max_d = eqtb_b32s1(DIMEN_BASE + DIMEN_PAR_SPLIT_MAX_DEPTH);
    let q = vert_break(mem_b32s1(v + 5), h, split_max_d);
    let mut p = mem_b32s1(v + 5);

    if p == q {
        set_mem_b32s1(v + 5, TEX_NULL);
    } else {
        loop {
            if mem_b16s1(p) == MARK_NODE {
                let mark_class = mem_b32s0(p + 1);
                let mark_ptr = mem_b32s1(p + 1);
                if mark_class != 0 {
                    crate::save::find_sa_element_global(MARK_VAL as u16, mark_class, true);
                    if mem_b32s1(cur_ptr() + 2) == TEX_NULL {
                        set_mem_b32s1(cur_ptr() + 2, mark_ptr);
                        set_mem_b32s0(mark_ptr, mem_b32s0(mark_ptr) + 1);
                    } else {
                        crate::mem::delete_token_ref_global(mem_b32s0(cur_ptr() + 3));
                    }
                    set_mem_b32s0(cur_ptr() + 3, mark_ptr);
                    set_mem_b32s0(mark_ptr, mem_b32s0(mark_ptr) + 1);
                } else if cur_mark(SPLIT_FIRST_MARK_CODE) == TEX_NULL {
                    set_cur_mark(SPLIT_FIRST_MARK_CODE, mark_ptr);
                    set_cur_mark(SPLIT_BOT_MARK_CODE, mark_ptr);
                    set_mem_b32s0(mark_ptr, mem_b32s0(mark_ptr) + 2);
                } else {
                    crate::mem::delete_token_ref_global(cur_mark(SPLIT_BOT_MARK_CODE));
                    set_cur_mark(SPLIT_BOT_MARK_CODE, mark_ptr);
                    set_mem_b32s0(mark_ptr, mem_b32s0(mark_ptr) + 1);
                }
            }
            if mem_b32s1(p) == q {
                set_mem_b32s1(p, TEX_NULL);
                break;
            }
            p = mem_b32s1(p);
        }
    }

    let mut q = prune_page_top_global(q, eqtb_b32s1(INT_BASE + INT_PAR_SAVING_VDISCARDS) > 0);
    p = mem_b32s1(v + 5);
    crate::mem::free_node_global(v, BOX_NODE_SIZE);
    if q != TEX_NULL {
        q = vpackage(q, 0, ADDITIONAL, MAX_HALFWORD);
    }
    if cur_val() < 256 {
        set_eqtb_b32s1(BOX_BASE + cur_val(), q);
    } else {
        crate::save::find_sa_element_global(4, cur_val(), false);
        if cur_ptr() != TEX_NULL {
            set_mem_b32s1(cur_ptr() + 1, q);
            set_mem_b32s0(cur_ptr() + 1, mem_b32s0(cur_ptr() + 1) + 1);
            crate::save::delete_sa_ref_global(cur_ptr());
        }
    }
    vpackage(p, h, EXACTLY, split_max_d)
}
/// translated from xetex-xetex0.c:9761 (prune_page_top)
///
/// Phase-C: threaded to disjoint `&mut MemState` / `&mut ScannerState` /
/// `&EqtbState` borrows. Walks the mem arena directly, allocates a split-top
/// skip glue via the threaded `new_skip_param`, frees discarded nodes via the
/// threaded `flush_node_list`, and touches `disc_ptr` on `ScannerState`
/// directly; `temp_ptr` is a raw `mod raw` global (safe under the borrow).
/// Arena exhaustion and the impossible corrupt-node `confusion` arm return the
/// `01b` §5 [`NodeRecycleAbort`] sentinel, which [`prune_page_top_global`]
/// fires after the borrow drops. Legacy callers use [`prune_page_top_global`].
pub fn prune_page_top(
    mem: &mut crate::state::MemState,
    scanner: &mut crate::state::ScannerState,
    eqtb: &crate::state::EqtbState,
    p: i32,
    s: bool,
) -> Result<i32, NodeRecycleAbort> {
    use crate::mem::TEX_NULL;
    let mut p = p;
    let mut prev_p;
    let mut q;
    let mut r = TEX_NULL;

    prev_p = TEMP_HEAD;
    mem.set_b32s1(TEMP_HEAD, p);

    while p != TEX_NULL {
        match mem.b16s1(p) {
            HLIST_NODE | VLIST_NODE_PPT | RULE_NODE => {
                q = crate::mem::new_skip_param(mem, eqtb, GLUE_PAR_SPLIT_TOP_SKIP_PPT);
                if q == crate::mem::ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow);
                }
                mem.set_b32s1(prev_p, q);
                mem.set_b32s1(q, p);
                let tp = temp_ptr();
                let height = mem.b32s1(p + 3);
                let avail_ht = mem.b32s1(tp + 1);
                if avail_ht > height {
                    mem.set_b32s1(tp + 1, avail_ht - height);
                } else {
                    mem.set_b32s1(tp + 1, 0);
                }
                p = TEX_NULL;
            }
            WHATSIT_NODE_PPT | MARK_NODE_PPT | INS_NODE_PPT => {
                prev_p = p;
                p = mem.b32s1(prev_p);
            }
            GLUE_NODE | KERN_NODE | PENALTY_NODE => {
                q = p;
                p = mem.b32s1(q);
                mem.set_b32s1(q, TEX_NULL);
                mem.set_b32s1(prev_p, p);
                if s {
                    if scanner.disc_ptr[VSPLIT_CODE_PPT] == TEX_NULL {
                        scanner.disc_ptr[VSPLIT_CODE_PPT] = q;
                    } else {
                        mem.set_b32s1(r, q);
                    }
                    r = q;
                } else {
                    flush_node_list(mem, q).map_err(NodeRecycleAbort::Confusion)?;
                }
            }
            _ => {
                return Err(NodeRecycleAbort::Confusion(c"pruning"));
            }
        }
    }

    Ok(mem.b32s1(TEMP_HEAD))
}
/// Transitional bridge-down wrapper for legacy `prune_page_top` callers.
pub unsafe fn prune_page_top_global(p: i32, s: bool) -> i32 {
    match crate::state::with_state_mut(|st| {
        let st = &mut *st;
        prune_page_top(&mut st.mem, &mut st.scanner, &st.eqtb, p, s)
    }) {
        Ok(r) => r,
        Err(e) => node_recycle_abort(e),
    }
}
/// translated from xetex-xetex0.c:9817 (do_marks)
///
/// Phase-C: threaded to `&mut MemState`. Recurses on itself and bottoms out on
/// the threaded `free_node` / `delete_token_ref`; `cur_ptr` is a raw `mod raw`
/// global (safe under the borrow). No cold abort path. Free-function calls that
/// take a `mem`-derived argument precompute it into a local to avoid overlapping
/// the `&mut mem` reborrow. Legacy callers use [`do_marks_global`].
pub fn do_marks(mem: &mut crate::state::MemState, a: i16, l: i16, q: i32) -> bool {
    use crate::globals::{cur_ptr, set_cur_ptr};
    use crate::mem::TEX_NULL;
    let mut q = q;

    if l < 4 {
        for i in 0i16..=15 {
            let cp = if i % 2 != 0 {
                mem.b32s1(q + (i / 2) as i32 + 1)
            } else {
                mem.b32s0(q + (i / 2) as i32 + 1)
            };
            set_cur_ptr(cp);

            let cur = cur_ptr();
            if cur != TEX_NULL && do_marks(mem, a, l + 1, cur) {
                if i % 2 != 0 {
                    mem.set_b32s1(q + (i / 2) as i32 + 1, TEX_NULL);
                } else {
                    mem.set_b32s0(q + (i / 2) as i32 + 1, TEX_NULL);
                }
                // mem[q].b16.s0-- (reference count of index node)
                mem.set_b16s0(q, mem.b16s0(q) - 1);
            }
        }

        if mem.b16s0(q) == 0 {
            crate::mem::free_node(mem, q, INDEX_NODE_SIZE_DM);
            q = TEX_NULL;
        }
    } else {
        match a {
            VSPLIT_INIT_DM => {
                if mem.b32s1(q + 2) != TEX_NULL {
                    let t = mem.b32s1(q + 2);
                    crate::mem::delete_token_ref(mem, t);
                    mem.set_b32s1(q + 2, TEX_NULL);
                    let t = mem.b32s0(q + 3);
                    crate::mem::delete_token_ref(mem, t);
                    mem.set_b32s0(q + 3, TEX_NULL);
                }
            }
            FIRE_UP_INIT_DM => {
                if mem.b32s0(q + 2) != TEX_NULL {
                    if mem.b32s0(q + 1) != TEX_NULL {
                        let t = mem.b32s0(q + 1);
                        crate::mem::delete_token_ref(mem, t);
                    }
                    let t = mem.b32s1(q + 1);
                    crate::mem::delete_token_ref(mem, t);
                    mem.set_b32s1(q + 1, TEX_NULL);
                    let ref_ptr = mem.b32s0(q + 2);
                    if mem.b32s1(ref_ptr) == TEX_NULL {
                        crate::mem::delete_token_ref(mem, ref_ptr);
                        mem.set_b32s0(q + 2, TEX_NULL);
                    } else {
                        mem.set_b32s0(ref_ptr, mem.b32s0(ref_ptr) + 1);
                    }
                    let v = mem.b32s0(q + 2);
                    mem.set_b32s0(q + 1, v);
                }
            }
            FIRE_UP_DONE_DM => {
                if mem.b32s0(q + 1) != TEX_NULL && mem.b32s1(q + 1) == TEX_NULL {
                    let head = mem.b32s0(q + 1);
                    mem.set_b32s1(q + 1, head);
                    let cur = mem.b32s0(head);
                    mem.set_b32s0(head, cur + 1);
                }
            }
            DESTROY_MARKS_DM => {
                for i in TOP_MARK_CODE_DM..=SPLIT_BOT_MARK_CODE_DM {
                    let cp = if i % 2 != 0 {
                        mem.b32s1(q + (i / 2) as i32 + 1)
                    } else {
                        mem.b32s0(q + (i / 2) as i32 + 1)
                    };
                    set_cur_ptr(cp);

                    if cur_ptr() != TEX_NULL {
                        crate::mem::delete_token_ref(mem, cur_ptr());
                        if i % 2 != 0 {
                            mem.set_b32s1(q + (i / 2) as i32 + 1, TEX_NULL);
                        } else {
                            mem.set_b32s0(q + (i / 2) as i32 + 1, TEX_NULL);
                        }
                    }
                }
            }
            _ => {}
        }

        if mem.b32s0(q + 2) == TEX_NULL && mem.b32s0(q + 3) == TEX_NULL {
            crate::mem::free_node(mem, q, MARK_CLASS_NODE_SIZE_DM);
            q = TEX_NULL;
        }
    }

    q == TEX_NULL
}
/// Transitional bridge-down wrapper for legacy `do_marks` callers.
pub fn do_marks_global(a: i16, l: i16, q: i32) -> bool {
    crate::state::with_state_mut(|st| do_marks(&mut st.mem, a, l, q))
}
fn hpack_in(
    mem: &mut MemState,
    eqtb: &EqtbState,
    fonts: &mut FontState,
    scanner: &mut ScannerState,
    shipout: &mut ShipoutState,
    list: i32,
    w: i32,
    m: i16,
) -> Result<PackOutcome, NodeRecycleAbort> {
    use crate::mem::{get_avail, TEX_NULL};

    const ADDITIONAL_HP: i16 = 1;
    const GARBAGE: i32 = 4_999_999 - 12; // MEM_TOP - 12

    shipout.last_badness = 0;
    let Some(r_node) = NodePtr::<HListNode>::alloc(mem) else {
        return Err(NodeRecycleAbort::Overflow);
    };
    r_node.set_subtype(mem, 0);
    r_node.set_shift_amount(mem, Scaled(0));
    r_node.set_list_ptr(mem, MemPtr::from_raw(list));
    let r = r_node.idx().0;
    let mut q = r_node.idx().offset(5).0;
    let mut h: i32 = 0;
    let mut d: i32 = 0;
    let mut x: i32 = 0;
    scanner.total_stretch = [0; 4];
    scanner.total_shrink = [0; 4];
    let mut diags = [None, None];

    if eqtb.b32s1(INT_BASE + INT_PAR_TEXXET_HP) > 0 {
        let t = get_avail(mem);
        if t == crate::mem::ALLOC_FAILED {
            return Err(NodeRecycleAbort::OverflowRunaway);
        }
        mem.set_b32s0(t, BEFORE_HP as i32);
        mem.set_b32s1(t, scanner.LR_ptr);
        scanner.LR_ptr = t;
    }

    let mut p = list;
    'outer: while p != TEX_NULL {
        while p >= mem.hi_mem_min {
            let f = mem.b16s1(p) as i32;
            let c = effective_char(fonts, true, f, mem.b16s0(p));
            let info = fonts.font_info_b16(fonts.char_base_entry(f) + c);
            x += fonts.font_info_b32s1(fonts.width_base_entry(f) + info.s3 as i32);
            let s = fonts.font_info_b32s1(fonts.height_base_entry(f) + info.s2 as i32 / 16);
            if s > h {
                h = s;
            }
            let s = fonts.font_info_b32s1(fonts.depth_base_entry(f) + info.s2 as i32 % 16);
            if s > d {
                d = s;
            }
            p = mem.b32s1(p);
        }

        if p == TEX_NULL {
            break 'outer;
        }

        let node_idx = MemIdx::new(p);
        let node_type = NodeType::from_raw(mem.b16s1(p));
        match node_type {
            Ok(NodeType::HList) => {
                // SAFETY: the `NODE_type` read above proves this list item is an hlist.
                let b = AnyBox::HList(unsafe {
                    NodePtr::<HListNode>::from_idx_unchecked(node_idx)
                });
                x += b.width(mem).0;
                let s = b.shift_amount(mem).0;
                let bh = b.height(mem).0 - s;
                if bh > h {
                    h = bh;
                }
                let bd = b.depth(mem).0 + s;
                if bd > d {
                    d = bd;
                }
            }
            Ok(NodeType::VList) => {
                // SAFETY: the `NODE_type` read above proves this list item is a vlist.
                let b = AnyBox::VList(unsafe {
                    NodePtr::<VListNode>::from_idx_unchecked(node_idx)
                });
                x += b.width(mem).0;
                let s = b.shift_amount(mem).0;
                let bh = b.height(mem).0 - s;
                if bh > h {
                    h = bh;
                }
                let bd = b.depth(mem).0 + s;
                if bd > d {
                    d = bd;
                }
            }
            Ok(NodeType::Rule) => {
                // SAFETY: the `NODE_type` read above proves this list item is a rule.
                let rule = unsafe { NodePtr::<RuleNode>::from_idx_unchecked(node_idx) };
                x += rule.width(mem).raw();
                let rh = rule.height(mem).raw();
                if rh > h {
                    h = rh;
                }
                let rd = rule.depth(mem).raw();
                if rd > d {
                    d = rd;
                }
            }
            Ok(NodeType::Unset) => {
                x += mem.b32s1(p + 1);
                if mem.b32s1(p + 3) > h {
                    h = mem.b32s1(p + 3);
                }
                if mem.b32s1(p + 2) > d {
                    d = mem.b32s1(p + 2);
                }
            }
            Ok(NodeType::Ins | NodeType::Mark | NodeType::Adjust) => {
                if scanner.adjust_tail != TEX_NULL || scanner.pre_adjust_tail != TEX_NULL {
                    while mem.b32s1(q) != p {
                        q = mem.b32s1(q);
                    }
                    if matches!(node_type, Ok(NodeType::Adjust)) {
                        if mem.b16s0(p) != 0 {
                            if scanner.pre_adjust_tail == TEX_NULL {
                                return Err(NodeRecycleAbort::Confusion(c"pre vadjust"));
                            }
                            mem.set_b32s1(scanner.pre_adjust_tail, mem.b32s1(p + 1));
                            while mem.b32s1(scanner.pre_adjust_tail) != TEX_NULL {
                                scanner.pre_adjust_tail = mem.b32s1(scanner.pre_adjust_tail);
                            }
                        } else {
                            if scanner.adjust_tail == TEX_NULL {
                                return Err(NodeRecycleAbort::Confusion(c"pre vadjust"));
                            }
                            mem.set_b32s1(scanner.adjust_tail, mem.b32s1(p + 1));
                            while mem.b32s1(scanner.adjust_tail) != TEX_NULL {
                                scanner.adjust_tail = mem.b32s1(scanner.adjust_tail);
                            }
                        }
                        p = mem.b32s1(p);
                        crate::mem::free_node(mem, mem.b32s1(q), SMALL_NODE_SIZE);
                    } else {
                        mem.set_b32s1(scanner.adjust_tail, p);
                        scanner.adjust_tail = p;
                        p = mem.b32s1(p);
                    }
                    mem.set_b32s1(q, p);
                    p = q;
                }
            }
            Ok(NodeType::Whatsit) => {
                let sub = mem.b16s0(p);
                match sub {
                    NATIVE_WORD_NODE_HP | NATIVE_WORD_NODE_AT_HP => {
                        let mut k: i32 =
                            if q != r_node.idx().offset(5).0 && mem.b16s1(q) == DISC_NODE {
                                mem.b16s0(q) as i32
                            } else {
                                0
                            };
                        while mem.b32s1(q) != p {
                            k -= 1;
                            q = mem.b32s1(q);
                            if mem.b16s1(q) == DISC_NODE {
                                k = mem.b16s0(q) as i32;
                            }
                        }
                        let mut pp = mem.b32s1(p);
                        loop {
                            if k <= 0 && pp != TEX_NULL && pp < mem.hi_mem_min {
                                let pp_type = mem.b16s1(pp);
                                let pp_sub = mem.b16s0(pp);
                                if pp_type == WHATSIT_NODE_HP
                                    && (pp_sub == NATIVE_WORD_NODE_HP
                                        || pp_sub == NATIVE_WORD_NODE_AT_HP)
                                    && mem.b16s2(pp + 4) == mem.b16s2(p + 4)
                                {
                                    pp = mem.b32s1(pp);
                                    continue;
                                } else if pp_type == DISC_NODE {
                                    let ppp = mem.b32s1(pp);
                                    if ppp != TEX_NULL
                                        && ppp < mem.hi_mem_min
                                        && mem.b16s1(ppp) == WHATSIT_NODE_HP
                                        && (mem.b16s0(ppp) == NATIVE_WORD_NODE_HP
                                            || mem.b16s0(ppp) == NATIVE_WORD_NODE_AT_HP)
                                        && mem.b16s2(ppp + 4) == mem.b16s2(p + 4)
                                    {
                                        pp = mem.b32s1(ppp);
                                        continue;
                                    }
                                }
                            }
                            break;
                        }
                        if pp != mem.b32s1(p) {
                            let mut total_chars: i32 = 0;
                            let mut scan_p = mem.b32s1(q);
                            while scan_p != pp {
                                if mem.b16s1(scan_p) == WHATSIT_NODE_HP {
                                    total_chars += mem.b16s1(scan_p + 4) as i32;
                                }
                                scan_p = mem.b32s1(scan_p);
                            }
                            let scan_p = mem.b32s1(q);
                            let merged = new_native_word_node(
                                mem,
                                eqtb,
                                mem.b16s2(scan_p + 4) as i32,
                                total_chars,
                            )?;
                            mem.set_b16s0(merged, mem.b16s0(scan_p));
                            mem.set_b32s1(q, merged);
                            let mut last_before_pp = scan_p;
                            while mem.b32s1(last_before_pp) != pp {
                                last_before_pp = mem.b32s1(last_before_pp);
                            }
                            mem.set_b32s1(merged, mem.b32s1(last_before_pp));
                            mem.set_b32s1(last_before_pp, TEX_NULL);
                            total_chars = 0;
                            let mut src_p = scan_p;
                            loop {
                                if mem.b16s1(src_p) == WHATSIT_NODE_HP {
                                    let count = mem.b16s1(src_p + 4) as i32;
                                    let src = mem.as_u16_ptr(src_p + NATIVE_NODE_SIZE_HP);
                                    let dst = mem.as_u16_ptr_mut(merged + NATIVE_NODE_SIZE_HP);
                                    for k_idx in 0..count {
                                        unsafe {
                                            *dst.offset((total_chars + k_idx) as isize) =
                                                *src.offset(k_idx as isize);
                                        }
                                    }
                                    total_chars += count;
                                }
                                if mem.b32s1(src_p) == TEX_NULL {
                                    break;
                                }
                                src_p = mem.b32s1(src_p);
                            }
                            flush_node_list(mem, scan_p).map_err(NodeRecycleAbort::Confusion)?;
                            p = mem.b32s1(q);
                            let use_glyph_metrics = eqtb.b32s1(INT_BASE + 72) > 0;
                            unsafe {
                                measure_native_node(
                                    fonts,
                                    mem.node_addr(p),
                                    use_glyph_metrics as libc::c_int,
                                );
                            }
                        }
                        if mem.b32s1(p + 3) > h {
                            h = mem.b32s1(p + 3);
                        }
                        if mem.b32s1(p + 2) > d {
                            d = mem.b32s1(p + 2);
                        }
                        x += mem.b32s1(p + 1);
                    }
                    42..=44 => {
                        if mem.b32s1(p + 3) > h {
                            h = mem.b32s1(p + 3);
                        }
                        if mem.b32s1(p + 2) > d {
                            d = mem.b32s1(p + 2);
                        }
                        x += mem.b32s1(p + 1);
                    }
                    _ => {}
                }
            }
            Ok(NodeType::Glue) => {
                // SAFETY: the `NODE_type` read above proves this list item is glue.
                let glue = unsafe { NodePtr::<GlueNode>::from_idx_unchecked(node_idx) };
                let spec = glue.glue_spec(mem).expect("glue node has glue_spec");
                x += spec.width(mem).0;
                let o = spec.stretch_order(mem) as usize;
                scanner.total_stretch[o] += spec.stretch(mem).0;
                let o = spec.shrink_order(mem) as usize;
                scanner.total_shrink[o] += spec.shrink(mem).0;
                if glue.subtype(mem) >= A_LEADERS_HP {
                    let leader = glue.leader_ptr(mem).0;
                    if mem.b32s1(leader + 3) > h {
                        h = mem.b32s1(leader + 3);
                    }
                    if mem.b32s1(leader + 2) > d {
                        d = mem.b32s1(leader + 2);
                    }
                }
            }
            Ok(NodeType::Kern) => {
                // SAFETY: the `NODE_type` read above proves this list item is a kern.
                let kern = unsafe { NodePtr::<KernNode>::from_idx_unchecked(node_idx) };
                x += kern.width(mem).0;
            }
            Ok(NodeType::Math) => {
                // SAFETY: the `NODE_type` read above proves this list item is a math node.
                let math = unsafe { NodePtr::<MathNode>::from_idx_unchecked(node_idx) };
                x += math.width(mem).0;
                if eqtb.b32s1(INT_BASE + INT_PAR_TEXXET_HP) > 0 {
                    let sub = math.subtype(mem);
                    if !sub.is_multiple_of(2) {
                        let expected = (L_CODE_HP as u32) * (sub as u32 / L_CODE_HP as u32) + 3;
                        if mem.b32s0(scanner.LR_ptr) == expected as i32 {
                            let t = scanner.LR_ptr;
                            scanner.LR_ptr = mem.b32s1(t);
                            mem.set_b32s1(t, mem.avail);
                            mem.avail = t;
                        } else {
                            scanner.LR_problems += 1;
                            mem.set_b16s1(p, KERN_NODE);
                            mem.set_b16s0(p, EXPLICIT_HP);
                        }
                    } else {
                        let t = get_avail(mem);
                        if t == crate::mem::ALLOC_FAILED {
                            return Err(NodeRecycleAbort::OverflowRunaway);
                        }
                        mem.set_b32s0(t, (L_CODE_HP as i32) * (sub as i32 / L_CODE_HP as i32) + 3);
                        mem.set_b32s1(t, scanner.LR_ptr);
                        scanner.LR_ptr = t;
                    }
                }
            }
            Ok(NodeType::Ligature) => {
                mem.set_word(GARBAGE, mem.word(p + 1));
                mem.set_b32s1(GARBAGE, mem.b32s1(p));
                p = GARBAGE;
                fonts.set_xtx_ligature_present(true);
                continue 'outer;
            }
            Ok(_) | Err(_) => {}
        }
        p = mem.b32s1(p);
    }

    if scanner.adjust_tail != TEX_NULL {
        mem.set_b32s1(scanner.adjust_tail, TEX_NULL);
    }
    if scanner.pre_adjust_tail != TEX_NULL {
        mem.set_b32s1(scanner.pre_adjust_tail, TEX_NULL);
    }
    r_node.set_height(mem, Scaled(h));
    r_node.set_depth(mem, Scaled(d));

    let w = if m == ADDITIONAL_HP { x + w } else { w };
    r_node.set_width(mem, Scaled(w));
    x = w - x;

    if x == 0 {
        r_node.set_glue_sign(mem, 0);
        r_node.set_glue_order(mem, 0);
        r_node.set_glue_set(mem, 0.0);
    } else if x > 0 {
        let o = if scanner.total_stretch[FILLL_HP] != 0 {
            FILLL_HP
        } else if scanner.total_stretch[FILL_HP] != 0 {
            FILL_HP
        } else if scanner.total_stretch[FIL_HP] != 0 {
            FIL_HP
        } else {
            NORMAL_HP
        };
        r_node.set_glue_order(mem, o as u16);
        r_node.set_glue_sign(mem, STRETCHING_HP);
        if scanner.total_stretch[o] != 0 {
            r_node.set_glue_set(mem, x as f64 / scanner.total_stretch[o] as f64);
        } else {
            r_node.set_glue_sign(mem, 0);
            r_node.set_glue_set(mem, 0.0);
        }
        if o == NORMAL_HP && r_node.list_ptr(mem).0 != TEX_NULL {
            shipout.last_badness = crate::helpers::badness(x, scanner.total_stretch[NORMAL_HP]);
            if shipout.last_badness > eqtb.b32s1(INT_BASE + INT_PAR_HBADNESS_HP) {
                push_pack_diag(
                    &mut diags,
                    PackDiag::HUnderfull {
                        badness: shipout.last_badness,
                        loose: shipout.last_badness <= 100,
                    },
                );
            }
        }
    } else {
        let o = if scanner.total_shrink[FILLL_HP] != 0 {
            FILLL_HP
        } else if scanner.total_shrink[FILL_HP] != 0 {
            FILL_HP
        } else if scanner.total_shrink[FIL_HP] != 0 {
            FIL_HP
        } else {
            NORMAL_HP
        };
        r_node.set_glue_order(mem, o as u16);
        r_node.set_glue_sign(mem, SHRINKING_HP);
        if scanner.total_shrink[o] != 0 {
            r_node.set_glue_set(mem, (-x) as f64 / scanner.total_shrink[o] as f64);
        } else {
            r_node.set_glue_sign(mem, 0);
            r_node.set_glue_set(mem, 0.0);
        }
        if scanner.total_shrink[o] < (-x) && o == NORMAL_HP && r_node.list_ptr(mem).0 != TEX_NULL {
            shipout.last_badness = 1_000_000;
            r_node.set_glue_set(mem, 1.0);
            if (-x) - scanner.total_shrink[NORMAL_HP] > eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_HFUZZ_HP)
                || eqtb.b32s1(INT_BASE + INT_PAR_HBADNESS_HP) < 100
            {
                if eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE_HP) > 0
                    && (-x) - scanner.total_shrink[NORMAL_HP]
                        > eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_HFUZZ_HP)
                {
                    while mem.b32s1(q) != TEX_NULL {
                        q = mem.b32s1(q);
                    }
                    let rule = crate::mem::new_rule(mem);
                    if rule == crate::mem::ALLOC_FAILED {
                        return Err(NodeRecycleAbort::Overflow);
                    }
                    let rule_node = mem
                        .classify_rule(MemIdx::new(rule))
                        .expect("valid rule node type")
                        .expect("rule node");
                    mem.set_b32s1(q, rule);
                    rule_node.set_width(
                        mem,
                        RuleDimen::Set(Scaled(eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_OVERFULL_RULE_HP))),
                    );
                }
                push_pack_diag(
                    &mut diags,
                    PackDiag::HOverfull {
                        excess: (-x) - scanner.total_shrink[NORMAL_HP],
                    },
                );
            }
        } else if o == NORMAL_HP && r_node.list_ptr(mem).0 != TEX_NULL {
            shipout.last_badness = crate::helpers::badness(-x, scanner.total_shrink[NORMAL_HP]);
            if shipout.last_badness > eqtb.b32s1(INT_BASE + INT_PAR_HBADNESS_HP) {
                push_pack_diag(
                    &mut diags,
                    PackDiag::HTight {
                        badness: shipout.last_badness,
                    },
                );
            }
        }
    }

    if eqtb.b32s1(INT_BASE + INT_PAR_TEXXET_HP) > 0 {
        if mem.b32s0(scanner.LR_ptr) != BEFORE_HP as i32 {
            while mem.b32s1(q) != TEX_NULL {
                q = mem.b32s1(q);
            }
            loop {
                let t = q;
                let new_q = crate::mem::new_math(mem, 0, mem.b32s0(scanner.LR_ptr) as i16);
                if new_q == crate::mem::ALLOC_FAILED {
                    return Err(NodeRecycleAbort::Overflow);
                }
                mem.set_b32s1(t, new_q);
                q = new_q;
                scanner.LR_problems += 10000;
                let t2 = scanner.LR_ptr;
                scanner.LR_ptr = mem.b32s1(t2);
                mem.set_b32s1(t2, mem.avail);
                mem.avail = t2;
                if mem.b32s0(scanner.LR_ptr) == BEFORE_HP as i32 {
                    break;
                }
            }
        }
        if scanner.LR_problems > 0 {
            push_pack_diag(
                &mut diags,
                PackDiag::LrProblem {
                    missing: scanner.LR_problems / 10000,
                    extra: scanner.LR_problems % 10000,
                },
            );
            scanner.LR_problems = 0;
        }
        let t = scanner.LR_ptr;
        scanner.LR_ptr = mem.b32s1(t);
        mem.set_b32s1(t, mem.avail);
        mem.avail = t;
        if scanner.LR_ptr != TEX_NULL {
            return Err(NodeRecycleAbort::Confusion(c"LR1"));
        }
    }

    Ok(PackOutcome { r, diags })
}

fn print_hpack_diag(box_ptr: i32, diag: PackDiag) {
    unsafe {
        match diag {
            PackDiag::HUnderfull { badness, loose } => {
                crate::output::print_ln();
                crate::output::diagnostic_begin_capture_warning_here();
                if loose {
                    crate::output::print_nl_cstr(c"Loose".as_ptr());
                } else {
                    crate::output::print_nl_cstr(c"Underfull".as_ptr());
                }
                crate::output::print_cstr(c" \\hbox (badness ".as_ptr());
                crate::output::print_int(badness);
            }
            PackDiag::HOverfull { excess } => {
                crate::output::print_ln();
                crate::output::diagnostic_begin_capture_warning_here();
                crate::output::print_nl_cstr(c"Overfull \\hbox (".as_ptr());
                crate::output::print_scaled(excess);
                crate::output::print_cstr(c"pt too wide".as_ptr());
            }
            PackDiag::HTight { badness } => {
                crate::output::print_ln();
                crate::output::diagnostic_begin_capture_warning_here();
                crate::output::print_nl_cstr(c"Tight \\hbox (badness ".as_ptr());
                crate::output::print_int(badness);
            }
            PackDiag::LrProblem { missing, extra } => {
                crate::output::print_ln();
                crate::output::diagnostic_begin_capture_warning_here();
                crate::output::print_nl_cstr(c"\\endL or \\endR problem (".as_ptr());
                crate::output::print_int(missing);
                crate::output::print_cstr(c" missing, ".as_ptr());
                crate::output::print_int(extra);
                crate::output::print_cstr(c" extra".as_ptr());
            }
            PackDiag::VUnderfull { .. } | PackDiag::VOverfull { .. } | PackDiag::VTight { .. } => {
                unreachable!("vpack diagnostic passed to hpack printer")
            }
        }
        print_hpack_common_ending(box_ptr);
    }
}

fn print_hpack_common_ending(box_ptr: i32) {
    unsafe {
        if crate::globals::output_active() {
            crate::output::print_cstr(c") has occurred while \\output is active".as_ptr());
        } else {
            let pack_begin_line = crate::globals::pack_begin_line();
            if pack_begin_line != 0 {
                if pack_begin_line > 0 {
                    crate::output::print_cstr(c") in paragraph at lines ".as_ptr());
                } else {
                    crate::output::print_cstr(c") in alignment at lines ".as_ptr());
                }
                crate::output::print_int(pack_begin_line.abs());
                crate::output::print_cstr(c"--".as_ptr());
            } else {
                crate::output::print_cstr(c") detected at line ".as_ptr());
            }
            crate::output::print_int(crate::globals::line());
        }
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::print_ln();
        crate::globals::set_font_in_short_display(0);
        short_display(mem_b32s1(box_ptr + 5));
        crate::output::print_ln();
        crate::output::begin_diagnostic();
        show_box(box_ptr);
        crate::output::end_diagnostic(true);
    }
}

/// translated from xetex-xetex0.c:6972 (hpack)
pub unsafe fn hpack(p: i32, w: i32, m: i16) -> i32 {
    let outcome = match crate::state::with_state_mut(|st| {
        hpack_in(
            &mut st.mem,
            &st.eqtb,
            &mut st.fonts,
            &mut st.scanner,
            &mut st.shipout,
            p,
            w,
            m,
        )
    }) {
        Ok(outcome) => outcome,
        Err(e) => node_recycle_abort(e),
    };
    for diag in outcome.diags.into_iter().flatten() {
        print_hpack_diag(outcome.r, diag);
    }
    outcome.r
}
fn vpackage_in(
    mem: &mut MemState,
    eqtb: &EqtbState,
    scanner: &mut ScannerState,
    shipout: &mut ShipoutState,
    list: i32,
    h: i32,
    m: i16,
    l: i32,
) -> Result<PackOutcome, NodeRecycleAbort> {
    use crate::mem::TEX_NULL;

    const ADDITIONAL_VP: i16 = 1;

    shipout.last_badness = 0;
    let Some(r_node) = NodePtr::<VListNode>::alloc(mem) else {
        return Err(NodeRecycleAbort::Overflow);
    };
    if eqtb.b32s1(INT_BASE + INT_PAR_XETEX_UPWARDS_VP) > 0 {
        r_node.set_subtype(mem, 1);
    } else {
        r_node.set_subtype(mem, 0);
    }
    r_node.set_shift_amount(mem, Scaled(0));
    r_node.set_list_ptr(mem, MemPtr::from_raw(list));

    let mut w: i32 = 0;
    let mut d: i32 = 0;
    let mut x: i32 = 0;
    scanner.total_stretch = [0; 4];
    scanner.total_shrink = [0; 4];
    let mut diags = [None, None];

    let mut p = list;
    while p != TEX_NULL {
        if p >= mem.hi_mem_min {
            return Err(NodeRecycleAbort::Confusion(c"vpack"));
        } else {
            let node_idx = MemIdx::new(p);
            match NodeType::from_raw(mem.b16s1(p)) {
                Ok(NodeType::HList) => {
                    // SAFETY: the `NODE_type` read above proves this list item is an hlist.
                    let b = AnyBox::HList(unsafe {
                        NodePtr::<HListNode>::from_idx_unchecked(node_idx)
                    });
                    x += d + b.height(mem).0;
                    d = b.depth(mem).0;
                    let width = b.width(mem).0 + b.shift_amount(mem).0;
                    if width > w {
                        w = width;
                    }
                }
                Ok(NodeType::VList) => {
                    // SAFETY: the `NODE_type` read above proves this list item is a vlist.
                    let b = AnyBox::VList(unsafe {
                        NodePtr::<VListNode>::from_idx_unchecked(node_idx)
                    });
                    x += d + b.height(mem).0;
                    d = b.depth(mem).0;
                    let width = b.width(mem).0 + b.shift_amount(mem).0;
                    if width > w {
                        w = width;
                    }
                }
                Ok(NodeType::Rule) => {
                    // SAFETY: the `NODE_type` read above proves this list item is a rule.
                    let rule = unsafe { NodePtr::<RuleNode>::from_idx_unchecked(node_idx) };
                    x += d + rule.height(mem).raw();
                    d = rule.depth(mem).raw();
                    let width = rule.width(mem).raw();
                    if width > w {
                        w = width;
                    }
                }
                Ok(NodeType::Unset) => {
                    x += d + mem.b32s1(p + 3);
                    d = mem.b32s1(p + 2);
                    if mem.b32s1(p + 1) > w {
                        w = mem.b32s1(p + 1);
                    }
                }
                Ok(NodeType::Whatsit) => {
                    let sub = mem.b16s0(p);
                    if sub == PIC_NODE_VP || sub == PDF_NODE_VP {
                        x += d + mem.b32s1(p + 3);
                        d = mem.b32s1(p + 2);
                        if mem.b32s1(p + 1) > w {
                            w = mem.b32s1(p + 1);
                        }
                    }
                }
                Ok(NodeType::Glue) => {
                    x += d;
                    d = 0;
                    // SAFETY: the `NODE_type` read above proves this list item is glue.
                    let glue = unsafe { NodePtr::<GlueNode>::from_idx_unchecked(node_idx) };
                    let spec = glue.glue_spec(mem).expect("glue node has glue_spec");
                    x += spec.width(mem).0;
                    let o = spec.stretch_order(mem) as usize;
                    scanner.total_stretch[o] += spec.stretch(mem).0;
                    let o = spec.shrink_order(mem) as usize;
                    scanner.total_shrink[o] += spec.shrink(mem).0;
                    if glue.subtype(mem) >= A_LEADERS_VP {
                        let leader = glue.leader_ptr(mem).0;
                        if mem.b32s1(leader + 1) > w {
                            w = mem.b32s1(leader + 1);
                        }
                    }
                }
                Ok(NodeType::Kern) => {
                    // SAFETY: the `NODE_type` read above proves this list item is a kern.
                    let kern = unsafe { NodePtr::<KernNode>::from_idx_unchecked(node_idx) };
                    x += d + kern.width(mem).0;
                    d = 0;
                }
                Ok(_) | Err(_) => {}
            }
        }
        p = mem.b32s1(p);
    }

    r_node.set_width(mem, Scaled(w));
    if d > l {
        x += d - l;
        r_node.set_depth(mem, Scaled(l));
    } else {
        r_node.set_depth(mem, Scaled(d));
    }

    let h = if m == ADDITIONAL_VP { x + h } else { h };
    r_node.set_height(mem, Scaled(h));
    x = h - x;

    if x == 0 {
        r_node.set_glue_sign(mem, 0);
        r_node.set_glue_order(mem, 0);
        r_node.set_glue_set(mem, 0.0);
    } else if x > 0 {
        let o = if scanner.total_stretch[FILLL_VP] != 0 {
            FILLL_VP
        } else if scanner.total_stretch[FILL_VP] != 0 {
            FILL_VP
        } else if scanner.total_stretch[FIL_VP] != 0 {
            FIL_VP
        } else {
            NORMAL_VP
        };
        r_node.set_glue_order(mem, o as u16);
        r_node.set_glue_sign(mem, STRETCHING_VP);
        if scanner.total_stretch[o] != 0 {
            r_node.set_glue_set(mem, x as f64 / scanner.total_stretch[o] as f64);
        } else {
            r_node.set_glue_sign(mem, 0);
            r_node.set_glue_set(mem, 0.0);
        }
        if o == NORMAL_VP && r_node.list_ptr(mem).0 != TEX_NULL {
            shipout.last_badness = crate::helpers::badness(x, scanner.total_stretch[NORMAL_VP]);
            if shipout.last_badness > eqtb.b32s1(INT_BASE + INT_PAR_VBADNESS_VP) {
                push_pack_diag(
                    &mut diags,
                    PackDiag::VUnderfull {
                        badness: shipout.last_badness,
                        loose: shipout.last_badness <= 100,
                    },
                );
            }
        }
    } else {
        let o = if scanner.total_shrink[FILLL_VP] != 0 {
            FILLL_VP
        } else if scanner.total_shrink[FILL_VP] != 0 {
            FILL_VP
        } else if scanner.total_shrink[FIL_VP] != 0 {
            FIL_VP
        } else {
            NORMAL_VP
        };
        r_node.set_glue_order(mem, o as u16);
        r_node.set_glue_sign(mem, SHRINKING_VP);
        if scanner.total_shrink[o] != 0 {
            r_node.set_glue_set(mem, (-(x as i64)) as f64 / scanner.total_shrink[o] as f64);
        } else {
            r_node.set_glue_sign(mem, 0);
            r_node.set_glue_set(mem, 0.0);
        }
        if scanner.total_shrink[o] < (-x) && o == NORMAL_VP && r_node.list_ptr(mem).0 != TEX_NULL {
            shipout.last_badness = 1_000_000;
            r_node.set_glue_set(mem, 1.0);
            if (-x) - scanner.total_shrink[NORMAL_VP] > eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_VFUZZ_VP)
                || eqtb.b32s1(INT_BASE + INT_PAR_VBADNESS_VP) < 100
            {
                push_pack_diag(
                    &mut diags,
                    PackDiag::VOverfull {
                        excess: (-x) - scanner.total_shrink[NORMAL_VP],
                    },
                );
            }
        } else if o == NORMAL_VP && r_node.list_ptr(mem).0 != TEX_NULL {
            shipout.last_badness = crate::helpers::badness(-x, scanner.total_shrink[NORMAL_VP]);
            if shipout.last_badness > eqtb.b32s1(INT_BASE + INT_PAR_VBADNESS_VP) {
                push_pack_diag(
                    &mut diags,
                    PackDiag::VTight {
                        badness: shipout.last_badness,
                    },
                );
            }
        }
    }

    Ok(PackOutcome {
        r: r_node.idx().0,
        diags,
    })
}

fn print_vpack_diag(box_ptr: i32, diag: PackDiag) {
    unsafe {
        match diag {
            PackDiag::VUnderfull { badness, loose } => {
                crate::output::print_ln();
                crate::output::diagnostic_begin_capture_warning_here();
                if loose {
                    crate::output::print_nl_cstr(c"Loose".as_ptr());
                } else {
                    crate::output::print_nl_cstr(c"Underfull".as_ptr());
                }
                crate::output::print_cstr(c" \\vbox (badness ".as_ptr());
                crate::output::print_int(badness);
            }
            PackDiag::VOverfull { excess } => {
                crate::output::print_ln();
                crate::output::diagnostic_begin_capture_warning_here();
                crate::output::print_nl_cstr(c"Overfull \\vbox (".as_ptr());
                crate::output::print_scaled(excess);
                crate::output::print_cstr(c"pt too high".as_ptr());
            }
            PackDiag::VTight { badness } => {
                crate::output::print_ln();
                crate::output::diagnostic_begin_capture_warning_here();
                crate::output::print_nl_cstr(c"Tight \\vbox (badness ".as_ptr());
                crate::output::print_int(badness);
            }
            PackDiag::HUnderfull { .. }
            | PackDiag::HOverfull { .. }
            | PackDiag::HTight { .. }
            | PackDiag::LrProblem { .. } => unreachable!("hpack diagnostic passed to vpack printer"),
        }
        print_vpack_common_ending(box_ptr);
    }
}

fn print_vpack_common_ending(box_ptr: i32) {
    unsafe {
        if crate::globals::output_active() {
            crate::output::print_cstr(c") has occurred while \\output is active".as_ptr());
        } else {
            let pack_begin_line = crate::globals::pack_begin_line();
            if pack_begin_line != 0 {
                crate::output::print_cstr(c") in alignment at lines ".as_ptr());
                crate::output::print_int(pack_begin_line.abs());
                crate::output::print_cstr(c"--".as_ptr());
            } else {
                crate::output::print_cstr(c") detected at line ".as_ptr());
            }
            crate::output::print_int(crate::globals::line());
            crate::output::print_ln();
        }
        capture_to_diagnostic(std::ptr::null_mut());
        crate::output::begin_diagnostic();
        show_box(box_ptr);
        crate::output::end_diagnostic(true);
    }
}

/// translated from xetex-xetex0.c:7424 (vpackage)
pub unsafe fn vpackage(p: i32, h: i32, m: i16, l: i32) -> i32 {
    let outcome = match crate::state::with_state_mut(|st| {
        vpackage_in(
            &mut st.mem,
            &st.eqtb,
            &mut st.scanner,
            &mut st.shipout,
            p,
            h,
            m,
            l,
        )
    }) {
        Ok(outcome) => outcome,
        Err(e) => node_recycle_abort(e),
    };
    for diag in outcome.diags.into_iter().flatten() {
        print_vpack_diag(outcome.r, diag);
    }
    outcome.r
}
