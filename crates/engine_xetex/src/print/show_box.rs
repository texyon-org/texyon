//! Displaying boxes: `show_box`, `short_display` and `show_node_list`.

use super::{
    print, print_char, print_current_string, print_delimiter, print_esc, print_fam_and_char,
    print_file_name, print_font_and_char, print_font_char, print_glue, print_int, print_ln,
    print_mark, print_native_word, print_raw_char, print_rule_dimen, print_scaled,
    print_skip_param, print_spec, print_style, print_write_whatsit,
};
use crate::base::codes::LimitCode;
use crate::base::consts::{EMPTY, MATH_CHAR, SUB_BOX, SUB_MLIST};
use crate::base::params::IntPar;
use crate::base::units::{GlueSign, MemPtr, NodeType, Scaled, StrNumber};
use crate::build::math::DEFAULT_CODE;
use crate::env::EngineEnv;
use crate::fonts::Side;
use crate::memory::mem::ZERO_GLUE;
use crate::memory::node::{
    AdjustPlacement, AnyNoad, AnyNode, BoxLr, BoxNode, CharNode, FenceSubtype, GlueKind, GlueNode,
    KernSubtype, LeaderKind, MathCode, MathWord, NoadVariant, NodePtr, WhatsitNode, WhatsitSubtype,
    WhatsitVariant,
};
use crate::state::EngineState;

pub(crate) fn show_box(st: &mut EngineState, env: &mut EngineEnv, p: MemPtr) {
    st.errors.depth_threshold = st.eqtb.int_par(IntPar::ShowBoxDepth);
    st.errors.breadth_max = st.eqtb.int_par(IntPar::ShowBoxBreadth);
    if st.errors.breadth_max <= 0 {
        st.errors.breadth_max = 5;
    }
    let room = st.strings.pool_size - st.strings.pool_ptr - 1;
    if st.errors.depth_threshold > room {
        st.errors.depth_threshold = room;
    }
    show_node_list(st, env, p);
    print_ln(st, env);
}

pub(crate) fn short_display(st: &mut EngineState, env: &mut EngineEnv, mut p: MemPtr) {
    while let Some(mut here) = p {
        if st.mem.is_char_node(here) {
            if st.mem.contains(p) {
                let ch = NodePtr::<CharNode>::at(here);
                let font = st.mem[ch].font;
                if font != st.errors.font_in_short_display {
                    if font > st.fonts.font_max {
                        print_char(st, env, b'*');
                    } else {
                        print_esc(st, env, st.eqtb.font_id_text(font));
                    }
                    print_char(st, env, b' ');
                    st.errors.font_in_short_display = font;
                }
                print(
                    st,
                    env,
                    StrNumber::single_char(i32::from(st.mem[ch].character)),
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
                ) => print(st, env, "[]"),
                Ok(AnyNode::Whatsit(w)) => {
                    if let Some(WhatsitVariant::NativeWord(w)) = w.variant(&st.mem) {
                        let font = st.mem[w].font;
                        if font != st.errors.font_in_short_display {
                            print_esc(st, env, st.eqtb.font_id_text(font));
                            print_char(st, env, b' ');
                            st.errors.font_in_short_display = font;
                        }
                        print_native_word(st, env, w);
                    } else {
                        print(st, env, "[]");
                    }
                }
                Ok(AnyNode::Rule(_)) => print_char(st, env, b'|'),
                Ok(AnyNode::Glue(g)) => {
                    // Glue pointing at the shared zero spec prints nothing.
                    if st.mem[g].glue_ptr != ZERO_GLUE {
                        print_char(st, env, b' ');
                    }
                }
                Ok(AnyNode::Math(m)) => {
                    if st.mem[m].subtype.get().is_some_and(MathCode::is_lr) {
                        print(st, env, "[]");
                    } else {
                        print_char(st, env, b'$');
                    }
                }
                Ok(AnyNode::Ligature(l)) => short_display(st, env, st.mem[l].lig_ptr),
                Ok(AnyNode::Disc(d)) => {
                    short_display(st, env, st.mem[d].pre_break);
                    short_display(st, env, st.mem[d].post_break);
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

fn node_list_display(st: &mut EngineState, env: &mut EngineEnv, c: u8, list: MemPtr) {
    st.strings.append_char(u16::from(c));
    show_node_list(st, env, list);
    st.strings.pool_ptr -= 1;
}

fn print_subsidiary_data(st: &mut EngineState, env: &mut EngineEnv, p: MathWord, c: u8) {
    if st.strings.cur_length() >= st.errors.depth_threshold {
        if p.math_type != EMPTY {
            print(st, env, " []");
        }
        return;
    }
    st.strings.append_char(u16::from(c));
    match p.math_type {
        MATH_CHAR => {
            print_ln(st, env);
            print_current_string(st, env);
            print_fam_and_char(st, env, p);
        }
        SUB_BOX => show_node_list(st, env, p.info()),
        SUB_MLIST => {
            if p.info().is_none() {
                print_ln(st, env);
                print_current_string(st, env);
                print(st, env, "{}");
            } else {
                show_node_list(st, env, p.info());
            }
        }
        _ => {}
    }
    st.strings.pool_ptr -= 1;
}

pub(super) fn show_node_list(st: &mut EngineState, env: &mut EngineEnv, mut p: MemPtr) {
    if st.strings.cur_length() > st.errors.depth_threshold {
        if p.is_some() {
            print(st, env, " []");
        }
        return;
    }
    let mut n = 0;
    while let Some(here) = p {
        print_ln(st, env);
        print_current_string(st, env);
        if !st.mem.contains(p) {
            print(st, env, "Bad link, display aborted.");
            return;
        }
        n += 1;
        if n > st.errors.breadth_max {
            print(st, env, "etc.");
            return;
        }
        if st.mem.is_char_node(here) {
            print_font_and_char(st, env, p);
        } else {
            match st.mem.classify(here) {
                Ok(AnyNode::Char(_)) => print_font_and_char(st, env, p),
                Ok(AnyNode::Box(b)) => show_box_node(st, env, b),
                Ok(AnyNode::Unset(node)) => {
                    print_esc(st, env, "unset");
                    print(st, env, "box(");
                    print_scaled(st, env, st.mem[node].height);
                    print_char(st, env, b'+');
                    print_scaled(st, env, st.mem[node].depth);
                    print(st, env, ")x");
                    print_scaled(st, env, st.mem[node].width);
                    let span_count = st.mem[node].span_count;
                    if span_count != 0 {
                        print(st, env, " (");
                        print_int(st, env, i32::from(span_count) + 1);
                        print(st, env, " columns)");
                    }
                    if st.mem[node].glue_stretch != Scaled::ZERO {
                        print(st, env, ", stretch ");
                        let order = st.mem[node].glue_order;
                        print_glue(st, env, st.mem[node].glue_stretch, order, None);
                    }
                    if st.mem[node].glue_shrink != Scaled::ZERO {
                        print(st, env, ", shrink ");
                        let order = st.mem[node].glue_sign;
                        print_glue(st, env, st.mem[node].glue_shrink, order, None);
                    }
                    node_list_display(st, env, b'.', st.mem[node].list_ptr);
                }
                Ok(AnyNode::Rule(node)) => {
                    print_esc(st, env, "rule(");
                    print_rule_dimen(st, env, st.mem[node].height.get());
                    print_char(st, env, b'+');
                    print_rule_dimen(st, env, st.mem[node].depth.get());
                    print(st, env, ")x");
                    print_rule_dimen(st, env, st.mem[node].width.get());
                }
                Ok(AnyNode::Ins(node)) => {
                    print_esc(st, env, "insert");
                    print_int(st, env, i32::from(st.mem[node].box_number));
                    print(st, env, ", natural size ");
                    print_scaled(st, env, st.mem[node].height);
                    print(st, env, "; split(");
                    print_spec(st, env, st.mem[node].split_top_ptr, None);
                    print_char(st, env, b',');
                    print_scaled(st, env, st.mem[node].depth);
                    print(st, env, "); float cost ");
                    print_int(st, env, st.mem[node].float_cost);
                    node_list_display(st, env, b'.', st.mem[node].ins_ptr);
                }
                Ok(AnyNode::Whatsit(node)) => show_whatsit(st, env, node),
                Ok(AnyNode::Glue(node)) => show_glue(st, env, node),
                Ok(AnyNode::Kern(node)) => {
                    let sub = st.mem[node].subtype;
                    if sub != KernSubtype::MuGlue {
                        print_esc(st, env, "kern");
                        if sub != KernSubtype::Normal {
                            print_char(st, env, b' ');
                        }
                        print_scaled(st, env, st.mem[node].width);
                        if sub == KernSubtype::AccKern {
                            print(st, env, " (for accent)");
                        } else if sub == KernSubtype::SpaceAdjustment {
                            print(st, env, " (space adjustment)");
                        }
                    } else {
                        print_esc(st, env, "mkern");
                        print_scaled(st, env, st.mem[node].width);
                        print(st, env, "mu");
                    }
                }
                Ok(AnyNode::MarginKern(node)) => {
                    print_esc(st, env, "kern");
                    print_scaled(st, env, st.mem[node].width);
                    if st.mem[node].side == Side::Left {
                        print(st, env, " (left margin)");
                    } else {
                        print(st, env, " (right margin)");
                    }
                }
                Ok(AnyNode::Math(node)) => match st.mem[node].subtype.get() {
                    Some(code @ (MathCode::Before | MathCode::After)) => {
                        print_esc(st, env, "math");
                        print(
                            st,
                            env,
                            if code == MathCode::Before {
                                "on"
                            } else {
                                "off"
                            },
                        );
                        if st.mem[node].width != Scaled::ZERO {
                            print(st, env, ", surrounded ");
                            print_scaled(st, env, st.mem[node].width);
                        }
                    }
                    Some(code) => {
                        print_esc(st, env, if code.is_end() { "end" } else { "begin" });
                        let dir = match code.lr_end() {
                            MathCode::EndR => b'R',
                            MathCode::EndL => b'L',
                            _ => b'M',
                        };
                        print_char(st, env, i32::from(dir));
                    }
                    None => print(st, env, "Unknown math node!"),
                },
                Ok(AnyNode::Ligature(node)) => {
                    let font = st.mem[node].font;
                    print_font_char(st, env, font, st.mem[node].character);
                    print(st, env, " (ligature ");
                    if st.mem[node].boundary.has_left() {
                        print_char(st, env, b'|');
                    }
                    st.errors.font_in_short_display = font;
                    short_display(st, env, st.mem[node].lig_ptr);
                    if st.mem[node].boundary.has_right() {
                        print_char(st, env, b'|');
                    }
                    print_char(st, env, b')');
                }
                Ok(AnyNode::Penalty(node)) => {
                    print_esc(st, env, "penalty ");
                    print_int(st, env, st.mem[node].penalty);
                }
                Ok(AnyNode::Disc(node)) => {
                    print_esc(st, env, "discretionary");
                    let replace_count = st.mem[node].replace_count;
                    if replace_count > 0 {
                        print(st, env, " replacing ");
                        print_int(st, env, i32::from(replace_count));
                    }
                    node_list_display(st, env, b'.', st.mem[node].pre_break);
                    node_list_display(st, env, b'|', st.mem[node].post_break);
                }
                Ok(AnyNode::Mark(node)) => {
                    print_esc(st, env, "mark");
                    let class = st.mem[node].mark_class;
                    if class != 0 {
                        print_char(st, env, b's');
                        print_int(st, env, class);
                    }
                    print_mark(st, env, st.mem[node].mark_ptr);
                }
                Ok(AnyNode::Adjust(node)) => {
                    print_esc(st, env, "vadjust");
                    if st.mem[node].placement == AdjustPlacement::Pre {
                        print(st, env, " pre ");
                    }
                    node_list_display(st, env, b'.', st.mem[node].adjust_ptr);
                }
                Ok(AnyNode::Style(node)) => {
                    print_style(st, env, i32::from(st.mem[node].style.raw()))
                }
                Ok(AnyNode::Choice(node)) => {
                    print_esc(st, env, "mathchoice");
                    node_list_display(st, env, b'D', st.mem[node].display_mlist);
                    node_list_display(st, env, b'T', st.mem[node].text_mlist);
                    node_list_display(st, env, b'S', st.mem[node].script_mlist);
                    node_list_display(st, env, b's', st.mem[node].script_script_mlist);
                }
                Ok(AnyNode::Noad(node)) => show_noad(st, env, node),
                Err(_) => print(st, env, "Unknown node type!"),
            }
        }
        p = here.link(&st.mem);
    }
}

fn show_box_node(st: &mut EngineState, env: &mut EngineEnv, b: NodePtr<BoxNode>) {
    let is_hlist = st.mem[b].is_hlist();
    print_esc(st, env, if is_hlist { b'h' } else { b'v' });
    print(st, env, "box(");
    print_scaled(st, env, st.mem[b].height);
    print_char(st, env, b'+');
    print_scaled(st, env, st.mem[b].depth);
    print(st, env, ")x");
    print_scaled(st, env, st.mem[b].width);
    let g = st.mem[b].glue_set.get();
    let glue_sign = st.mem[b].glue_sign;
    if g != 0.0 && glue_sign != GlueSign::Normal {
        print(st, env, ", glue set ");
        if glue_sign == GlueSign::Shrinking {
            print(st, env, "- ");
        }
        let order = st.mem[b].glue_order;
        if g.abs() > 20000.0 {
            if g > 0.0 {
                print_char(st, env, b'>');
            } else {
                print(st, env, "< -");
            }
            print_glue(st, env, Scaled::UNITY * 20000, order, None);
        } else {
            let set = Scaled::round(65536.0 * g);
            print_glue(st, env, set, order, None);
        }
    }
    if st.mem[b].shift_amount != Scaled::ZERO {
        print(st, env, ", shifted ");
        print_scaled(st, env, st.mem[b].shift_amount);
    }
    if is_hlist && st.mem[b].lr() == BoxLr::Dlist {
        print(st, env, ", display");
    }
    node_list_display(st, env, b'.', st.mem[b].list_ptr);
}

fn show_whatsit(st: &mut EngineState, env: &mut EngineEnv, node: NodePtr<WhatsitNode>) {
    let print_font = |st: &mut EngineState, env: &mut EngineEnv, font| {
        print_esc(st, env, st.eqtb.font_id_text(font));
    };
    match node.variant(&st.mem) {
        Some(WhatsitVariant::Open(open)) => {
            let open = st.mem[open];
            print_write_whatsit(st, env, "openout", open.stream);
            print_char(st, env, b'=');
            print_file_name(st, env, open.name, open.area, open.ext);
        }
        Some(WhatsitVariant::Write(write)) => {
            let write = st.mem[write];
            print_write_whatsit(st, env, "write", write.stream);
            print_mark(st, env, write.tokens);
        }
        Some(WhatsitVariant::Close(close)) => {
            print_write_whatsit(st, env, "closeout", st.mem[close].stream);
        }
        Some(WhatsitVariant::Special(special)) => {
            print_esc(st, env, "special");
            print_mark(st, env, st.mem[special].tokens);
        }
        Some(WhatsitVariant::LateSpecial(special)) => {
            print_esc(st, env, "special");
            print(st, env, " shipout");
            print_mark(st, env, st.mem[special].tokens);
        }
        Some(WhatsitVariant::Language(language)) => {
            let language = st.mem[language];
            print_esc(st, env, "setlanguage");
            print_int(st, env, language.lang);
            print(st, env, " (hyphenmin ");
            print_int(st, env, i32::from(language.lhm));
            print_char(st, env, b',');
            print_int(st, env, i32::from(language.rhm));
            print_char(st, env, b')');
        }
        Some(WhatsitVariant::SavePos(_)) => print_esc(st, env, "pdfsavepos"),
        Some(WhatsitVariant::NativeWord(word)) => {
            print_font(st, env, st.mem[word].font);
            print_char(st, env, b' ');
            print_native_word(st, env, word);
        }
        Some(WhatsitVariant::Glyph(glyph)) => {
            print_font(st, env, st.mem[glyph].font);
            print(st, env, " glyph#");
            print_int(st, env, i32::from(st.mem[glyph].glyph));
        }
        Some(WhatsitVariant::Pic(pic)) => {
            if st.mem[pic].subtype.get() == Some(WhatsitSubtype::Pic) {
                print_esc(st, env, "XeTeXpicfile");
            } else {
                print_esc(st, env, "XeTeXpdffile");
            }
            print(st, env, " \"");
            for i in 0..st.mem.entries(pic).len() {
                let b = st.mem.entries(pic)[i];
                print_raw_char(st, env, i32::from(b), true);
            }
            print(st, env, b'"');
        }
        None => print(st, env, "whatsit?"),
    }
}

fn show_glue(st: &mut EngineState, env: &mut EngineEnv, node: NodePtr<GlueNode>) {
    let Some(kind) = st.mem[node].subtype.get() else {
        return print(st, env, "Unknown glue node!");
    };
    if let GlueKind::Leaders(leaders) = kind {
        print_esc(st, env, "");
        match leaders {
            LeaderKind::Aligned => {}
            LeaderKind::Centered => print_char(st, env, b'c'),
            LeaderKind::Expanded => print_char(st, env, b'x'),
        }
        print(st, env, "leaders ");
        print_spec(st, env, st.mem[node].glue_ptr, None);
        node_list_display(st, env, b'.', st.mem[node].leader_ptr);
        return;
    }
    print_esc(st, env, "glue");
    match kind {
        GlueKind::Normal | GlueKind::Leaders(_) => {}
        GlueKind::Param(par) => {
            print_char(st, env, b'(');
            print_skip_param(st, env, par.raw());
            print_char(st, env, b')');
        }
        GlueKind::CondMath => {
            print_char(st, env, b'(');
            print_esc(st, env, "nonscript");
            print_char(st, env, b')');
        }
        GlueKind::Mu => {
            print_char(st, env, b'(');
            print_esc(st, env, "mskip");
            print_char(st, env, b')');
        }
    }
    if kind != GlueKind::CondMath {
        print_char(st, env, b' ');
        let unit = (kind == GlueKind::Mu).then_some("mu");
        print_spec(st, env, st.mem[node].glue_ptr, unit);
    }
}

fn show_noad(st: &mut EngineState, env: &mut EngineEnv, any: NodePtr<AnyNoad>) {
    let n = match any.variant(&st.mem) {
        NoadVariant::Fraction(f) => {
            print_esc(st, env, "fraction, thickness ");
            let frac = st.mem[f];
            if frac.thickness == DEFAULT_CODE {
                print(st, env, "= default");
            } else {
                print_scaled(st, env, frac.thickness);
            }
            if !frac.left_delimiter.is_null() {
                print(st, env, ", left-delimiter ");
                print_delimiter(st, env, frac.left_delimiter);
            }
            if !frac.right_delimiter.is_null() {
                print(st, env, ", right-delimiter ");
                print_delimiter(st, env, frac.right_delimiter);
            }
            print_subsidiary_data(st, env, frac.numerator, b'\\');
            print_subsidiary_data(st, env, frac.denominator, b'/');
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
            print_esc(st, env, name);
            let fence = st.mem[f];
            print_delimiter(st, env, fence.delimiter);
            print_subsidiary_data(st, env, fence.supscr, b'^');
            print_subsidiary_data(st, env, fence.subscr, b'_');
            return;
        }
        NoadVariant::Radical(r) => {
            print_esc(st, env, "radical");
            print_delimiter(st, env, st.mem[r].left_delimiter);
            r.as_noad()
        }
        NoadVariant::Accent(a) => {
            print_esc(st, env, "accent");
            print_fam_and_char(st, env, st.mem[a].accent_chr);
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
                print_esc(st, env, name);
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
        print_esc(st, env, name);
    }
    let noad = st.mem[n];
    print_subsidiary_data(st, env, noad.nucleus, b'.');
    print_subsidiary_data(st, env, noad.supscr, b'^');
    print_subsidiary_data(st, env, noad.subscr, b'_');
}
