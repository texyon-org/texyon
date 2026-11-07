//! Displaying boxes: `show_box`, `short_display` and `show_node_list`.

use crate::consts::{
    ACC_KERN, AFTER, A_LEADERS, COND_MATH_GLUE, C_LEADERS, DLIST, EMPTY, L_CODE, MATH_CHAR,
    MU_GLUE, R_CODE, SPACE_ADJUSTMENT, SUB_BOX, SUB_MLIST, TEX_NULL, X_LEADERS,
};
use crate::math::{DEFAULT_CODE, LIMITS};
use crate::mem::ZERO_GLUE;
use crate::node::{AnyNode, BoxNode, CharNode, MathField, NodePtr, WhatsitNode, WhatsitSubtype};
use crate::params::IntPar;
use crate::state::EngineState;
use crate::units::{GlueSign, MemPtr, NodeType, Scaled};

use super::show::show_info;
use super::{
    print, print_char, print_current_string, print_delimiter, print_esc, print_fam_and_char,
    print_file_name, print_font_and_char, print_font_char, print_glue, print_int, print_ln,
    print_mark, print_native_word, print_raw_char, print_rule_dimen, print_scaled,
    print_skip_param, print_spec, print_style, print_write_whatsit,
};

pub(crate) fn show_box(st: &mut EngineState, p: MemPtr) {
    st.errors.depth_threshold = st.eqtb.int_par(IntPar::ShowBoxDepth);
    st.errors.breadth_max = st.eqtb.int_par(IntPar::ShowBoxBreadth);
    if st.errors.breadth_max <= 0 {
        st.errors.breadth_max = 5;
    }
    let room = st.strings.pool_size - st.strings.pool_ptr - 1;
    if st.errors.depth_threshold > room {
        st.errors.depth_threshold = room;
    }
    show_node_list(st, p);
    print_ln(st);
}

pub(crate) fn short_display(st: &mut EngineState, mut p: MemPtr) {
    while p.raw() > 0 {
        if st.mem.is_char_node(p) {
            if p <= st.mem.mem_end {
                let ch = NodePtr::<CharNode>::at(p);
                let font = st.mem[ch].font;
                if font != st.errors.font_in_short_display {
                    if font > st.fonts.font_max {
                        print_char(st, b'*');
                    } else {
                        print_esc(st, st.eqtb.font_id_text(font));
                    }
                    print_char(st, b' ');
                    st.errors.font_in_short_display = font;
                }
                print(st, i32::from(st.mem[ch].character));
            }
        } else {
            match st.mem.classify(p) {
                Ok(
                    AnyNode::Box(_)
                    | AnyNode::Ins(_)
                    | AnyNode::Mark(_)
                    | AnyNode::Adjust(_)
                    | AnyNode::Unset(_),
                ) => print(st, "[]"),
                Ok(AnyNode::Whatsit(w)) => {
                    if w.whatsit_subtype(&st.mem)
                        .is_some_and(|s| s.is_native_word())
                    {
                        let font = w.native_font(&st.mem);
                        if font != st.errors.font_in_short_display {
                            print_esc(st, st.eqtb.font_id_text(font));
                            print_char(st, b' ');
                            st.errors.font_in_short_display = font;
                        }
                        print_native_word(st, w);
                    } else {
                        print(st, "[]");
                    }
                }
                Ok(AnyNode::Rule(_)) => print_char(st, b'|'),
                Ok(AnyNode::Glue(g)) => {
                    // Glue pointing at the shared zero spec prints nothing.
                    if st.mem[g].glue_ptr != ZERO_GLUE {
                        print_char(st, b' ');
                    }
                }
                Ok(AnyNode::Math(m)) => {
                    if st.mem[m].subtype >= L_CODE {
                        print(st, "[]");
                    } else {
                        print_char(st, b'$');
                    }
                }
                Ok(AnyNode::Ligature(l)) => short_display(st, st.mem[l].lig_ptr),
                Ok(AnyNode::Disc(d)) => {
                    short_display(st, st.mem[d].pre_break);
                    short_display(st, st.mem[d].post_break);
                    for _ in 0..st.mem[d].subtype {
                        if !p.link(&st.mem).is_null() {
                            p = p.link(&st.mem);
                        }
                    }
                }
                _ => {}
            }
        }
        p = p.link(&st.mem);
    }
}

/// Shows sublist `list` with `c` appended to the indentation string.
fn node_list_display(st: &mut EngineState, c: u8, list: MemPtr) {
    st.strings.append_char(u16::from(c));
    show_node_list(st, list);
    st.strings.pool_ptr -= 1;
}

/// Shows the noad field at `p`, marked with `c` in the indentation string.
fn print_subsidiary_data(st: &mut EngineState, p: NodePtr<MathField>, c: u8) {
    if st.strings.cur_length() >= st.errors.depth_threshold {
        if p.math_type(&st.mem) != EMPTY {
            print(st, " []");
        }
        return;
    }
    st.strings.append_char(u16::from(c));
    st.mem.temp_ptr = p.ptr();
    match p.math_type(&st.mem) {
        MATH_CHAR => {
            print_ln(st);
            print_current_string(st);
            print_fam_and_char(st, p);
        }
        SUB_BOX => show_info(st),
        SUB_MLIST => {
            if p.info(&st.mem).is_null() {
                print_ln(st);
                print_current_string(st);
                print(st, "{}");
            } else {
                show_info(st);
            }
        }
        _ => {}
    }
    st.strings.pool_ptr -= 1;
}

pub(super) fn show_node_list(st: &mut EngineState, mut p: MemPtr) {
    if st.strings.cur_length() > st.errors.depth_threshold {
        if p.raw() > TEX_NULL {
            print(st, " []");
        }
        return;
    }
    let mut n = 0;
    while p.raw() > 0 {
        print_ln(st);
        print_current_string(st);
        if p > st.mem.mem_end {
            print(st, "Bad link, display aborted.");
            return;
        }
        n += 1;
        if n > st.errors.breadth_max {
            print(st, "etc.");
            return;
        }
        if st.mem.is_char_node(p) {
            print_font_and_char(st, p);
        } else {
            match st.mem.classify(p) {
                Ok(AnyNode::Box(b)) => show_box_node(st, b),
                Ok(AnyNode::Unset(node)) => {
                    print_esc(st, "unset");
                    print(st, "box(");
                    print_scaled(st, st.mem[node].height);
                    print_char(st, b'+');
                    print_scaled(st, st.mem[node].depth);
                    print(st, ")x");
                    print_scaled(st, st.mem[node].width);
                    let span_count = st.mem[node].subtype;
                    if span_count != 0 {
                        print(st, " (");
                        print_int(st, i32::from(span_count) + 1);
                        print(st, " columns)");
                    }
                    if st.mem[node].glue_stretch != Scaled::ZERO {
                        print(st, ", stretch ");
                        let order = st.mem[node].glue_order;
                        print_glue(st, st.mem[node].glue_stretch, order, None);
                    }
                    if st.mem[node].glue_shrink != Scaled::ZERO {
                        print(st, ", shrink ");
                        let order = st.mem[node].glue_sign;
                        print_glue(st, st.mem[node].glue_shrink, order, None);
                    }
                    node_list_display(st, b'.', st.mem[node].list_ptr);
                }
                Ok(AnyNode::Rule(node)) => {
                    print_esc(st, "rule(");
                    print_rule_dimen(st, st.mem[node].height.get());
                    print_char(st, b'+');
                    print_rule_dimen(st, st.mem[node].depth.get());
                    print(st, ")x");
                    print_rule_dimen(st, st.mem[node].width.get());
                }
                Ok(AnyNode::Ins(node)) => {
                    print_esc(st, "insert");
                    print_int(st, i32::from(st.mem[node].subtype));
                    print(st, ", natural size ");
                    print_scaled(st, st.mem[node].height);
                    print(st, "; split(");
                    print_spec(st, st.mem[node].split_top_ptr.ptr(), None);
                    print_char(st, b',');
                    print_scaled(st, st.mem[node].depth);
                    print(st, "); float cost ");
                    print_int(st, st.mem[node].float_cost);
                    node_list_display(st, b'.', st.mem[node].ins_ptr);
                }
                Ok(AnyNode::Whatsit(node)) => show_whatsit(st, node),
                Ok(AnyNode::Glue(node)) => show_glue(st, node),
                Ok(AnyNode::Kern(node)) => {
                    let sub = st.mem[node].subtype;
                    if sub != MU_GLUE {
                        print_esc(st, "kern");
                        if sub != 0 {
                            print_char(st, b' ');
                        }
                        print_scaled(st, st.mem[node].width);
                        if sub == ACC_KERN {
                            print(st, " (for accent)");
                        } else if sub == SPACE_ADJUSTMENT {
                            print(st, " (space adjustment)");
                        }
                    } else {
                        print_esc(st, "mkern");
                        print_scaled(st, st.mem[node].width);
                        print(st, "mu");
                    }
                }
                Ok(AnyNode::MarginKern(node)) => {
                    print_esc(st, "kern");
                    print_scaled(st, st.mem[node].width);
                    if st.mem[node].subtype == 0 {
                        print(st, " (left margin)");
                    } else {
                        print(st, " (right margin)");
                    }
                }
                Ok(AnyNode::Math(node)) => {
                    let sub = st.mem[node].subtype;
                    if sub > AFTER {
                        print_esc(st, if sub % 2 == 1 { "end" } else { "begin" });
                        let dir = if sub > R_CODE {
                            b'R'
                        } else if sub > L_CODE {
                            b'L'
                        } else {
                            b'M'
                        };
                        print_char(st, i32::from(dir));
                    } else {
                        print_esc(st, "math");
                        if sub == 0 {
                            print(st, "on");
                        } else {
                            print(st, "off");
                        }
                        if st.mem[node].width != Scaled::ZERO {
                            print(st, ", surrounded ");
                            print_scaled(st, st.mem[node].width);
                        }
                    }
                }
                Ok(AnyNode::Ligature(node)) => {
                    let font = st.mem[node].font;
                    print_font_char(st, font, st.mem[node].character);
                    print(st, " (ligature ");
                    if st.mem[node].subtype > 1 {
                        print_char(st, b'|');
                    }
                    st.errors.font_in_short_display = font;
                    short_display(st, st.mem[node].lig_ptr);
                    if st.mem[node].subtype % 2 == 1 {
                        print_char(st, b'|');
                    }
                    print_char(st, b')');
                }
                Ok(AnyNode::Penalty(node)) => {
                    print_esc(st, "penalty ");
                    print_int(st, st.mem[node].penalty);
                }
                Ok(AnyNode::Disc(node)) => {
                    print_esc(st, "discretionary");
                    let replace_count = st.mem[node].subtype;
                    if replace_count > 0 {
                        print(st, " replacing ");
                        print_int(st, i32::from(replace_count));
                    }
                    node_list_display(st, b'.', st.mem[node].pre_break);
                    node_list_display(st, b'|', st.mem[node].post_break);
                }
                Ok(AnyNode::Mark(node)) => {
                    print_esc(st, "mark");
                    let class = st.mem[node].mark_class;
                    if class != 0 {
                        print_char(st, b's');
                        print_int(st, class);
                    }
                    print_mark(st, st.mem[node].mark_ptr);
                }
                Ok(AnyNode::Adjust(node)) => {
                    print_esc(st, "vadjust");
                    if st.mem[node].subtype != 0 {
                        print(st, " pre ");
                    }
                    node_list_display(st, b'.', st.mem[node].adjust_ptr);
                }
                Ok(AnyNode::Style(node)) => print_style(st, i32::from(st.mem[node].subtype)),
                Ok(AnyNode::Choice(node)) => {
                    print_esc(st, "mathchoice");
                    node_list_display(st, b'D', st.mem[node].display_mlist);
                    node_list_display(st, b'T', st.mem[node].text_mlist);
                    node_list_display(st, b'S', st.mem[node].script_mlist);
                    node_list_display(st, b's', st.mem[node].script_script_mlist);
                }
                Ok(AnyNode::Noad(node)) => show_noad(st, node),
                Err(_) => print(st, "Unknown node type!"),
            }
        }
        p = p.link(&st.mem);
    }
}

fn show_box_node(st: &mut EngineState, b: NodePtr<BoxNode>) {
    let is_hlist = st.mem[b].is_hlist();
    print_esc(st, i32::from(if is_hlist { b'h' } else { b'v' }));
    print(st, "box(");
    print_scaled(st, st.mem[b].height);
    print_char(st, b'+');
    print_scaled(st, st.mem[b].depth);
    print(st, ")x");
    print_scaled(st, st.mem[b].width);
    let g = st.mem[b].glue_set;
    let glue_sign = st.mem[b].glue_sign;
    if g != 0.0 && glue_sign != GlueSign::Normal {
        print(st, ", glue set ");
        if glue_sign == GlueSign::Shrinking {
            print(st, "- ");
        }
        let order = st.mem[b].glue_order;
        if g.abs() > 20000.0 {
            if g > 0.0 {
                print_char(st, b'>');
            } else {
                print(st, "< -");
            }
            print_glue(st, Scaled::UNITY * 20000, order, None);
        } else {
            let set = Scaled::round(65536.0 * g);
            print_glue(st, set, order, None);
        }
    }
    if st.mem[b].shift_amount != Scaled::ZERO {
        print(st, ", shifted ");
        print_scaled(st, st.mem[b].shift_amount);
    }
    if is_hlist && st.mem[b].subtype == DLIST {
        print(st, ", display");
    }
    node_list_display(st, b'.', st.mem[b].list_ptr);
}

fn show_whatsit(st: &mut EngineState, node: NodePtr<WhatsitNode>) {
    let print_font = |st: &mut EngineState| {
        print_esc(st, st.eqtb.font_id_text(node.native_font(&st.mem)));
    };
    match node.whatsit_subtype(&st.mem) {
        Some(WhatsitSubtype::Open) => {
            print_write_whatsit(st, "openout", node);
            print_char(st, b'=');
            let (name, area, ext) = (
                node.open_name(&st.mem),
                node.open_area(&st.mem),
                node.open_ext(&st.mem),
            );
            print_file_name(st, name, area, ext);
        }
        Some(WhatsitSubtype::Write) => {
            print_write_whatsit(st, "write", node);
            print_mark(st, node.write_tokens(&st.mem));
        }
        Some(WhatsitSubtype::Close) => print_write_whatsit(st, "closeout", node),
        Some(WhatsitSubtype::Special) => {
            print_esc(st, "special");
            print_mark(st, node.write_tokens(&st.mem));
        }
        Some(WhatsitSubtype::Language) => {
            print_esc(st, "setlanguage");
            print_int(st, node.what_lang(&st.mem));
            print(st, " (hyphenmin ");
            print_int(st, i32::from(node.what_lhm(&st.mem)));
            print_char(st, b',');
            print_int(st, i32::from(node.what_rhm(&st.mem)));
            print_char(st, b')');
        }
        Some(WhatsitSubtype::PdfSavePos) => print_esc(st, "pdfsavepos"),
        Some(WhatsitSubtype::NativeWord | WhatsitSubtype::NativeWordAt) => {
            print_font(st);
            print_char(st, b' ');
            print_native_word(st, node);
        }
        Some(WhatsitSubtype::Glyph) => {
            print_font(st);
            print(st, " glyph#");
            print_int(st, i32::from(node.native_glyph(&st.mem)));
        }
        Some(sub @ (WhatsitSubtype::Pic | WhatsitSubtype::Pdf)) => {
            if sub == WhatsitSubtype::Pic {
                print_esc(st, "XeTeXpicfile");
            } else {
                print_esc(st, "XeTeXpdffile");
            }
            print(st, " \"");
            for i in 0..node.pic_path_len(&st.mem) as usize {
                let b = node.pic_path(&st.mem)[i];
                print_raw_char(st, i32::from(b), true);
            }
            print(st, b'"');
        }
        _ => print(st, "whatsit?"),
    }
}

fn show_glue(st: &mut EngineState, node: NodePtr<crate::node::GlueNode>) {
    let sub = st.mem[node].subtype;
    if sub >= A_LEADERS {
        print_esc(st, "");
        if sub == C_LEADERS {
            print_char(st, b'c');
        } else if sub == X_LEADERS {
            print_char(st, b'x');
        }
        print(st, "leaders ");
        print_spec(st, st.mem[node].glue_ptr.ptr(), None);
        node_list_display(st, b'.', st.mem[node].leader_ptr);
        return;
    }
    print_esc(st, "glue");
    if sub != 0 {
        print_char(st, b'(');
        if sub < COND_MATH_GLUE {
            print_skip_param(st, i32::from(sub) - 1);
        } else if sub == COND_MATH_GLUE {
            print_esc(st, "nonscript");
        } else {
            print_esc(st, "mskip");
        }
        print_char(st, b')');
    }
    if sub != COND_MATH_GLUE {
        print_char(st, b' ');
        let unit = (sub >= COND_MATH_GLUE).then_some("mu");
        print_spec(st, st.mem[node].glue_ptr.ptr(), unit);
    }
}

fn show_noad(st: &mut EngineState, node: NodePtr<crate::node::Noad>) {
    let nt = node.noad_type(&st.mem);
    if nt == NodeType::FractionNoad {
        print_esc(st, "fraction, thickness ");
        let thickness = node.thickness(&st.mem);
        if thickness == DEFAULT_CODE {
            print(st, "= default");
        } else {
            print_scaled(st, thickness);
        }
        let left = node.left_delimiter();
        if !left.is_null(&st.mem) {
            print(st, ", left-delimiter ");
            print_delimiter(st, left);
        }
        let right = node.right_delimiter();
        if !right.is_null(&st.mem) {
            print(st, ", right-delimiter ");
            print_delimiter(st, right);
        }
        print_subsidiary_data(st, node.numerator(), b'\\');
        print_subsidiary_data(st, node.denominator(), b'/');
        return;
    }

    let name = match nt {
        NodeType::OrdNoad => "mathord",
        NodeType::OpNoad => "mathop",
        NodeType::BinNoad => "mathbin",
        NodeType::RelNoad => "mathrel",
        NodeType::OpenNoad => "mathopen",
        NodeType::CloseNoad => "mathclose",
        NodeType::PunctNoad => "mathpunct",
        NodeType::InnerNoad => "mathinner",
        NodeType::OverNoad => "overline",
        NodeType::UnderNoad => "underline",
        NodeType::VcenterNoad => "vcenter",
        NodeType::RadicalNoad => "radical",
        NodeType::AccentNoad => "accent",
        NodeType::LeftNoad => "left",
        NodeType::RightNoad if node.subtype(&st.mem) == 0 => "right",
        NodeType::RightNoad => "middle",
        _ => "",
    };
    if !name.is_empty() {
        print_esc(st, name);
    }
    match nt {
        NodeType::RadicalNoad => print_delimiter(st, node.left_delimiter()),
        NodeType::AccentNoad => print_fam_and_char(st, node.accent_chr()),
        NodeType::LeftNoad | NodeType::RightNoad => print_delimiter(st, node.delimiter()),
        _ => {}
    }
    if !matches!(nt, NodeType::LeftNoad | NodeType::RightNoad) {
        let sub = node.subtype(&st.mem);
        if sub != 0 {
            print_esc(st, if sub == LIMITS { "limits" } else { "nolimits" });
        }
        print_subsidiary_data(st, node.nucleus(), b'.');
    }
    print_subsidiary_data(st, node.supscr(), b'^');
    print_subsidiary_data(st, node.subscr(), b'_');
}
