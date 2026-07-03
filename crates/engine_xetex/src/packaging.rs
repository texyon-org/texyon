//! Box/node packing and list utilities — `hpack`/`vpackage`/`vsplit`/
//! `vert_break`/`append_to_vlist`/`prune_page_top`/`do_marks` and the
//! node-recycle utilities `copy_node_list`/`flush_node_list`/`just_copy`/
//! `just_reverse`.

use crate::consts::{
    BOX_BASE, BOX_NODE_SIZE, DIMEN_BASE, GLUE_BASE, GLUE_SPEC_SIZE, INT_BASE, KERN_NODE, MATH_NODE,
    MAX_HALFWORD, MEDIUM_NODE_SIZE, MIN_HALFWORD, RULE_NODE_SIZE, SMALL_NODE_SIZE,
};
use crate::errors::ErrorOutcome;
use crate::ext::measure_native_node;
use crate::fonts::{effective_char, new_native_word_node};
use crate::node::{
    AnyBox, AnyNode, DiscNode, GlueNode, GlueSpecNode, HListNode, KernNode, MarkNode, NodePtr,
    RuleDimen, SaNode, VListNode, WhatsitNode, WhatsitSubtype,
};
use crate::units::NodeType;

// Shared node-recycle infrastructure stays in scan.rs (also used by
// prune_page_top and the relocated font-loading group); import it here.
use crate::scan::{cnl_get_avail, cnl_get_node, NodeRecycleAbort};
use crate::state::{EqtbState, FontState, MemState, ScannerState, ShipoutState};
use crate::units::{MemIdx, MemPtr, Scaled};

// Module-private consts shared with scan.rs (TEMP_HEAD/IGNORE_DEPTH still used
// there too, so duplicated rather than relocated — same value).
const TEMP_HEAD: i32 = 4_999_999 - 3;
const IGNORE_DEPTH: i32 = -65_536_000; // xetex_format.h:388

// ---------------------------------------------------------------------------
// flush_node_list — NODE_type dispatch now goes through `MemState::classify`
// (`AnyNode`); only the two noad variants whose *size* differs from the base
// `NOAD_SIZE` need a raw type code to pick the free size.
// ---------------------------------------------------------------------------
const RADICAL_NOAD_FNL: u16 = 24;
const ACCENT_NOAD_FNL: u16 = 28;
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
// copy_node_list — NODE_type dispatch now goes through `MemState::classify`
// (`AnyNode`); only the whatsit subtypes below stay as raw codes.
// ---------------------------------------------------------------------------
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

fn classify_native_word_hp(mem: &MemState, p: i32) -> Option<NodePtr<WhatsitNode>> {
    if p == crate::mem::TEX_NULL || p >= mem.hi_mem_min {
        return None;
    }
    let w = mem.classify_whatsit(MemIdx::new(p)).ok().flatten()?;
    match w.subtype(mem) {
        NATIVE_WORD_NODE_HP | NATIVE_WORD_NODE_AT_HP => Some(w),
        _ => None,
    }
}

fn classify_native_word_with_font_hp(
    mem: &MemState,
    p: i32,
    font: i32,
) -> Option<NodePtr<WhatsitNode>> {
    let w = classify_native_word_hp(mem, p)?;
    (w.native_font(mem).0 == font).then_some(w)
}

fn classify_disc_hp(mem: &MemState, p: i32) -> Option<NodePtr<DiscNode>> {
    if p >= mem.hi_mem_min {
        return None;
    }
    mem.classify_disc(MemIdx::new(p)).ok().flatten()
}

#[derive(Copy, Clone)]
pub(crate) enum PackDiag {
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
    /// The freshly-packed box, classified once at its allocation site (an
    /// `HListNode` from [`hpack_in`], a `VListNode` from [`vpackage_in`]).
    r_box: AnyBox,
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

fn copy_box_shell(
    mem: &mut crate::state::MemState,
    p: i32,
    list_ptr: MemPtr,
) -> Result<i32, NodeRecycleAbort> {
    let r = cnl_get_node!(mem, BOX_NODE_SIZE);
    let mut w = BOX_NODE_SIZE;
    while w > 0 {
        w -= 1;
        mem.set_word(r + w, mem.word(p + w));
    }
    let r_box = mem
        .classify_box(MemIdx::new(r))
        .map_err(|_| NodeRecycleAbort::Confusion(c"copying"))?
        .expect("copied box header classifies as a box");
    r_box.set_list_ptr(mem, list_ptr);
    Ok(r)
}

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

    let mut p = p;
    let mut h = h;

    while p != TEX_NULL {
        let mut words: i32 = 1;

        // Determine r and words for this node. None = skip linking (goto not_found).
        let r_opt: Option<i32> = if p >= mem.hi_mem_min {
            // char node
            Some(cnl_get_avail!(mem))
        } else {
            match mem.classify(MemIdx::new(p)).ok() {
                Some(AnyNode::HList(_)) | Some(AnyNode::VList(_)) => {
                    let r = copy_box_shell(mem, p, MemPtr::NULL)?;
                    words = 0; // already copied
                    Some(r)
                }
                Some(AnyNode::Rule(_)) => {
                    words = RULE_NODE_SIZE;
                    Some(cnl_get_node!(mem, RULE_NODE_SIZE))
                }
                Some(AnyNode::Ligature(_)) => {
                    let r = cnl_get_avail!(mem);
                    mem.set_word(r, mem.word(p + 1)); // mem[r] = mem[p + 1]
                    words = 0; // skip the copy loop (goto found)
                    Some(r)
                }
                Some(AnyNode::Kern(_)) | Some(AnyNode::Math(_)) => {
                    words = MEDIUM_NODE_SIZE;
                    Some(cnl_get_node!(mem, MEDIUM_NODE_SIZE))
                }
                Some(AnyNode::Glue(glue)) => {
                    let gp = glue.glue_ptr(mem); // GLUE_NODE_glue_ptr(p)
                    let r = cnl_get_node!(mem, MEDIUM_NODE_SIZE);
                    // GLUE_SPEC_ref_count(GLUE_NODE_glue_ptr(p))++
                    let spec = NodePtr::<GlueSpecNode>::from_glue_ptr(gp).unwrap();
                    spec.set_ref_count(mem, spec.ref_count(mem) + 1);
                    mem.set_word(r, mem.word(p));
                    mem.set_word(
                        r + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE,
                        mem.word(p + MEDIUM_NODE_SIZE - SYNCTEX_FIELD_SIZE),
                    );
                    let r_glue = match mem
                        .classify(MemIdx::new(r))
                        .map_err(|_| NodeRecycleAbort::Confusion(c"copying"))?
                    {
                        AnyNode::Glue(r_glue) => r_glue,
                        _ => unreachable!("copied glue header classifies as glue"),
                    };
                    r_glue.set_glue_ptr(mem, gp);
                    r_glue.set_leader_ptr(mem, MemPtr::NULL);
                    words = 0; // already copied
                    Some(r)
                }
                Some(AnyNode::Whatsit(ws)) => {
                    match ws.whatsit_subtype(mem) {
                        Some(WhatsitSubtype::Open) => {
                            words = OPEN_NODE_SIZE;
                            Some(cnl_get_node!(mem, OPEN_NODE_SIZE))
                        }
                        Some(WhatsitSubtype::Write) | Some(WhatsitSubtype::Special) => {
                            let r = cnl_get_node!(mem, WRITE_NODE_SIZE);
                            // TOKEN_LIST_ref_count(WRITE_NODE_tokens(p))++
                            let toks = ws.write_tokens(mem).0;
                            mem.set_b32s0(toks, mem.b32s0(toks) + 1);
                            words = WRITE_NODE_SIZE;
                            Some(r)
                        }
                        Some(WhatsitSubtype::Close) | Some(WhatsitSubtype::Language) => {
                            words = SMALL_NODE_SIZE;
                            Some(cnl_get_node!(mem, SMALL_NODE_SIZE))
                        }
                        Some(WhatsitSubtype::NativeWord) | Some(WhatsitSubtype::NativeWordAt) => {
                            let sz = ws.native_size(mem); // NATIVE_NODE_size(p)
                            let r = cnl_get_node!(mem, sz);
                            let mut nw = sz;
                            while nw > 0 {
                                nw -= 1;
                                mem.set_word(r + nw, mem.word(p + nw));
                            }
                            // The copy carried p's NODE_type, so r is a whatsit too.
                            let rw = match mem
                                .classify(MemIdx::new(r))
                                .map_err(|_| NodeRecycleAbort::Confusion(c"ext2"))?
                            {
                                AnyNode::Whatsit(rw) => rw,
                                _ => unreachable!("copied whatsit header classifies as whatsit"),
                            };
                            rw.set_native_glyph_info_ptr(mem, std::ptr::null_mut());
                            rw.set_native_glyph_count(mem, 0);
                            crate::mem::copy_native_glyph_info(mem, p, r);
                            words = 0; // already copied, skip outer loop
                            Some(r)
                        }
                        Some(WhatsitSubtype::Glyph) => {
                            words = GLYPH_NODE_SIZE;
                            Some(cnl_get_node!(mem, GLYPH_NODE_SIZE))
                        }
                        Some(WhatsitSubtype::Pic) | Some(WhatsitSubtype::Pdf) => {
                            let path_len = ws.pic_path_len(mem) as i32; // PIC_NODE_path_len
                            words =
                                PIC_NODE_SIZE + (path_len + MEM_WORD_BYTES - 1) / MEM_WORD_BYTES;
                            Some(cnl_get_node!(mem, words))
                        }
                        Some(WhatsitSubtype::PdfSavePos) => Some(cnl_get_node!(mem, SMALL_NODE_SIZE)),
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

// translated from xetex-xetex0.c:8155 (just_reverse)
pub fn just_reverse(
    mem: &mut crate::state::MemState,
    scanner: &mut crate::state::ScannerState,
    p: i32,
) -> Result<(), NodeRecycleAbort> {
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
                            mem.temp_ptr = scanner.LR_ptr;
                            scanner.LR_ptr = mem.b32s1(mem.temp_ptr);
                            mem.set_b32s1(mem.temp_ptr, mem.avail);
                            mem.avail = mem.temp_ptr;

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
                        mem.temp_ptr = tp;
                        mem.set_b32s0(mem.temp_ptr, L_CODE * (subtype / L_CODE) + 3);
                        mem.set_b32s1(mem.temp_ptr, scanner.LR_ptr);
                        scanner.LR_ptr = mem.temp_ptr;

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

/// translated from xetex-xetex0.c:765 (flush_node_list)
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
                match mem.classify(MemIdx::new(p)).map_err(|_| c"flushing")? {
                    AnyNode::HList(b) => {
                        let c = b.list_ptr(mem).0;
                        flush_node_list(mem, c)?;
                        crate::mem::free_node(mem, p, BOX_NODE_SIZE);
                        break 'done;
                    }
                    AnyNode::VList(b) => {
                        let c = b.list_ptr(mem).0;
                        flush_node_list(mem, c)?;
                        crate::mem::free_node(mem, p, BOX_NODE_SIZE);
                        break 'done;
                    }
                    AnyNode::Unset(u) => {
                        let c = u.list_ptr(mem).0;
                        flush_node_list(mem, c)?;
                        crate::mem::free_node(mem, p, BOX_NODE_SIZE);
                        break 'done;
                    }
                    AnyNode::Rule(_) => {
                        crate::mem::free_node(mem, p, RULE_NODE_SIZE);
                        break 'done;
                    }
                    AnyNode::Ins(ins) => {
                        let c = ins.ins_ptr(mem).0;
                        flush_node_list(mem, c)?;
                        let g = ins.split_top_ptr(mem).0;
                        crate::mem::delete_glue_ref(mem, g);
                        crate::mem::free_node(mem, p, INS_NODE_SIZE_FNL);
                        break 'done;
                    }
                    AnyNode::Whatsit(w) => {
                        match w.subtype(mem) {
                            OPEN_NODE_FNL => {
                                crate::mem::free_node(mem, p, OPEN_NODE_SIZE_FNL);
                            }
                            WRITE_NODE_FNL | SPECIAL_NODE_FNL => {
                                let t = w.write_tokens(mem).0;
                                crate::mem::delete_token_ref(mem, t);
                                crate::mem::free_node(mem, p, WRITE_NODE_SIZE_FNL);
                                break 'done;
                            }
                            CLOSE_NODE_FNL | LANGUAGE_NODE_FNL => {
                                crate::mem::free_node(mem, p, SMALL_NODE_SIZE);
                            }
                            NATIVE_WORD_NODE_FNL | NATIVE_WORD_NODE_AT_FNL => {
                                let glyph_ptr = w.native_glyph_info_ptr(mem);
                                if !glyph_ptr.is_null() {
                                    // SAFETY: glyph_ptr was xmalloc'd by the native-word shaper.
                                    unsafe { libc::free(glyph_ptr) };
                                    w.set_native_glyph_info_ptr(mem, std::ptr::null_mut());
                                    w.set_native_glyph_count(mem, 0);
                                }
                                let sz = w.native_size(mem);
                                crate::mem::free_node(mem, p, sz);
                            }
                            GLYPH_NODE_FNL => {
                                crate::mem::free_node(mem, p, GLYPH_NODE_SIZE_FNL);
                            }
                            PIC_NODE_FNL | PDF_NODE_FNL => {
                                let path_len = w.pic_path_len(mem) as i32;
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
                    AnyNode::Glue(glue) => {
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
                    AnyNode::Kern(_) | AnyNode::Math(_) | AnyNode::Penalty(_) => {
                        crate::mem::free_node(mem, p, MEDIUM_NODE_SIZE);
                        break 'done;
                    }
                    AnyNode::MarginKern(_) => {
                        crate::mem::free_node(mem, p, MARGIN_KERN_NODE_SIZE_FNL);
                        break 'done;
                    }
                    AnyNode::Ligature(lig) => {
                        let c = lig.lig_ptr(mem).0;
                        flush_node_list(mem, c)?;
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    AnyNode::Mark(mark) => {
                        let t = mark.mark_ptr(mem).0;
                        crate::mem::delete_token_ref(mem, t);
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    AnyNode::Disc(disc) => {
                        let c1 = disc.pre_break(mem).0;
                        flush_node_list(mem, c1)?;
                        let c2 = disc.post_break(mem).0;
                        flush_node_list(mem, c2)?;
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    AnyNode::Adjust(adjust) => {
                        let c = adjust.adjust_ptr(mem).0;
                        flush_node_list(mem, c)?;
                        // falls through to free_node(p, SMALL_NODE_SIZE)
                    }
                    AnyNode::Style(_) => {
                        crate::mem::free_node(mem, p, STYLE_NODE_SIZE_FNL);
                        break 'done;
                    }
                    AnyNode::Choice(choice) => {
                        let c1 = choice.display_mlist(mem).0;
                        flush_node_list(mem, c1)?;
                        let c2 = choice.text_mlist(mem).0;
                        flush_node_list(mem, c2)?;
                        let c3 = choice.script_mlist(mem).0;
                        flush_node_list(mem, c3)?;
                        let c4 = choice.script_script_mlist(mem).0;
                        flush_node_list(mem, c4)?;
                        crate::mem::free_node(mem, p, STYLE_NODE_SIZE_FNL);
                        break 'done;
                    }
                    AnyNode::Noad(p_noad) => {
                        match p_noad.noad_type(mem) {
                            NodeType::LeftNoad | NodeType::RightNoad => {
                                crate::mem::free_node(mem, p, NOAD_SIZE_FNL);
                            }
                            NodeType::FractionNoad => {
                                let c1 = p_noad.supscr_field(mem);
                                flush_node_list(mem, c1)?;
                                let c2 = p_noad.subscr_field(mem);
                                flush_node_list(mem, c2)?;
                                crate::mem::free_node(mem, p, FRACTION_NOAD_SIZE_FNL);
                            }
                            _ => {
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
                            }
                        }
                        break 'done;
                    }
                }
                crate::mem::free_node(mem, p, SMALL_NODE_SIZE);
            }
        }
        p = q;
    }
    Ok(())
}
/// translated from xetex-xetex0.c:953 (copy_node_list)
pub fn copy_node_list(
    mem: &mut crate::state::MemState,
    mut p: i32,
) -> Result<i32, NodeRecycleAbort> {
    use crate::mem::TEX_NULL;
    let h = cnl_get_avail!(mem);
    let mut q = h;
    while p != TEX_NULL {
        let mut words = 1i32;
        let r: i32;
        if p >= mem.hi_mem_min {
            r = cnl_get_avail!(mem);
        } else {
            match mem
                .classify(MemIdx::new(p))
                .map_err(|_| NodeRecycleAbort::Confusion(c"copying"))?
            {
                AnyNode::HList(src_box) => {
                    let copied = copy_node_list(mem, src_box.list_ptr(mem).0)?;
                    r = copy_box_shell(mem, p, MemPtr::from_raw(copied))?;
                    words = 0; // already copied
                }
                AnyNode::VList(src_box) => {
                    let copied = copy_node_list(mem, src_box.list_ptr(mem).0)?;
                    r = copy_box_shell(mem, p, MemPtr::from_raw(copied))?;
                    words = 0; // already copied
                }
                AnyNode::Unset(u) => {
                    let src = u.list_ptr(mem).0;
                    r = cnl_get_node!(mem, BOX_NODE_SIZE);
                    // SYNCTEX_tag(r, BOX_NODE_SIZE) = SYNCTEX_tag(p, BOX_NODE_SIZE)
                    mem.set_b32s0(r + 7, mem.b32s0(p + 7));
                    // SYNCTEX_line(r, BOX_NODE_SIZE) = SYNCTEX_line(p, BOX_NODE_SIZE)
                    mem.set_b32s1(r + 7, mem.b32s1(p + 7));
                    mem.set_word(r + 6, mem.word(p + 6));
                    mem.set_word(r + 5, mem.word(p + 5));
                    let copied = copy_node_list(mem, src)?;
                    mem.set_b32s1(r + 5, copied);
                    words = 5;
                }
                AnyNode::Rule(_) => {
                    r = cnl_get_node!(mem, RULE_NODE_SIZE);
                    words = RULE_NODE_SIZE - 1;
                }
                AnyNode::Ins(ins) => {
                    r = cnl_get_node!(mem, INS_NODE_SIZE_CNL);
                    mem.set_word(r + 4, mem.word(p + 4));
                    // GLUE_SPEC_ref_count(INSERTION_NODE_split_top_ptr(p))++
                    let spec =
                        NodePtr::<GlueSpecNode>::from_glue_ptr(ins.split_top_ptr(mem)).unwrap();
                    spec.set_ref_count(mem, spec.ref_count(mem) + 1);
                    let src = ins.ins_ptr(mem).0;
                    let copied = copy_node_list(mem, src)?;
                    mem.set_b32s0(r + 4, copied);
                    words = INS_NODE_SIZE_CNL - 1;
                }
                AnyNode::Whatsit(ws) => {
                    match ws.subtype(mem) {
                        OPEN_NODE_CNL => {
                            r = cnl_get_node!(mem, OPEN_NODE_SIZE_CNL);
                            words = OPEN_NODE_SIZE_CNL;
                        }
                        WRITE_NODE_CNL | SPECIAL_NODE_CNL => {
                            // mem[mem[p+1].b32.s1].b32.s0++ (token list ref count)
                            let tok = ws.write_tokens(mem).0;
                            mem.set_b32s0(tok, mem.b32s0(tok) + 1);
                            r = cnl_get_node!(mem, WRITE_NODE_SIZE_CNL);
                            words = WRITE_NODE_SIZE_CNL;
                        }
                        CLOSE_NODE_CNL | LANGUAGE_NODE_CNL => {
                            r = cnl_get_node!(mem, SMALL_NODE_SIZE);
                            words = SMALL_NODE_SIZE;
                        }
                        NATIVE_WORD_NODE_CNL | NATIVE_WORD_NODE_AT_CNL => {
                            let sz = ws.native_size(mem);
                            r = cnl_get_node!(mem, sz);
                            let mut nw = sz;
                            while nw > 0 {
                                nw -= 1;
                                mem.set_word(r + nw, mem.word(p + nw));
                            }
                            let rw = match mem
                                .classify(MemIdx::new(r))
                                .map_err(|_| NodeRecycleAbort::Confusion(c"ext2"))?
                            {
                                AnyNode::Whatsit(rw) => rw,
                                _ => unreachable!("copied whatsit header classifies as whatsit"),
                            };
                            rw.set_native_glyph_info_ptr(mem, std::ptr::null_mut());
                            rw.set_native_glyph_count(mem, 0);
                            crate::mem::copy_native_glyph_info(mem, p, r);
                            words = 0;
                        }
                        GLYPH_NODE_CNL => {
                            r = cnl_get_node!(mem, GLYPH_NODE_SIZE_CNL);
                            words = GLYPH_NODE_SIZE_CNL;
                        }
                        PIC_NODE_CNL | PDF_NODE_CNL => {
                            let path_len = ws.pic_path_len(mem) as i32;
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
                AnyNode::Glue(glue) => {
                    let gp = glue.glue_ptr(mem);
                    r = cnl_get_node!(mem, MEDIUM_NODE_SIZE);
                    // GLUE_SPEC_ref_count(GLUE_NODE_glue_ptr(p))++
                    let spec = NodePtr::<GlueSpecNode>::from_glue_ptr(gp).unwrap();
                    spec.set_ref_count(mem, spec.ref_count(mem) + 1);
                    mem.set_word(r, mem.word(p));
                    mem.set_word(r + 2, mem.word(p + 2));
                    let copied = copy_node_list(mem, glue.leader_ptr(mem).0)?;
                    let r_glue = match mem
                        .classify(MemIdx::new(r))
                        .map_err(|_| NodeRecycleAbort::Confusion(c"copying"))?
                    {
                        AnyNode::Glue(r_glue) => r_glue,
                        _ => unreachable!("copied glue header classifies as glue"),
                    };
                    r_glue.set_glue_ptr(mem, gp);
                    r_glue.set_leader_ptr(mem, MemPtr::from_raw(copied));
                    words = 0; // already copied
                }
                AnyNode::Kern(_) | AnyNode::Math(_) | AnyNode::Penalty(_) => {
                    r = cnl_get_node!(mem, MEDIUM_NODE_SIZE);
                    words = MEDIUM_NODE_SIZE;
                }
                AnyNode::MarginKern(_) => {
                    r = cnl_get_node!(mem, MARGIN_KERN_NODE_SIZE_CNL);
                    words = MARGIN_KERN_NODE_SIZE_CNL;
                }
                AnyNode::Ligature(lig) => {
                    r = cnl_get_node!(mem, SMALL_NODE_SIZE);
                    mem.set_word(r, mem.word(p));
                    mem.set_word(r + 1, mem.word(p + 1));
                    let src = lig.lig_ptr(mem).0;
                    let copied = copy_node_list(mem, src)?;
                    let r_lig = match mem
                        .classify(MemIdx::new(r))
                        .map_err(|_| NodeRecycleAbort::Confusion(c"copying"))?
                    {
                        AnyNode::Ligature(r_lig) => r_lig,
                        _ => unreachable!("copied ligature header classifies as ligature"),
                    };
                    r_lig.set_lig_ptr(mem, MemPtr::from_raw(copied));
                    words = 0; // already copied
                }
                AnyNode::Disc(disc) => {
                    r = cnl_get_node!(mem, SMALL_NODE_SIZE);
                    mem.set_word(r, mem.word(p));
                    let r_disc = match mem
                        .classify(MemIdx::new(r))
                        .map_err(|_| NodeRecycleAbort::Confusion(c"copying"))?
                    {
                        AnyNode::Disc(r_disc) => r_disc,
                        _ => unreachable!("copied disc header classifies as disc"),
                    };
                    let src0 = disc.pre_break(mem).0;
                    let copied0 = copy_node_list(mem, src0)?;
                    r_disc.set_pre_break(mem, MemPtr::from_raw(copied0));
                    let src1 = disc.post_break(mem).0;
                    let copied1 = copy_node_list(mem, src1)?;
                    r_disc.set_post_break(mem, MemPtr::from_raw(copied1));
                    words = 0; // already copied
                }
                AnyNode::Mark(mark) => {
                    // mem[MARK_NODE_ptr(p)].b32.s0++ (token list ref count)
                    let tok = mark.mark_ptr(mem).0;
                    r = cnl_get_node!(mem, SMALL_NODE_SIZE);
                    mem.set_b32s0(tok, mem.b32s0(tok) + 1);
                    words = SMALL_NODE_SIZE;
                }
                AnyNode::Adjust(adjust) => {
                    let src = adjust.adjust_ptr(mem).0;
                    r = cnl_get_node!(mem, SMALL_NODE_SIZE);
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
// translated from xetex-xetex0.c:8020 (append_to_vlist)
pub fn append_to_vlist(cl: &mut crate::mem_types::ListStateRecord, 
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
    if cl.aux_s1() > IGNORE_DEPTH {
        let baseline_spec = eqtb.b32s1(GLUE_BASE + GLUE_PAR_BASELINE_SKIP);
        // \baselineskip is a glue parameter spec (never null).
        let bsl =
            NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(baseline_spec)).expect("spec");
        let d = bsl.width(mem).0
            - cl.aux_s1()
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
            let bsl_spec = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(
                mem.temp_ptr,
            ))
            .expect("new_skip_param sets temp_ptr to a glue spec");
            bsl_spec.set_width(mem, Scaled(d));
            p
        };
        mem.set_b32s1(cl.tail, p);
        cl.tail = p;
    }
    mem.set_b32s1(cl.tail, b_idx);
    cl.tail = b_idx;
    cl.set_aux_s1(if upwards {
        b.height(mem).0
    } else {
        b.depth(mem).0
    });
    Ok(())
}

/// Classifies the raw box halfword then appends it.
pub(crate) fn append_to_vlist_emit_in(
    st: &mut crate::state::EngineState,
    b: i32,
) -> Result<(), NodeRecycleAbort> {
    let bx = st
        .mem
        .classify_box(MemIdx::new(b))
        .expect("valid node type for append_to_vlist")
        .expect("append_to_vlist receives an hlist or vlist node");
    append_to_vlist(&mut st.input.cur_list, &mut st.mem, &st.eqtb, bx)
}
// translated from xetex-xetex0.c:7725 (vert_break)
struct VertBreakCursor {
    p: MemPtr,
    prev_p: MemPtr,
    least_cost: i32,
    best_place: MemPtr,
    prev_dp: i32,
    finish_glue_after_shrinkage: Option<NodePtr<GlueNode>>,
}

enum VertBreakStep {
    Done(i32),
    InfiniteShrinkage {
        glue: NodePtr<GlueNode>,
        spec: NodePtr<GlueSpecNode>,
    },
}

enum VertBreakNext {
    NotFound,
    UpdateHeights(VertBreakUpdateSource),
    CheckPi {
        pi: i32,
        update_after: Option<VertBreakUpdateSource>,
    },
}

#[derive(Copy, Clone)]
enum VertBreakUpdateSource {
    Kern(NodePtr<KernNode>),
    Glue(NodePtr<GlueNode>),
}

enum VertBreakHeightSource {
    Kern(NodePtr<KernNode>),
    GlueSpec(NodePtr<GlueSpecNode>),
}

impl VertBreakHeightSource {
    fn width(self, mem: &MemState) -> i32 {
        match self {
            VertBreakHeightSource::Kern(kern) => kern.width(mem).0,
            VertBreakHeightSource::GlueSpec(spec) => spec.width(mem).0,
        }
    }
}

fn start_vert_break(scanner: &mut ScannerState, p: MemPtr) -> VertBreakCursor {
    for k in 1..=6usize {
        scanner.active_width[k] = 0;
    }

    VertBreakCursor {
        p,
        prev_p: p,
        least_cost: MAX_HALFWORD,
        best_place: MemPtr::NULL,
        prev_dp: 0,
        finish_glue_after_shrinkage: None,
    }
}

fn advance_vert_break(
    mem: &MemState,
    scanner: &mut ScannerState,
    cursor: &mut VertBreakCursor,
    d: i32,
) {
    if cursor.prev_dp > d {
        scanner.active_width[1] += cursor.prev_dp - d;
        cursor.prev_dp = d;
    }
    cursor.prev_p = cursor.p;
    cursor.p = MemPtr::from_raw(mem.b32s1(cursor.prev_p.0));
}

fn finish_vert_break_updated_height(
    mem: &MemState,
    scanner: &mut ScannerState,
    cursor: &mut VertBreakCursor,
    source: VertBreakHeightSource,
    d: i32,
) {
    scanner.active_width[1] += cursor.prev_dp + source.width(mem);
    cursor.prev_dp = 0;
    advance_vert_break(mem, scanner, cursor, d);
}

fn classify_vert_break_next(
    mem: &MemState,
    scanner: &mut ScannerState,
    cursor: &mut VertBreakCursor,
) -> Result<VertBreakNext, NodeRecycleAbort> {
    use crate::mem::TEX_NULL;

    const EJECT_PENALTY: i32 = -10_000; // xetex-constants.h:336

    if cursor.p.is_null() {
        return Ok(VertBreakNext::CheckPi {
            pi: EJECT_PENALTY,
            update_after: None,
        });
    }

    let p = cursor.p.as_idx().expect("non-null vert_break cursor");
    let node = mem
        .classify(p)
        .map_err(|_| NodeRecycleAbort::Confusion(c"vertbreak"))?;
    match node {
        AnyNode::HList(b) => {
            scanner.active_width[1] += cursor.prev_dp + b.height(mem).0;
            cursor.prev_dp = b.depth(mem).0;
            Ok(VertBreakNext::NotFound)
        }
        AnyNode::VList(b) => {
            scanner.active_width[1] += cursor.prev_dp + b.height(mem).0;
            cursor.prev_dp = b.depth(mem).0;
            Ok(VertBreakNext::NotFound)
        }
        AnyNode::Rule(rule) => {
            scanner.active_width[1] += cursor.prev_dp + rule.height(mem).raw();
            cursor.prev_dp = rule.depth(mem).raw();
            Ok(VertBreakNext::NotFound)
        }
        AnyNode::Whatsit(w) => {
            let sub = w.whatsit_subtype(mem);
            if sub == Some(WhatsitSubtype::Pic) || sub == Some(WhatsitSubtype::Pdf) {
                scanner.active_width[1] += cursor.prev_dp + w.height(mem).0;
                cursor.prev_dp = w.depth(mem).0;
            }
            Ok(VertBreakNext::NotFound)
        }
        AnyNode::Glue(glue) => {
            if mem.b16s1(cursor.prev_p.0) < MATH_NODE {
                Ok(VertBreakNext::CheckPi {
                    pi: 0,
                    update_after: Some(VertBreakUpdateSource::Glue(glue)),
                })
            } else {
                Ok(VertBreakNext::UpdateHeights(VertBreakUpdateSource::Glue(
                    glue,
                )))
            }
        }
        AnyNode::Kern(kern) => {
            let link = kern.link(mem).0;
            if link != TEX_NULL
                && mem
                    .classify_glue(MemIdx::new(link))
                    .ok()
                    .flatten()
                    .is_some()
            {
                Ok(VertBreakNext::CheckPi {
                    pi: 0,
                    update_after: Some(VertBreakUpdateSource::Kern(kern)),
                })
            } else {
                Ok(VertBreakNext::UpdateHeights(VertBreakUpdateSource::Kern(
                    kern,
                )))
            }
        }
        AnyNode::Penalty(penalty) => Ok(VertBreakNext::CheckPi {
            pi: penalty.penalty(mem),
            update_after: None,
        }),
        AnyNode::Ins(_) | AnyNode::Mark(_) => Ok(VertBreakNext::NotFound),
        _ => Err(NodeRecycleAbort::Confusion(c"vertbreak")),
    }
}

fn vert_break_resume(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    cursor: &mut VertBreakCursor,
    h: i32,
    d: i32,
) -> Result<VertBreakStep, NodeRecycleAbort> {
    const EJECT_PENALTY: i32 = -10_000; // xetex-constants.h:336
    const INF_PENALTY: i32 = 10_000; // xetex-constants.h:338
    const INF_BAD: i32 = 10_000; // xetex-constants.h:337
    const NORMAL: u16 = 0;

    loop {
        if let Some(glue) = cursor.finish_glue_after_shrinkage.take() {
            let spec = glue.glue_spec(mem).expect("glue node has glue_spec");
            finish_vert_break_updated_height(
                mem,
                scanner,
                cursor,
                VertBreakHeightSource::GlueSpec(spec),
                d,
            );
            continue;
        }

        let update_source = match classify_vert_break_next(mem, scanner, cursor)? {
            VertBreakNext::NotFound => None,
            VertBreakNext::UpdateHeights(source) => Some(source),
            VertBreakNext::CheckPi { pi, update_after } => {
                if pi < INF_PENALTY {
                    let b = if scanner.active_width[1] < h {
                        if scanner.active_width[3] != 0
                            || scanner.active_width[4] != 0
                            || scanner.active_width[5] != 0
                        {
                            0
                        } else {
                            crate::helpers::badness(
                                h - scanner.active_width[1],
                                scanner.active_width[2],
                            )
                        }
                    } else if scanner.active_width[1] - h > scanner.active_width[6] {
                        MAX_HALFWORD
                    } else {
                        crate::helpers::badness(
                            scanner.active_width[1] - h,
                            scanner.active_width[6],
                        )
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
                    if b <= cursor.least_cost {
                        cursor.best_place = cursor.p;
                        cursor.least_cost = b;
                        scanner.best_height_plus_depth = scanner.active_width[1] + cursor.prev_dp;
                    }
                    if b == MAX_HALFWORD || pi <= EJECT_PENALTY {
                        return Ok(VertBreakStep::Done(cursor.best_place.0));
                    }
                }
                // In the TEX_NULL case pi == EJECT_PENALTY, so we always
                // return above. If we continue here, p is a valid node.
                update_after
            }
        };

        if let Some(update_source) = update_source {
            match update_source {
                VertBreakUpdateSource::Kern(kern) => {
                    finish_vert_break_updated_height(
                        mem,
                        scanner,
                        cursor,
                        VertBreakHeightSource::Kern(kern),
                        d,
                    );
                    continue;
                }
                VertBreakUpdateSource::Glue(glue) => {
                    let spec = glue.glue_spec(mem).expect("glue node has glue_spec");
                    let stretch_order = spec.stretch_order(mem) as usize;
                    scanner.active_width[2 + stretch_order] += spec.stretch(mem).0;
                    scanner.active_width[6] += spec.shrink(mem).0;
                    if spec.shrink_order(mem) != NORMAL && spec.shrink(mem).0 != 0 {
                        cursor.finish_glue_after_shrinkage = Some(glue);
                        return Ok(VertBreakStep::InfiniteShrinkage { glue, spec });
                    }
                    finish_vert_break_updated_height(
                        mem,
                        scanner,
                        cursor,
                        VertBreakHeightSource::GlueSpec(spec),
                        d,
                    );
                    continue;
                }
            }
        }

        advance_vert_break(mem, scanner, cursor, d);
        if cursor.p.is_null() {
            continue;
        }
    }
}

fn print_vert_break_infinite_shrinkage_error_in(
    st: &mut crate::state::EngineState,
) -> ErrorOutcome {
    crate::output::error_here_with_diagnostic_in(
        st,
        c"Infinite glue shrinkage found in box being split".as_ptr(),
    );
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 4;
    st.errors.help_line[3] = c"The box you are \\vsplitting contains some infinitely".as_ptr();
    st.errors.help_line[2] =
        c"shrinkable glue, e.g., `\\vss' or `\\vskip 0pt minus 1fil'.".as_ptr();
    st.errors.help_line[1] =
        c"Such glue doesn't belong there; but you can safely proceed,".as_ptr();
    st.errors.help_line[0] = c"since the offensive shrinkability has been made finite.".as_ptr();
    crate::errors::error_in(st)
}

fn repair_vert_break_infinite_shrinkage(
    mem: &mut MemState,
    glue: NodePtr<GlueNode>,
    spec: NodePtr<GlueSpecNode>,
) -> Result<(), NodeRecycleAbort> {
    const NORMAL: u16 = 0;

    let r = crate::mem::new_spec(mem, spec.idx().0);
    if r == crate::mem::ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    let new_spec =
        NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(r)).expect("new_spec returns spec");
    new_spec.set_shrink_order(mem, NORMAL);
    crate::mem::delete_glue_ref(mem, spec.idx().0);
    glue.set_glue_ptr(mem, MemPtr::from_raw(r));
    Ok(())
}

pub(crate) enum VertBreakAbort {
    NodeRecycle(NodeRecycleAbort),
    Error(ErrorOutcome),
}

impl From<NodeRecycleAbort> for VertBreakAbort {
    fn from(e: NodeRecycleAbort) -> Self {
        VertBreakAbort::NodeRecycle(e)
    }
}

impl VertBreakAbort {
    /// Map to the unified [`EngineAbort`] terminal, mirroring the
    /// `PageBuildAbort::VertBreak` arms of `fire_page_build_abort`.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            VertBreakAbort::NodeRecycle(e) => e.into_engine_abort(),
            VertBreakAbort::Error(o) => o.into_engine_abort(),
        }
    }
}

/// Runs the vertical-break state machine, emits the infinite-shrinkage
/// diagnostic, and repairs the offending glue.
pub(crate) fn vert_break_in(
    st: &mut crate::state::EngineState,
    p: i32,
    h: i32,
    d: i32,
) -> Result<i32, VertBreakAbort> {
    let mut cursor = start_vert_break(&mut st.scanner, MemPtr::from_raw(p));
    loop {
        match vert_break_resume(&mut st.mem, &mut st.scanner, &mut cursor, h, d)? {
            VertBreakStep::Done(best_place) => return Ok(best_place),
            VertBreakStep::InfiniteShrinkage { glue, spec } => {
                let outcome = print_vert_break_infinite_shrinkage_error_in(st);
                if !matches!(outcome, ErrorOutcome::Continue) {
                    return Err(VertBreakAbort::Error(outcome));
                }
                repair_vert_break_infinite_shrinkage(&mut st.mem, glue, spec)?;
            }
        }
    }
}

// translated from xetex-xetex0.c:7727 (vsplit)
const DIMEN_PAR_SPLIT_MAX_DEPTH_VS: i32 = 6; // xetex_format.h:128
const INT_PAR_SAVING_VDISCARDS_VS: i32 = 63; // xetex_format.h:98
const EXACTLY_VS: i16 = 0; // xetex-constants.h:256
const ADDITIONAL_VS: i16 = 1; // xetex-constants.h:264
const VSPLIT_CODE_VS: usize = 3; // xetex_format.h:483
const MARK_VAL_VS: usize = 7; // xetex-constants.h:300
const SPLIT_FIRST_MARK_CODE_VS: usize = 3; // xetex_format.h:688
const SPLIT_BOT_MARK_CODE_VS: usize = 4; // xetex_format.h:689

struct VsplitState {
    h: i32,
    v: NodePtr<VListNode>,
    split_max_d: i32,
    vert_break: VertBreakCursor,
}

#[derive(Copy, Clone)]
struct VsplitOutcome {
    result: i32,
    packages: [Option<PackOutcome>; 2],
}

enum VsplitStart {
    Done(VsplitOutcome),
    NeedsVboxDiagnostic,
    Ready(VsplitState),
}

enum VsplitStep {
    Done(VsplitOutcome),
    InfiniteShrinkage {
        glue: NodePtr<GlueNode>,
        spec: NodePtr<GlueSpecNode>,
    },
}

fn push_vsplit_package(packages: &mut [Option<PackOutcome>; 2], outcome: PackOutcome) {
    if packages[0].is_none() {
        packages[0] = Some(outcome);
    } else {
        packages[1] = Some(outcome);
    }
}

fn empty_vsplit_outcome(result: i32) -> VsplitOutcome {
    VsplitOutcome {
        result,
        packages: [None, None],
    }
}

fn find_box_register(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    eqtb: &EqtbState,
    n: i32,
) -> Result<i32, NodeRecycleAbort> {
    use crate::mem::TEX_NULL;

    scanner.cur_val = n;
    if scanner.cur_val < 256 {
        Ok(eqtb.b32s1(BOX_BASE + scanner.cur_val))
    } else {
        let r = crate::save::find_sa_element(mem, scanner, 4, scanner.cur_val, false);
        if r == crate::mem::ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        if scanner.cur_ptr == TEX_NULL {
            Ok(TEX_NULL)
        } else {
            Ok(mem.b32s1(scanner.cur_ptr + 1))
        }
    }
}

fn reset_vsplit_marks(
    mem: &mut MemState,
    scanner: &mut ScannerState,
) -> Result<(), NodeRecycleAbort> {
    use crate::mem::TEX_NULL;

    let disc = scanner.disc_ptr[VSPLIT_CODE_VS];
    flush_node_list(mem, disc).map_err(NodeRecycleAbort::Confusion)?;
    scanner.disc_ptr[VSPLIT_CODE_VS] = TEX_NULL;

    let vsr = scanner.sa_root[MARK_VAL_VS];
    if vsr != TEX_NULL && do_marks(mem, scanner, 0, 0, vsr) {
        scanner.sa_root[MARK_VAL_VS] = TEX_NULL;
    }

    if scanner.cur_mark[SPLIT_FIRST_MARK_CODE_VS] != TEX_NULL {
        let first = scanner.cur_mark[SPLIT_FIRST_MARK_CODE_VS];
        crate::mem::delete_token_ref(mem, first);
        scanner.cur_mark[SPLIT_FIRST_MARK_CODE_VS] = TEX_NULL;
        let bot = scanner.cur_mark[SPLIT_BOT_MARK_CODE_VS];
        crate::mem::delete_token_ref(mem, bot);
        scanner.cur_mark[SPLIT_BOT_MARK_CODE_VS] = TEX_NULL;
    }

    Ok(())
}

fn start_vsplit(
    mem: &mut MemState,
    eqtb: &EqtbState,
    scanner: &mut ScannerState,
    n: i32,
    h: i32,
) -> Result<VsplitStart, NodeRecycleAbort> {
    use crate::mem::TEX_NULL;

    let v = find_box_register(mem, scanner, eqtb, n)?;
    reset_vsplit_marks(mem, scanner)?;

    if v == TEX_NULL {
        return Ok(VsplitStart::Done(empty_vsplit_outcome(TEX_NULL)));
    }

    let v = match mem
        .classify_box(MemIdx::new(v))
        .map_err(|_| NodeRecycleAbort::Confusion(c"vsplit"))?
    {
        Some(AnyBox::VList(v)) => v,
        Some(AnyBox::HList(_)) | None => return Ok(VsplitStart::NeedsVboxDiagnostic),
    };

    let split_max_d = eqtb.b32s1(DIMEN_BASE + DIMEN_PAR_SPLIT_MAX_DEPTH_VS);
    Ok(VsplitStart::Ready(VsplitState {
        h,
        v,
        split_max_d,
        vert_break: start_vert_break(scanner, v.list_ptr(mem)),
    }))
}

fn collect_split_mark(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    mark: NodePtr<MarkNode>,
) -> Result<(), NodeRecycleAbort> {
    use crate::mem::TEX_NULL;

    let mark_class = mark.mark_class(mem);
    let mark_ptr = mark.mark_ptr(mem).0;
    if mark_class != 0 {
        let r = crate::save::find_sa_element(mem, scanner, MARK_VAL_VS as u16, mark_class, true);
        if r == crate::mem::ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        if mem.b32s1(scanner.cur_ptr + 2) == TEX_NULL {
            mem.set_b32s1(scanner.cur_ptr + 2, mark_ptr);
            mem.set_b32s0(mark_ptr, mem.b32s0(mark_ptr) + 1);
        } else {
            let old = mem.b32s0(scanner.cur_ptr + 3);
            crate::mem::delete_token_ref(mem, old);
        }
        mem.set_b32s0(scanner.cur_ptr + 3, mark_ptr);
        mem.set_b32s0(mark_ptr, mem.b32s0(mark_ptr) + 1);
    } else if scanner.cur_mark[SPLIT_FIRST_MARK_CODE_VS] == TEX_NULL {
        scanner.cur_mark[SPLIT_FIRST_MARK_CODE_VS] = mark_ptr;
        scanner.cur_mark[SPLIT_BOT_MARK_CODE_VS] = mark_ptr;
        mem.set_b32s0(mark_ptr, mem.b32s0(mark_ptr) + 2);
    } else {
        let old = scanner.cur_mark[SPLIT_BOT_MARK_CODE_VS];
        crate::mem::delete_token_ref(mem, old);
        scanner.cur_mark[SPLIT_BOT_MARK_CODE_VS] = mark_ptr;
        mem.set_b32s0(mark_ptr, mem.b32s0(mark_ptr) + 1);
    }
    Ok(())
}

fn split_vlist_at_break(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    state: &VsplitState,
    q: i32,
) -> Result<i32, NodeRecycleAbort> {
    use crate::mem::TEX_NULL;

    let mut p = state.v.list_ptr(mem).0;
    if p == q {
        state.v.set_list_ptr(mem, MemPtr::NULL);
    } else {
        loop {
            if let AnyNode::Mark(mark) = mem
                .classify(MemIdx::new(p))
                .map_err(|_| NodeRecycleAbort::Confusion(c"vsplit"))?
            {
                collect_split_mark(mem, scanner, mark)?;
            }
            if mem.b32s1(p) == q {
                mem.set_b32s1(p, TEX_NULL);
                break;
            }
            p = mem.b32s1(p);
        }
    }

    Ok(state.v.list_ptr(mem).0)
}

fn store_split_remainder(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    eqtb: &mut EqtbState,
    q: i32,
) -> Result<(), NodeRecycleAbort> {
    use crate::mem::TEX_NULL;

    if scanner.cur_val < 256 {
        eqtb.set_b32s1(BOX_BASE + scanner.cur_val, q);
    } else {
        let r = crate::save::find_sa_element(mem, scanner, 4, scanner.cur_val, false);
        if r == crate::mem::ALLOC_FAILED {
            return Err(NodeRecycleAbort::Overflow);
        }
        if scanner.cur_ptr != TEX_NULL {
            mem.set_b32s1(scanner.cur_ptr + 1, q);
            mem.set_b32s0(scanner.cur_ptr + 1, mem.b32s0(scanner.cur_ptr + 1) + 1);
            crate::save::delete_sa_ref(mem, scanner, scanner.cur_ptr);
        }
    }
    Ok(())
}

fn finish_vsplit_after_break(
    mem: &mut MemState,
    eqtb: &mut EqtbState,
    scanner: &mut ScannerState,
    shipout: &mut ShipoutState,
    state: &VsplitState,
    q: i32,
) -> Result<VsplitOutcome, NodeRecycleAbort> {
    use crate::mem::TEX_NULL;

    let mut packages = [None, None];
    let p = split_vlist_at_break(mem, scanner, state, q)?;
    let save_discards = eqtb.b32s1(INT_BASE + INT_PAR_SAVING_VDISCARDS_VS) > 0;
    let mut q = prune_page_top(mem, scanner, eqtb, q, save_discards)?;

    crate::mem::free_node(mem, state.v.idx().0, BOX_NODE_SIZE);
    if q != TEX_NULL {
        let packed = vpackage_in(
            mem,
            eqtb,
            scanner,
            shipout,
            q,
            0,
            ADDITIONAL_VS,
            MAX_HALFWORD,
        )?;
        q = packed.r_box.idx().0;
        push_vsplit_package(&mut packages, packed);
    }
    store_split_remainder(mem, scanner, eqtb, q)?;

    let result = vpackage_in(
        mem,
        eqtb,
        scanner,
        shipout,
        p,
        state.h,
        EXACTLY_VS,
        state.split_max_d,
    )?;
    let r = result.r_box.idx().0;
    push_vsplit_package(&mut packages, result);

    Ok(VsplitOutcome {
        result: r,
        packages,
    })
}

fn vsplit_resume(
    mem: &mut MemState,
    eqtb: &mut EqtbState,
    scanner: &mut ScannerState,
    shipout: &mut ShipoutState,
    state: &mut VsplitState,
) -> Result<VsplitStep, NodeRecycleAbort> {
    loop {
        match vert_break_resume(
            mem,
            scanner,
            &mut state.vert_break,
            state.h,
            state.split_max_d,
        )? {
            VertBreakStep::Done(q) => {
                let outcome = finish_vsplit_after_break(mem, eqtb, scanner, shipout, state, q)?;
                return Ok(VsplitStep::Done(outcome));
            }
            VertBreakStep::InfiniteShrinkage { glue, spec } => {
                return Ok(VsplitStep::InfiniteShrinkage { glue, spec });
            }
        }
    }
}

fn print_vsplit_needs_vbox_error_in(st: &mut crate::state::EngineState) -> ErrorOutcome {
    crate::output::error_here_with_diagnostic_in(st, c"".as_ptr());
    crate::output::print_esc_cstr_in(st, c"vsplit".as_ptr());
    crate::output::print_cstr_in(st, c" needs a ".as_ptr());
    crate::output::print_esc_cstr_in(st, c"vbox".as_ptr());
    crate::output::capture_to_diagnostic_in(st, std::ptr::null_mut());
    st.errors.help_ptr = 2;
    st.errors.help_line[1] = c"The box you are trying to split is an \\hbox.".as_ptr();
    st.errors.help_line[0] = c"I can't split such a box, so I'll leave it alone.".as_ptr();
    crate::errors::error_in(st)
}

fn print_vsplit_packages_in(st: &mut crate::state::EngineState, outcome: VsplitOutcome) {
    for package in outcome.packages.into_iter().flatten() {
        for diag in package.diags.into_iter().flatten() {
            print_vpack_diag_in(st, package.r_box, diag);
        }
    }
}

/// Cold abort sentinel for [`vsplit_in`].
pub(crate) enum VsplitAbort {
    Node(NodeRecycleAbort),
    Error(ErrorOutcome),
}

impl From<NodeRecycleAbort> for VsplitAbort {
    fn from(e: NodeRecycleAbort) -> Self {
        VsplitAbort::Node(e)
    }
}

impl VsplitAbort {
    /// Map to the unified [`EngineAbort`] terminal, mirroring the
    /// `MainControlAbort::Vsplit` arms of `MainControlAbort::fire`.
    pub(crate) fn into_engine_abort(self) -> crate::errors::EngineAbort {
        match self {
            VsplitAbort::Node(e) => e.into_engine_abort(),
            VsplitAbort::Error(o) => o.into_engine_abort(),
        }
    }
}

/// The over/underfull-vbox and infinite-shrinkage diagnostics print through
/// their `_in` cores.
pub(crate) fn vsplit_in(
    st: &mut crate::state::EngineState,
    n: i32,
    h: i32,
) -> Result<i32, VsplitAbort> {
    let mut state = match start_vsplit(&mut st.mem, &st.eqtb, &mut st.scanner, n, h)? {
        VsplitStart::Done(outcome) => {
            let result = outcome.result;
            print_vsplit_packages_in(st, outcome);
            return Ok(result);
        }
        VsplitStart::NeedsVboxDiagnostic => {
            let outcome = print_vsplit_needs_vbox_error_in(st);
            if !matches!(outcome, ErrorOutcome::Continue) {
                return Err(VsplitAbort::Error(outcome));
            }
            return Ok(crate::mem::TEX_NULL);
        }
        VsplitStart::Ready(state) => state,
    };

    loop {
        match vsplit_resume(
            &mut st.mem,
            &mut st.eqtb,
            &mut st.scanner,
            &mut st.shipout,
            &mut state,
        )? {
            VsplitStep::Done(outcome) => {
                let result = outcome.result;
                print_vsplit_packages_in(st, outcome);
                return Ok(result);
            }
            VsplitStep::InfiniteShrinkage { glue, spec } => {
                let outcome = print_vert_break_infinite_shrinkage_error_in(st);
                if !matches!(outcome, ErrorOutcome::Continue) {
                    return Err(VsplitAbort::Error(outcome));
                }
                repair_vert_break_infinite_shrinkage(&mut st.mem, glue, spec)?;
            }
        }
    }
}

fn insert_split_top_skip_before(
    mem: &mut MemState,
    eqtb: &EqtbState,
    prev_p: i32,
    p: i32,
    height: i32,
) -> Result<(), NodeRecycleAbort> {
    let q = crate::mem::new_skip_param(mem, eqtb, GLUE_PAR_SPLIT_TOP_SKIP_PPT);
    if q == crate::mem::ALLOC_FAILED {
        return Err(NodeRecycleAbort::Overflow);
    }
    mem.set_b32s1(prev_p, q);
    mem.set_b32s1(q, p);
    let spec = NodePtr::<GlueSpecNode>::from_glue_ptr(MemPtr::from_raw(mem.temp_ptr))
        .expect("new_skip_param sets temp_ptr to a glue spec");
    let avail_ht = spec.width(mem).0;
    if avail_ht > height {
        spec.set_width(mem, Scaled(avail_ht - height));
    } else {
        spec.set_width(mem, Scaled(0));
    }
    Ok(())
}

#[derive(Copy, Clone)]
enum PruneDiscardNode {
    Glue(NodePtr<GlueNode>),
    Kern(NodePtr<KernNode>),
    Penalty(NodePtr<crate::node::PenaltyNode>),
}

impl PruneDiscardNode {
    fn idx(self) -> MemIdx {
        match self {
            PruneDiscardNode::Glue(node) => node.idx(),
            PruneDiscardNode::Kern(node) => node.idx(),
            PruneDiscardNode::Penalty(node) => node.idx(),
        }
    }

    fn link(self, mem: &MemState) -> MemPtr {
        match self {
            PruneDiscardNode::Glue(node) => node.link(mem),
            PruneDiscardNode::Kern(node) => node.link(mem),
            PruneDiscardNode::Penalty(node) => node.link(mem),
        }
    }

    fn set_link(self, mem: &mut MemState, link: MemPtr) {
        match self {
            PruneDiscardNode::Glue(node) => node.set_link(mem, link),
            PruneDiscardNode::Kern(node) => node.set_link(mem, link),
            PruneDiscardNode::Penalty(node) => node.set_link(mem, link),
        }
    }
}

fn prune_discard_node(
    mem: &mut MemState,
    scanner: &mut ScannerState,
    prev_p: i32,
    r: &mut i32,
    node: PruneDiscardNode,
    save_discards: bool,
) -> Result<i32, NodeRecycleAbort> {
    use crate::mem::TEX_NULL;

    let q = node.link(mem).0;
    node.set_link(mem, MemPtr::NULL);
    mem.set_b32s1(prev_p, q);
    if save_discards {
        let discarded = node.idx().0;
        if scanner.disc_ptr[VSPLIT_CODE_PPT] == TEX_NULL {
            scanner.disc_ptr[VSPLIT_CODE_PPT] = discarded;
        } else {
            mem.set_b32s1(*r, discarded);
        }
        *r = discarded;
    } else {
        flush_node_list(mem, node.idx().0).map_err(NodeRecycleAbort::Confusion)?;
    }
    Ok(q)
}

/// translated from xetex-xetex0.c:9761 (prune_page_top)
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
    let mut r = TEX_NULL;

    prev_p = TEMP_HEAD;
    mem.set_b32s1(TEMP_HEAD, p);

    while p != TEX_NULL {
        match mem
            .classify(MemIdx::new(p))
            .map_err(|_| NodeRecycleAbort::Confusion(c"pruning"))?
        {
            AnyNode::HList(b) => {
                insert_split_top_skip_before(mem, eqtb, prev_p, p, b.height(mem).0)?;
                p = TEX_NULL;
            }
            AnyNode::VList(b) => {
                insert_split_top_skip_before(mem, eqtb, prev_p, p, b.height(mem).0)?;
                p = TEX_NULL;
            }
            AnyNode::Rule(rule) => {
                insert_split_top_skip_before(mem, eqtb, prev_p, p, rule.height(mem).raw())?;
                p = TEX_NULL;
            }
            AnyNode::Whatsit(_) | AnyNode::Mark(_) | AnyNode::Ins(_) => {
                prev_p = p;
                p = mem.b32s1(prev_p);
            }
            AnyNode::Glue(node) => {
                p = prune_discard_node(
                    mem,
                    scanner,
                    prev_p,
                    &mut r,
                    PruneDiscardNode::Glue(node),
                    s,
                )?;
            }
            AnyNode::Kern(node) => {
                p = prune_discard_node(
                    mem,
                    scanner,
                    prev_p,
                    &mut r,
                    PruneDiscardNode::Kern(node),
                    s,
                )?;
            }
            AnyNode::Penalty(node) => {
                p = prune_discard_node(
                    mem,
                    scanner,
                    prev_p,
                    &mut r,
                    PruneDiscardNode::Penalty(node),
                    s,
                )?;
            }
            _ => {
                return Err(NodeRecycleAbort::Confusion(c"pruning"));
            }
        }
    }

    Ok(mem.b32s1(TEMP_HEAD))
}
/// translated from xetex-xetex0.c:9817 (do_marks)
pub fn do_marks(
    mem: &mut crate::state::MemState,
    scanner: &mut crate::state::ScannerState,
    a: i16,
    l: i16,
    q: i32,
) -> bool {
    use crate::mem::TEX_NULL;
    let mut q = q;
    // SAFETY: `do_marks` is only entered (and recursed) with a real
    // sparse-array record index — an index node (l < 4) or a mark-class node
    // (l == 4). This is the family's single audited construction site; it has
    // no NODE_type to classify. Child/mark slot `i` maps to `mem[q+i/2+1].b32`.
    let node = unsafe { NodePtr::<SaNode>::from_idx_unchecked(MemIdx::new(q)) };

    if l < 4 {
        for i in 0i16..=15 {
            scanner.cur_ptr = node.child(mem, i as i32).0;

            let cur = scanner.cur_ptr;
            if cur != TEX_NULL && do_marks(mem, scanner, a, l + 1, cur) {
                node.set_child(mem, i as i32, MemPtr::NULL);
                // index node populated-child reference count--
                node.set_ref_count(mem, node.ref_count(mem) - 1);
            }
        }

        if node.ref_count(mem) == 0 {
            crate::mem::free_node(mem, q, INDEX_NODE_SIZE_DM);
            q = TEX_NULL;
        }
    } else {
        // Mark-class node child slots, by eTeX mark code:
        //   0 = top, 1 = first, 2 = bot, 3 = split_first, 4 = split_bot.
        match a {
            VSPLIT_INIT_DM => {
                if !node.child(mem, 3).is_null() {
                    crate::mem::delete_token_ref(mem, node.child(mem, 3).0);
                    node.set_child(mem, 3, MemPtr::NULL);
                    crate::mem::delete_token_ref(mem, node.child(mem, 4).0);
                    node.set_child(mem, 4, MemPtr::NULL);
                }
            }
            FIRE_UP_INIT_DM => {
                if !node.child(mem, 2).is_null() {
                    if !node.child(mem, 0).is_null() {
                        crate::mem::delete_token_ref(mem, node.child(mem, 0).0);
                    }
                    crate::mem::delete_token_ref(mem, node.child(mem, 1).0);
                    node.set_child(mem, 1, MemPtr::NULL);
                    let ref_ptr = node.child(mem, 2).0;
                    if mem.b32s1(ref_ptr) == TEX_NULL {
                        crate::mem::delete_token_ref(mem, ref_ptr);
                        node.set_child(mem, 2, MemPtr::NULL);
                    } else {
                        mem.set_b32s0(ref_ptr, mem.b32s0(ref_ptr) + 1);
                    }
                    let v = node.child(mem, 2);
                    node.set_child(mem, 0, v);
                }
            }
            FIRE_UP_DONE_DM => {
                if !node.child(mem, 0).is_null() && node.child(mem, 1).is_null() {
                    let head = node.child(mem, 0);
                    node.set_child(mem, 1, head);
                    let cur = mem.b32s0(head.0);
                    mem.set_b32s0(head.0, cur + 1);
                }
            }
            DESTROY_MARKS_DM => {
                for i in TOP_MARK_CODE_DM..=SPLIT_BOT_MARK_CODE_DM {
                    scanner.cur_ptr = node.child(mem, i as i32).0;

                    if scanner.cur_ptr != TEX_NULL {
                        crate::mem::delete_token_ref(mem, scanner.cur_ptr);
                        node.set_child(mem, i as i32, MemPtr::NULL);
                    }
                }
            }
            _ => {}
        }

        if node.child(mem, 2).is_null() && node.child(mem, 4).is_null() {
            crate::mem::free_node(mem, q, MARK_CLASS_NODE_SIZE_DM);
            q = TEX_NULL;
        }
    }

    q == TEX_NULL
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
        match mem.classify(node_idx) {
            Ok(AnyNode::HList(h_box)) => {
                let b = AnyBox::HList(h_box);
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
            Ok(AnyNode::VList(v_box)) => {
                let b = AnyBox::VList(v_box);
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
            Ok(AnyNode::Rule(rule)) => {
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
            Ok(AnyNode::Unset(u)) => {
                x += u.width(mem).0;
                let uh = u.height(mem).0;
                if uh > h {
                    h = uh;
                }
                let ud = u.depth(mem).0;
                if ud > d {
                    d = ud;
                }
            }
            Ok(AnyNode::Adjust(adjust)) => {
                if scanner.adjust_tail != TEX_NULL || scanner.pre_adjust_tail != TEX_NULL {
                    while mem.b32s1(q) != p {
                        q = mem.b32s1(q);
                    }
                    let adjust_list = adjust.adjust_ptr(mem).0;
                    if adjust.subtype(mem) != 0 {
                        if scanner.pre_adjust_tail == TEX_NULL {
                            return Err(NodeRecycleAbort::Confusion(c"pre vadjust"));
                        }
                        mem.set_b32s1(scanner.pre_adjust_tail, adjust_list);
                        while mem.b32s1(scanner.pre_adjust_tail) != TEX_NULL {
                            scanner.pre_adjust_tail = mem.b32s1(scanner.pre_adjust_tail);
                        }
                    } else {
                        if scanner.adjust_tail == TEX_NULL {
                            return Err(NodeRecycleAbort::Confusion(c"pre vadjust"));
                        }
                        mem.set_b32s1(scanner.adjust_tail, adjust_list);
                        while mem.b32s1(scanner.adjust_tail) != TEX_NULL {
                            scanner.adjust_tail = mem.b32s1(scanner.adjust_tail);
                        }
                    }
                    let removed = p;
                    p = adjust.link(mem).0;
                    crate::mem::free_node(mem, removed, SMALL_NODE_SIZE);
                    mem.set_b32s1(q, p);
                    p = q;
                }
            }
            Ok(AnyNode::Ins(ins)) => {
                if scanner.adjust_tail != TEX_NULL || scanner.pre_adjust_tail != TEX_NULL {
                    while mem.b32s1(q) != p {
                        q = mem.b32s1(q);
                    }
                    mem.set_b32s1(scanner.adjust_tail, p);
                    scanner.adjust_tail = p;
                    p = ins.link(mem).0;
                    mem.set_b32s1(q, p);
                    p = q;
                }
            }
            Ok(AnyNode::Mark(mark)) => {
                if scanner.adjust_tail != TEX_NULL || scanner.pre_adjust_tail != TEX_NULL {
                    while mem.b32s1(q) != p {
                        q = mem.b32s1(q);
                    }
                    mem.set_b32s1(scanner.adjust_tail, p);
                    scanner.adjust_tail = p;
                    p = mark.link(mem).0;
                    mem.set_b32s1(q, p);
                    p = q;
                }
            }
            Ok(AnyNode::Whatsit(w)) => {
                let sub = w.subtype(mem);
                // The matched node's native font, used to test mergeability of the
                // following native-word fragments.
                let p_font = w.native_font(mem).0;
                match sub {
                    NATIVE_WORD_NODE_HP | NATIVE_WORD_NODE_AT_HP => {
                        let mut k: i32 = if q != r_node.idx().offset(5).0 {
                            if let Some(disc) = classify_disc_hp(mem, q) {
                                disc.subtype(mem) as i32
                            } else {
                                0
                            }
                        } else {
                            0
                        };
                        while mem.b32s1(q) != p {
                            k -= 1;
                            q = mem.b32s1(q);
                            if let Some(disc) = classify_disc_hp(mem, q) {
                                k = disc.subtype(mem) as i32;
                            }
                        }
                        let mut pp = mem.b32s1(p);
                        loop {
                            if k <= 0 && pp != TEX_NULL && pp < mem.hi_mem_min {
                                if let Some(ppw) =
                                    classify_native_word_with_font_hp(mem, pp, p_font)
                                {
                                    pp = ppw.link(mem).0;
                                    continue;
                                } else if let Some(disc) = classify_disc_hp(mem, pp) {
                                    let ppp = disc.link(mem).0;
                                    if let Some(pppw) =
                                        classify_native_word_with_font_hp(mem, ppp, p_font)
                                    {
                                        pp = pppw.link(mem).0;
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
                                if let Some(scan_w) = classify_native_word_hp(mem, scan_p) {
                                    total_chars += scan_w.native_length(mem) as i32;
                                }
                                scan_p = mem.b32s1(scan_p);
                            }
                            let scan_p = mem.b32s1(q);
                            let scan_w = classify_native_word_hp(mem, scan_p)
                                .expect("native merge starts at a native-word node");
                            let merged = new_native_word_node(
                                mem,
                                eqtb,
                                scan_w.native_font(mem).0,
                                total_chars,
                            )?;
                            let merged_w = mem
                                .classify_whatsit(MemIdx::new(merged))
                                .map_err(|_| NodeRecycleAbort::Confusion(c"hpack"))?
                                .expect("new_native_word_node returns a whatsit");
                            merged_w.set_subtype(mem, scan_w.subtype(mem));
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
                                if let Some(src_w) = classify_native_word_hp(mem, src_p) {
                                    let count = src_w.native_length(mem) as i32;
                                    let src = src_w.native_text_ptr(mem);
                                    let dst = merged_w.native_text_ptr_mut(mem);
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
                                )?;
                            }
                        }
                        let p_native = mem
                            .classify_whatsit(MemIdx::new(p))
                            .map_err(|_| NodeRecycleAbort::Confusion(c"hpack"))?
                            .expect("native-word hpack cursor remains a whatsit");
                        let ph = p_native.height(mem).0;
                        if ph > h {
                            h = ph;
                        }
                        let pd = p_native.depth(mem).0;
                        if pd > d {
                            d = pd;
                        }
                        x += p_native.width(mem).0;
                    }
                    42..=44 => {
                        // GLYPH_NODE / PIC_NODE / PDF_NODE: `p` is unchanged here
                        // (only the native-word branch above can reassign it), so
                        // the matched handle still applies.
                        if w.height(mem).0 > h {
                            h = w.height(mem).0;
                        }
                        if w.depth(mem).0 > d {
                            d = w.depth(mem).0;
                        }
                        x += w.width(mem).0;
                    }
                    _ => {}
                }
            }
            Ok(AnyNode::Glue(glue)) => {
                let spec = glue.glue_spec(mem).expect("glue node has glue_spec");
                x += spec.width(mem).0;
                let o = spec.stretch_order(mem) as usize;
                scanner.total_stretch[o] += spec.stretch(mem).0;
                let o = spec.shrink_order(mem) as usize;
                scanner.total_shrink[o] += spec.shrink(mem).0;
                if glue.subtype(mem) >= A_LEADERS_HP {
                    let leader = glue.leader_ptr(mem).0;
                    let (leader_h, leader_d) = match mem.classify(MemIdx::new(leader)).ok() {
                        Some(AnyNode::HList(leader_box)) => {
                            (leader_box.height(mem).0, leader_box.depth(mem).0)
                        }
                        Some(AnyNode::VList(leader_box)) => {
                            (leader_box.height(mem).0, leader_box.depth(mem).0)
                        }
                        Some(AnyNode::Rule(rule)) => {
                            (rule.height(mem).raw(), rule.depth(mem).raw())
                        }
                        _ => (mem.b32s1(leader + 3), mem.b32s1(leader + 2)),
                    };
                    if leader_h > h {
                        h = leader_h;
                    }
                    if leader_d > d {
                        d = leader_d;
                    }
                }
            }
            Ok(AnyNode::Kern(kern)) => {
                x += kern.width(mem).0;
            }
            Ok(AnyNode::Math(math)) => {
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
            Ok(AnyNode::Ligature(lig)) => {
                mem.set_b16s1(GARBAGE, lig.font(mem).0 as u16);
                mem.set_b16s0(GARBAGE, lig.character(mem));
                mem.set_b32s1(GARBAGE, lig.link(mem).0);
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
        if o == NORMAL_HP && !r_node.list_ptr(mem).is_null() {
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
        if scanner.total_shrink[o] < (-x) && o == NORMAL_HP && !r_node.list_ptr(mem).is_null() {
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
                    let Some(rule_node) = crate::mem::alloc_rule(mem) else {
                        return Err(NodeRecycleAbort::Overflow);
                    };
                    mem.set_b32s1(q, rule_node.idx().0);
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
        } else if o == NORMAL_HP && !r_node.list_ptr(mem).is_null() {
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

    Ok(PackOutcome {
        r_box: AnyBox::HList(r_node),
        diags,
    })
}

/// Emit the over/underfull-`\hbox` diagnostic.
pub(crate) fn print_hpack_diag_in(
    st: &mut crate::state::EngineState,
    box_ptr: AnyBox,
    diag: PackDiag,
) {
    use crate::output::{
        diagnostic_begin_capture_warning_here_in, print_cstr_in, print_int_in, print_ln_in,
        print_nl_cstr_in, print_scaled_in,
    };
    match diag {
        PackDiag::HUnderfull { badness, loose } => {
            print_ln_in(st);
            diagnostic_begin_capture_warning_here_in(st);
            if loose {
                print_nl_cstr_in(st, c"Loose".as_ptr());
            } else {
                print_nl_cstr_in(st, c"Underfull".as_ptr());
            }
            print_cstr_in(st, c" \\hbox (badness ".as_ptr());
            print_int_in(st, badness);
        }
        PackDiag::HOverfull { excess } => {
            print_ln_in(st);
            diagnostic_begin_capture_warning_here_in(st);
            print_nl_cstr_in(st, c"Overfull \\hbox (".as_ptr());
            print_scaled_in(st, excess);
            print_cstr_in(st, c"pt too wide".as_ptr());
        }
        PackDiag::HTight { badness } => {
            print_ln_in(st);
            diagnostic_begin_capture_warning_here_in(st);
            print_nl_cstr_in(st, c"Tight \\hbox (badness ".as_ptr());
            print_int_in(st, badness);
        }
        PackDiag::LrProblem { missing, extra } => {
            print_ln_in(st);
            diagnostic_begin_capture_warning_here_in(st);
            print_nl_cstr_in(st, c"\\endL or \\endR problem (".as_ptr());
            print_int_in(st, missing);
            print_cstr_in(st, c" missing, ".as_ptr());
            print_int_in(st, extra);
            print_cstr_in(st, c" extra".as_ptr());
        }
        PackDiag::VUnderfull { .. } | PackDiag::VOverfull { .. } | PackDiag::VTight { .. } => {
            unreachable!("vpack diagnostic passed to hpack printer")
        }
    }
    print_hpack_common_ending_in(st, box_ptr);
}

fn print_hpack_common_ending_in(st: &mut crate::state::EngineState, box_ptr: AnyBox) {
    use crate::output::{
        begin_diagnostic_in, capture_to_diagnostic_in, end_diagnostic_in, print_cstr_in,
        print_int_in, print_ln_in, short_display_in, show_box_in,
    };
    if st.shipout.output_active {
        print_cstr_in(st, c") has occurred while \\output is active".as_ptr());
    } else {
        let pack_begin_line = st.scanner.pack_begin_line;
        if pack_begin_line != 0 {
            if pack_begin_line > 0 {
                print_cstr_in(st, c") in paragraph at lines ".as_ptr());
            } else {
                print_cstr_in(st, c") in alignment at lines ".as_ptr());
            }
            print_int_in(st, pack_begin_line.abs());
            print_cstr_in(st, c"--".as_ptr());
        } else {
            print_cstr_in(st, c") detected at line ".as_ptr());
        }
        print_int_in(st, st.mem.line);
    }
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    print_ln_in(st);
    st.errors.font_in_short_display = 0;
    let list = box_ptr.list_ptr(&st.mem).0;
    short_display_in(st, list);
    print_ln_in(st);
    begin_diagnostic_in(st);
    show_box_in(st, box_ptr.idx().0);
    end_diagnostic_in(st, true);
}

/// Runs the [`hpack_in`] core and prints any over/underfull-`\hbox` diagnostic
/// (via [`print_hpack_diag_in`]) so log order is preserved.
pub(crate) fn hpack_emit_in(
    st: &mut crate::state::EngineState,
    p: i32,
    w: i32,
    m: i16,
) -> Result<AnyBox, NodeRecycleAbort> {
    let outcome = hpack_in(
        &mut st.mem,
        &st.eqtb,
        &mut st.fonts,
        &mut st.scanner,
        &mut st.shipout,
        p,
        w,
        m,
    )?;
    for diag in outcome.diags.into_iter().flatten() {
        print_hpack_diag_in(st, outcome.r_box, diag);
    }
    // `hpack_in` classified the freshly-allocated `HListNode` at its alloc site
    // and carried the handle out in `PackOutcome`; hand it to callers verbatim.
    Ok(outcome.r_box)
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
            match mem.classify(node_idx) {
                Ok(AnyNode::HList(h_box)) => {
                    let b = AnyBox::HList(h_box);
                    x += d + b.height(mem).0;
                    d = b.depth(mem).0;
                    let width = b.width(mem).0 + b.shift_amount(mem).0;
                    if width > w {
                        w = width;
                    }
                }
                Ok(AnyNode::VList(v_box)) => {
                    let b = AnyBox::VList(v_box);
                    x += d + b.height(mem).0;
                    d = b.depth(mem).0;
                    let width = b.width(mem).0 + b.shift_amount(mem).0;
                    if width > w {
                        w = width;
                    }
                }
                Ok(AnyNode::Rule(rule)) => {
                    x += d + rule.height(mem).raw();
                    d = rule.depth(mem).raw();
                    let width = rule.width(mem).raw();
                    if width > w {
                        w = width;
                    }
                }
                Ok(AnyNode::Unset(u)) => {
                    x += d + u.height(mem).0;
                    d = u.depth(mem).0;
                    let uw = u.width(mem).0;
                    if uw > w {
                        w = uw;
                    }
                }
                Ok(AnyNode::Whatsit(wn)) => {
                    let sub = wn.subtype(mem);
                    if sub == PIC_NODE_VP || sub == PDF_NODE_VP {
                        x += d + wn.height(mem).0;
                        d = wn.depth(mem).0;
                        let ww = wn.width(mem).0;
                        if ww > w {
                            w = ww;
                        }
                    }
                }
                Ok(AnyNode::Glue(glue)) => {
                    x += d;
                    d = 0;
                    let spec = glue.glue_spec(mem).expect("glue node has glue_spec");
                    x += spec.width(mem).0;
                    let o = spec.stretch_order(mem) as usize;
                    scanner.total_stretch[o] += spec.stretch(mem).0;
                    let o = spec.shrink_order(mem) as usize;
                    scanner.total_shrink[o] += spec.shrink(mem).0;
                    if glue.subtype(mem) >= A_LEADERS_VP {
                        let leader = glue.leader_ptr(mem).0;
                        let leader_w = match mem.classify(MemIdx::new(leader)).ok() {
                            Some(AnyNode::HList(leader_box)) => leader_box.width(mem).0,
                            Some(AnyNode::VList(leader_box)) => leader_box.width(mem).0,
                            Some(AnyNode::Rule(rule)) => rule.width(mem).raw(),
                            _ => mem.b32s1(leader + 1),
                        };
                        if leader_w > w {
                            w = leader_w;
                        }
                    }
                }
                Ok(AnyNode::Kern(kern)) => {
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
        if o == NORMAL_VP && !r_node.list_ptr(mem).is_null() {
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
        if scanner.total_shrink[o] < (-x) && o == NORMAL_VP && !r_node.list_ptr(mem).is_null() {
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
        } else if o == NORMAL_VP && !r_node.list_ptr(mem).is_null() {
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
        r_box: AnyBox::VList(r_node),
        diags,
    })
}

/// Emit the over/underfull-`\vbox` diagnostic.
pub(crate) fn print_vpack_diag_in(
    st: &mut crate::state::EngineState,
    box_ptr: AnyBox,
    diag: PackDiag,
) {
    use crate::output::{
        diagnostic_begin_capture_warning_here_in, print_cstr_in, print_int_in, print_ln_in,
        print_nl_cstr_in, print_scaled_in,
    };
    match diag {
        PackDiag::VUnderfull { badness, loose } => {
            print_ln_in(st);
            diagnostic_begin_capture_warning_here_in(st);
            if loose {
                print_nl_cstr_in(st, c"Loose".as_ptr());
            } else {
                print_nl_cstr_in(st, c"Underfull".as_ptr());
            }
            print_cstr_in(st, c" \\vbox (badness ".as_ptr());
            print_int_in(st, badness);
        }
        PackDiag::VOverfull { excess } => {
            print_ln_in(st);
            diagnostic_begin_capture_warning_here_in(st);
            print_nl_cstr_in(st, c"Overfull \\vbox (".as_ptr());
            print_scaled_in(st, excess);
            print_cstr_in(st, c"pt too high".as_ptr());
        }
        PackDiag::VTight { badness } => {
            print_ln_in(st);
            diagnostic_begin_capture_warning_here_in(st);
            print_nl_cstr_in(st, c"Tight \\vbox (badness ".as_ptr());
            print_int_in(st, badness);
        }
        PackDiag::HUnderfull { .. }
        | PackDiag::HOverfull { .. }
        | PackDiag::HTight { .. }
        | PackDiag::LrProblem { .. } => {
            unreachable!("hpack diagnostic passed to vpack printer")
        }
    }
    print_vpack_common_ending_in(st, box_ptr);
}

fn print_vpack_common_ending_in(st: &mut crate::state::EngineState, box_ptr: AnyBox) {
    use crate::output::{
        begin_diagnostic_in, capture_to_diagnostic_in, end_diagnostic_in, print_cstr_in,
        print_int_in, print_ln_in, show_box_in,
    };
    if st.shipout.output_active {
        print_cstr_in(st, c") has occurred while \\output is active".as_ptr());
    } else {
        let pack_begin_line = st.scanner.pack_begin_line;
        if pack_begin_line != 0 {
            print_cstr_in(st, c") in alignment at lines ".as_ptr());
            print_int_in(st, pack_begin_line.abs());
            print_cstr_in(st, c"--".as_ptr());
        } else {
            print_cstr_in(st, c") detected at line ".as_ptr());
        }
        print_int_in(st, st.mem.line);
        print_ln_in(st);
    }
    capture_to_diagnostic_in(st, std::ptr::null_mut());
    begin_diagnostic_in(st);
    show_box_in(st, box_ptr.idx().0);
    end_diagnostic_in(st, true);
}

/// Runs [`vpackage_in`] and emits any over/underfull diagnostic.
pub(crate) fn vpackage_emit_in(
    st: &mut crate::state::EngineState,
    p: i32,
    h: i32,
    m: i16,
    l: i32,
) -> Result<AnyBox, NodeRecycleAbort> {
    let outcome = vpackage_in(
        &mut st.mem,
        &st.eqtb,
        &mut st.scanner,
        &mut st.shipout,
        p,
        h,
        m,
        l,
    )?;
    for diag in outcome.diags.into_iter().flatten() {
        print_vpack_diag_in(st, outcome.r_box, diag);
    }
    // `vpackage_in` carried the classified `VListNode` handle out in
    // `PackOutcome`; hand it to callers verbatim.
    Ok(outcome.r_box)
}
