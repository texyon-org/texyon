//! Displaying boxes: `show_box`, `short_display` and `show_node_list`.

use crate::base::codes::LimitCode;
use crate::base::consts::{EMPTY, MATH_CHAR, SUB_BOX, SUB_MLIST};
use crate::base::params::IntPar;
use crate::base::units::{GlueSign, MemPtr, NodeType, Scaled};
use crate::build::math::DEFAULT_CODE;
use crate::fonts::Side;
use crate::memory::mem::ZERO_GLUE;
use crate::memory::node::MathWord;
use crate::memory::node::{
    AdjustPlacement, AnyNoad, AnyNode, BoxLr, BoxNode, CharNode, FenceSubtype, GlueKind,
    KernSubtype, LeaderKind, MathCode, NoadVariant, NodePtr, WhatsitNode, WhatsitSubtype,
    WhatsitVariant,
};
use crate::state::EngineState;

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
    while let Some(mut here) = p {
        if st.mem.is_char_node(here) {
            if st.mem.contains(p) {
                let ch = NodePtr::<CharNode>::at(here);
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
                print(
                    st,
                    crate::base::units::StrNumber::single_char(i32::from(st.mem[ch].character)),
                );
            }
        } else {
            match st.mem.classify(here) {
                Ok(
                    AnyNode::Box(_)
                    | AnyNode::Ins(_)
                    | AnyNode::Mark(_)
                    | AnyNode::Adjust(_)
                    | AnyNode::Unset(_),
                ) => print(st, "[]"),
                Ok(AnyNode::Whatsit(w)) => {
                    if let Some(WhatsitVariant::NativeWord(w)) = w.variant(&st.mem) {
                        let font = st.mem[w].font;
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
                    if st.mem[m].subtype.get().is_some_and(MathCode::is_lr) {
                        print(st, "[]");
                    } else {
                        print_char(st, b'$');
                    }
                }
                Ok(AnyNode::Ligature(l)) => short_display(st, st.mem[l].lig_ptr),
                Ok(AnyNode::Disc(d)) => {
                    short_display(st, st.mem[d].pre_break);
                    short_display(st, st.mem[d].post_break);
                    for _ in 0..st.mem[d].replace_count {
                        if let Some(next) = here.link(&st.mem) {
                            here = next;
                        }
                    }
                }
                _ => {}
            }
        }
        p = here.link(&st.mem);
    }
}

/// Shows sublist `list` with `c` appended to the indentation string.
fn node_list_display(st: &mut EngineState, c: u8, list: MemPtr) {
    st.strings.append_char(u16::from(c));
    show_node_list(st, list);
    st.strings.pool_ptr -= 1;
}

/// Shows the noad field at `p`, marked with `c` in the indentation string.
fn print_subsidiary_data(st: &mut EngineState, p: MathWord, c: u8) {
    if st.strings.cur_length() >= st.errors.depth_threshold {
        if p.math_type != EMPTY {
            print(st, " []");
        }
        return;
    }
    st.strings.append_char(u16::from(c));
    match p.math_type {
        MATH_CHAR => {
            print_ln(st);
            print_current_string(st);
            print_fam_and_char(st, p);
        }
        SUB_BOX => show_node_list(st, p.info()),
        SUB_MLIST => {
            if p.info().is_none() {
                print_ln(st);
                print_current_string(st);
                print(st, "{}");
            } else {
                show_node_list(st, p.info());
            }
        }
        _ => {}
    }
    st.strings.pool_ptr -= 1;
}

pub(super) fn show_node_list(st: &mut EngineState, mut p: MemPtr) {
    if st.strings.cur_length() > st.errors.depth_threshold {
        if p.is_some() {
            print(st, " []");
        }
        return;
    }
    let mut n = 0;
    while let Some(here) = p {
        print_ln(st);
        print_current_string(st);
        if !st.mem.contains(p) {
            print(st, "Bad link, display aborted.");
            return;
        }
        n += 1;
        if n > st.errors.breadth_max {
            print(st, "etc.");
            return;
        }
        if st.mem.is_char_node(here) {
            print_font_and_char(st, p);
        } else {
            match st.mem.classify(here) {
                Ok(AnyNode::Char(_)) => print_font_and_char(st, p),
                Ok(AnyNode::Box(b)) => show_box_node(st, b),
                Ok(AnyNode::Unset(node)) => {
                    print_esc(st, "unset");
                    print(st, "box(");
                    print_scaled(st, st.mem[node].height);
                    print_char(st, b'+');
                    print_scaled(st, st.mem[node].depth);
                    print(st, ")x");
                    print_scaled(st, st.mem[node].width);
                    let span_count = st.mem[node].span_count;
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
                    print_int(st, i32::from(st.mem[node].box_number));
                    print(st, ", natural size ");
                    print_scaled(st, st.mem[node].height);
                    print(st, "; split(");
                    print_spec(st, st.mem[node].split_top_ptr, None);
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
                    if sub != KernSubtype::MuGlue {
                        print_esc(st, "kern");
                        if sub != KernSubtype::Normal {
                            print_char(st, b' ');
                        }
                        print_scaled(st, st.mem[node].width);
                        if sub == KernSubtype::AccKern {
                            print(st, " (for accent)");
                        } else if sub == KernSubtype::SpaceAdjustment {
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
                    if st.mem[node].side == Side::Left {
                        print(st, " (left margin)");
                    } else {
                        print(st, " (right margin)");
                    }
                }
                Ok(AnyNode::Math(node)) => match st.mem[node].subtype.get() {
                    Some(code @ (MathCode::Before | MathCode::After)) => {
                        print_esc(st, "math");
                        print(
                            st,
                            if code == MathCode::Before {
                                "on"
                            } else {
                                "off"
                            },
                        );
                        if st.mem[node].width != Scaled::ZERO {
                            print(st, ", surrounded ");
                            print_scaled(st, st.mem[node].width);
                        }
                    }
                    Some(code) => {
                        print_esc(st, if code.is_end() { "end" } else { "begin" });
                        let dir = match code.lr_end() {
                            MathCode::EndR => b'R',
                            MathCode::EndL => b'L',
                            _ => b'M',
                        };
                        print_char(st, i32::from(dir));
                    }
                    None => print(st, "Unknown math node!"),
                },
                Ok(AnyNode::Ligature(node)) => {
                    let font = st.mem[node].font;
                    print_font_char(st, font, st.mem[node].character);
                    print(st, " (ligature ");
                    if st.mem[node].boundary.has_left() {
                        print_char(st, b'|');
                    }
                    st.errors.font_in_short_display = font;
                    short_display(st, st.mem[node].lig_ptr);
                    if st.mem[node].boundary.has_right() {
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
                    let replace_count = st.mem[node].replace_count;
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
                    if st.mem[node].placement == AdjustPlacement::Pre {
                        print(st, " pre ");
                    }
                    node_list_display(st, b'.', st.mem[node].adjust_ptr);
                }
                Ok(AnyNode::Style(node)) => print_style(st, i32::from(st.mem[node].style.raw())),
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
        p = here.link(&st.mem);
    }
}

fn show_box_node(st: &mut EngineState, b: NodePtr<BoxNode>) {
    let is_hlist = st.mem[b].is_hlist();
    print_esc(st, if is_hlist { b'h' } else { b'v' });
    print(st, "box(");
    print_scaled(st, st.mem[b].height);
    print_char(st, b'+');
    print_scaled(st, st.mem[b].depth);
    print(st, ")x");
    print_scaled(st, st.mem[b].width);
    let g = st.mem[b].glue_set.get();
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
    if is_hlist && st.mem[b].lr() == BoxLr::Dlist {
        print(st, ", display");
    }
    node_list_display(st, b'.', st.mem[b].list_ptr);
}

fn show_whatsit(st: &mut EngineState, node: NodePtr<WhatsitNode>) {
    let print_font = |st: &mut EngineState, font| {
        print_esc(st, st.eqtb.font_id_text(font));
    };
    match node.variant(&st.mem) {
        Some(WhatsitVariant::Open(open)) => {
            let open = st.mem[open];
            print_write_whatsit(st, "openout", open.stream);
            print_char(st, b'=');
            print_file_name(st, open.name, open.area, open.ext);
        }
        Some(WhatsitVariant::Write(write)) => {
            let write = st.mem[write];
            print_write_whatsit(st, "write", write.stream);
            print_mark(st, write.tokens);
        }
        Some(WhatsitVariant::Close(close)) => {
            print_write_whatsit(st, "closeout", st.mem[close].stream);
        }
        Some(WhatsitVariant::Special(special)) => {
            print_esc(st, "special");
            print_mark(st, st.mem[special].tokens);
        }
        Some(WhatsitVariant::LateSpecial(special)) => {
            print_esc(st, "special");
            print(st, " shipout");
            print_mark(st, st.mem[special].tokens);
        }
        Some(WhatsitVariant::Language(language)) => {
            let language = st.mem[language];
            print_esc(st, "setlanguage");
            print_int(st, language.lang);
            print(st, " (hyphenmin ");
            print_int(st, i32::from(language.lhm));
            print_char(st, b',');
            print_int(st, i32::from(language.rhm));
            print_char(st, b')');
        }
        Some(WhatsitVariant::SavePos(_)) => print_esc(st, "pdfsavepos"),
        Some(WhatsitVariant::NativeWord(word)) => {
            print_font(st, st.mem[word].font);
            print_char(st, b' ');
            print_native_word(st, word);
        }
        Some(WhatsitVariant::Glyph(glyph)) => {
            print_font(st, st.mem[glyph].font);
            print(st, " glyph#");
            print_int(st, i32::from(st.mem[glyph].glyph));
        }
        Some(WhatsitVariant::Pic(pic)) => {
            if st.mem[pic].subtype.get() == Some(WhatsitSubtype::Pic) {
                print_esc(st, "XeTeXpicfile");
            } else {
                print_esc(st, "XeTeXpdffile");
            }
            print(st, " \"");
            for i in 0..st.mem.entries(pic).len() {
                let b = st.mem.entries(pic)[i];
                print_raw_char(st, i32::from(b), true);
            }
            print(st, b'"');
        }
        None => print(st, "whatsit?"),
    }
}

fn show_glue(st: &mut EngineState, node: NodePtr<crate::memory::node::GlueNode>) {
    let Some(kind) = st.mem[node].subtype.get() else {
        return print(st, "Unknown glue node!");
    };
    if let GlueKind::Leaders(leaders) = kind {
        print_esc(st, "");
        match leaders {
            LeaderKind::Aligned => {}
            LeaderKind::Centered => print_char(st, b'c'),
            LeaderKind::Expanded => print_char(st, b'x'),
        }
        print(st, "leaders ");
        print_spec(st, st.mem[node].glue_ptr, None);
        node_list_display(st, b'.', st.mem[node].leader_ptr);
        return;
    }
    print_esc(st, "glue");
    match kind {
        GlueKind::Normal | GlueKind::Leaders(_) => {}
        GlueKind::Param(par) => {
            print_char(st, b'(');
            print_skip_param(st, par.raw());
            print_char(st, b')');
        }
        GlueKind::CondMath => {
            print_char(st, b'(');
            print_esc(st, "nonscript");
            print_char(st, b')');
        }
        GlueKind::Mu => {
            print_char(st, b'(');
            print_esc(st, "mskip");
            print_char(st, b')');
        }
    }
    if kind != GlueKind::CondMath {
        print_char(st, b' ');
        let unit = (kind == GlueKind::Mu).then_some("mu");
        print_spec(st, st.mem[node].glue_ptr, unit);
    }
}

fn show_noad(st: &mut EngineState, any: NodePtr<AnyNoad>) {
    let n = match any.variant(&st.mem) {
        NoadVariant::Fraction(f) => {
            print_esc(st, "fraction, thickness ");
            let frac = st.mem[f];
            if frac.thickness == DEFAULT_CODE {
                print(st, "= default");
            } else {
                print_scaled(st, frac.thickness);
            }
            if !frac.left_delimiter.is_null() {
                print(st, ", left-delimiter ");
                print_delimiter(st, frac.left_delimiter);
            }
            if !frac.right_delimiter.is_null() {
                print(st, ", right-delimiter ");
                print_delimiter(st, frac.right_delimiter);
            }
            print_subsidiary_data(st, frac.numerator, b'\\');
            print_subsidiary_data(st, frac.denominator, b'/');
            return;
        }
        NoadVariant::Fence(f) => {
            let name = if st.mem[f].noad_type == NodeType::LeftNoad {
                "left"
            } else if st.mem[f].subtype == FenceSubtype::Normal {
                "right"
            } else {
                "middle"
            };
            print_esc(st, name);
            let fence = st.mem[f];
            print_delimiter(st, fence.delimiter);
            print_subsidiary_data(st, fence.supscr, b'^');
            print_subsidiary_data(st, fence.subscr, b'_');
            return;
        }
        NoadVariant::Radical(r) => {
            print_esc(st, "radical");
            print_delimiter(st, st.mem[r].left_delimiter);
            r.as_noad()
        }
        NoadVariant::Accent(a) => {
            print_esc(st, "accent");
            print_fam_and_char(st, st.mem[a].accent_chr);
            a.as_noad()
        }
        NoadVariant::Noad(n) => {
            let name = match st.mem[n].noad_type.get() {
                Some(NodeType::OrdNoad) => "mathord",
                Some(NodeType::OpNoad) => "mathop",
                Some(NodeType::BinNoad) => "mathbin",
                Some(NodeType::RelNoad) => "mathrel",
                Some(NodeType::OpenNoad) => "mathopen",
                Some(NodeType::CloseNoad) => "mathclose",
                Some(NodeType::PunctNoad) => "mathpunct",
                Some(NodeType::InnerNoad) => "mathinner",
                Some(NodeType::OverNoad) => "overline",
                Some(NodeType::UnderNoad) => "underline",
                Some(NodeType::VcenterNoad) => "vcenter",
                _ => "",
            };
            if !name.is_empty() {
                print_esc(st, name);
            }
            n
        }
    };
    // WEB shows every noad's subtype as limits, so a bottom accent shows `\nolimits`.
    let limits = st.mem[n].limits();
    if limits != LimitCode::Normal {
        let name = if limits == LimitCode::Limits {
            "limits"
        } else {
            "nolimits"
        };
        print_esc(st, name);
    }
    let noad = st.mem[n];
    print_subsidiary_data(st, noad.nucleus, b'.');
    print_subsidiary_data(st, noad.supscr, b'^');
    print_subsidiary_data(st, noad.subscr, b'_');
}
