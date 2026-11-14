//! Attaching subscripts and superscripts (`make_scripts`).

use super::mlist::{clean_box, fetch};
use super::{MathStyle, SCRIPT_SCRIPT_SIZE};
use crate::base::consts::{EMPTY, MATH_CHAR, MAX_DIMEN, NULL_FONT, SCRIPT_SIZE, SUB_MLIST};
use crate::base::params::DimenPar;
use crate::base::units::{FontId, MemPtr, NodeType, Scaled};
use crate::build::packaging::PackSpec;
use crate::fonts::ot_math::ScriptKind;
use crate::memory::mem::MemState;
use crate::memory::node::{AnyNode, BoxNode, MathSlot, Noad, NoadVariant, NodePtr};
use crate::print::errors::Abort;
use crate::state::EngineState;

const SUBSCRIPTTOPMAX: i32 = 9;
const SUPERSCRIPTBOTTOMMIN: i32 = 13;
const SUBSUPERSCRIPTGAPMIN: i32 = 15;
const SUPERSCRIPTBOTTOMMAXWITHSUBSCRIPT: i32 = 16;

impl MemState {
    fn append_to_new_hlist(&mut self, q: NodePtr<Noad>, node: MemPtr) {
        match self[q.any()].new_hlist {
            None => self[q.any()].new_hlist = node,
            Some(head) => self.list_tail(head).set_link(self, node),
        }
    }
}

struct ScriptKernAttachment {
    p: MemPtr,
    kern: Scaled,
    is_ot_math: bool,
}

fn maybe_attach_script_kern(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    p: MemPtr,
    script_f: FontId,
    script_g: u16,
    kind: ScriptKind,
    shift: Scaled,
) -> Result<ScriptKernAttachment, Abort> {
    let cur_f = st.scanner.cur_f;
    let nucleus = st.mem.glyph_whatsit(p);

    if !st.fonts.font_is_ot_math(cur_f) {
        return Ok(ScriptKernAttachment {
            p,
            kern: Scaled::ZERO,
            is_ot_math: false,
        });
    }

    let Some(nucleus) = nucleus else {
        return Ok(ScriptKernAttachment {
            p,
            kern: Scaled::ZERO,
            is_ot_math: true,
        });
    };

    let kern = st.fonts.get_ot_math_kern(
        st.mem[nucleus].font,
        i32::from(st.mem[nucleus].glyph),
        script_f,
        i32::from(script_g),
        kind,
        shift,
    );
    let p = if kern == Scaled::ZERO {
        p
    } else {
        let k = st.mem.new_kern(kern)?;
        st.mem.append_to_new_hlist(q, k.ptr());
        st.mem[q.any()].new_hlist
    };
    Ok(ScriptKernAttachment {
        p,
        kern,
        is_ot_math: true,
    })
}

#[derive(Clone, Copy)]
enum ScriptBoxAdjustment {
    Sub1,
    Sub2,
    Superscript,
}

struct ScriptBoxPrep {
    box_node: NodePtr<BoxNode>,
    shift: Scaled,
}

fn clean_script_box(
    st: &mut EngineState,
    script_field: MathSlot,
    clean_style: MathStyle,
    saved_font: FontId,
    adjustment: ScriptBoxAdjustment,
    style: MathStyle,
) -> Result<ScriptBoxPrep, Abort> {
    let node = clean_box(st, st.mem[script_field], clean_style)?;
    let shift = match adjustment {
        ScriptBoxAdjustment::Sub1 => st.fonts.sub1(&st.eqtb, style.size()),
        ScriptBoxAdjustment::Sub2 => st.fonts.sub2(&st.eqtb, style.size()),
        ScriptBoxAdjustment::Superscript => {
            if style.is_cramped() {
                st.fonts.sup3(&st.eqtb, style.size())
            } else if style < MathStyle::TEXT {
                st.fonts.sup1(&st.eqtb, style.size())
            } else {
                st.fonts.sup2(&st.eqtb, style.size())
            }
        }
    };
    st.scanner.cur_f = saved_font;
    let w = st.mem[node].width + st.eqtb.dimen_par(DimenPar::ScriptSpace);
    st.mem[node].width = w;
    Ok(ScriptBoxPrep {
        box_node: node,
        shift,
    })
}

struct ScriptShifts {
    up: Scaled,
    down: Scaled,
}

fn initial_script_shifts(
    st: &mut EngineState,
    p: MemPtr,
    style: MathStyle,
) -> Result<ScriptShifts, Abort> {
    if st.mem.as_char(p).is_some() || st.mem.glyph_whatsit(p).is_some() {
        Ok(ScriptShifts {
            up: Scaled::ZERO,
            down: Scaled::ZERO,
        })
    } else {
        let zb = crate::build::packaging::hpack(st, p, PackSpec::NATURAL)?;
        let t = if style < MathStyle::SCRIPT {
            SCRIPT_SIZE
        } else {
            SCRIPT_SCRIPT_SIZE
        };
        let shifts = ScriptShifts {
            up: st.mem[zb].height - st.fonts.sup_drop(&st.eqtb, t),
            down: st.mem[zb].depth + st.fonts.sub_drop(&st.eqtb, t),
        };
        zb.free(&mut st.mem);
        Ok(shifts)
    }
}

struct SubSupStackInput {
    q: NodePtr<Noad>,
    p: MemPtr,
    x: NodePtr<BoxNode>,
    y: NodePtr<BoxNode>,
    sub_f: FontId,
    sub_g: u16,
    sup_f: FontId,
    sup_g: u16,
    shift_down: Scaled,
    shift_up: Scaled,
    delta: Scaled,
}

fn finish_subsup_stack(st: &mut EngineState, input: SubSupStackInput) -> Result<(), Abort> {
    let mut p = input.p;
    let mut sub_kern;
    let mut sup_kern = Scaled::ZERO;
    let attachment = maybe_attach_script_kern(
        st,
        input.q,
        p,
        input.sub_f,
        input.sub_g,
        ScriptKind::Sub,
        input.shift_down,
    )?;
    p = attachment.p;
    sub_kern = attachment.kern;
    if attachment.is_ot_math {
        let sub_empty = st.mem[input.q].subscr.math_type == EMPTY;
        if sub_empty {
            let attachment = maybe_attach_script_kern(
                st,
                input.q,
                p,
                input.sup_f,
                input.sup_g,
                ScriptKind::Sup,
                input.shift_up,
            )?;
            sup_kern = attachment.kern;
        }
    } else {
        sup_kern = Scaled::ZERO;
        sub_kern = Scaled::ZERO;
    }

    let kern_width =
        (input.shift_up - st.mem[input.x].depth) - (st.mem[input.y].height - input.shift_down);
    st.mem[input.x].shift_amount = sup_kern + input.delta - sub_kern;
    let kern = st.mem.new_kern(kern_width)?;
    st.mem[input.x].link = kern.ptr();
    st.mem[kern].link = input.y.ptr();

    let packaged =
        crate::build::packaging::vpackage(st, input.x.ptr(), PackSpec::NATURAL, MAX_DIMEN)?;
    st.mem[packaged].shift_amount = input.shift_down;
    st.mem.append_to_new_hlist(input.q, packaged.ptr());
    Ok(())
}

struct PreparedScript {
    box_node: NodePtr<BoxNode>,
    glyph: u16,
    font: FontId,
    shift: Scaled,
}

fn prepare_subscript(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    adjustment: ScriptBoxAdjustment,
    shift_down: Scaled,
    style: MathStyle,
) -> Result<PreparedScript, Abort> {
    let script_field = MathSlot::Subscr(q);
    let (glyph, font) = first_script_glyph(st, script_field, style)?;
    let save_f = st.scanner.cur_f;
    let clean_style = style.sub();
    let prepared = clean_script_box(st, script_field, clean_style, save_f, adjustment, style)?;
    Ok(PreparedScript {
        box_node: prepared.box_node,
        glyph,
        font,
        shift: shift_down.max(prepared.shift),
    })
}

fn finish_subscript_only(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    p: MemPtr,
    shift_down: Scaled,
    style: MathStyle,
) -> Result<(), Abort> {
    let PreparedScript {
        box_node: x,
        glyph: sub_g,
        font: sub_f,
        shift: shift_down,
    } = prepare_subscript(st, q, ScriptBoxAdjustment::Sub1, shift_down, style)?;

    let cur_f = st.scanner.cur_f;
    let box_depth = st.mem[x].height;
    let x_height = st.fonts.math_x_height(&st.eqtb, style.size());
    let clr = if st.fonts.font_is_ot_math(cur_f) {
        box_depth - st.fonts.get_ot_math_constant(cur_f, SUBSCRIPTTOPMAX)
    } else {
        box_depth - (x_height * 4).abs() / 5
    };

    let shift_down = shift_down.max(clr);
    st.mem[x].shift_amount = shift_down;

    let _ = maybe_attach_script_kern(st, q, p, sub_f, sub_g, ScriptKind::Sub, shift_down)?;
    st.mem.append_to_new_hlist(q, x.ptr());
    Ok(())
}

fn prepare_superscript(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    mut shift_up: Scaled,
    style: MathStyle,
) -> Result<PreparedScript, Abort> {
    let (glyph, font) = first_script_glyph(st, MathSlot::Supscr(q), style)?;
    let save_f = st.scanner.cur_f;
    let clean_style = style.sup();
    let prepared = clean_script_box(
        st,
        MathSlot::Supscr(q),
        clean_style,
        save_f,
        ScriptBoxAdjustment::Superscript,
        style,
    )?;
    shift_up = shift_up.max(prepared.shift);

    let cur_f = st.scanner.cur_f;
    let box_height = st.mem[prepared.box_node].depth;
    let x_height = st.fonts.math_x_height(&st.eqtb, style.size());
    let clr = if st.fonts.font_is_ot_math(cur_f) {
        box_height + st.fonts.get_ot_math_constant(cur_f, SUPERSCRIPTBOTTOMMIN)
    } else {
        box_height + x_height.abs() / 4
    };

    Ok(PreparedScript {
        box_node: prepared.box_node,
        glyph,
        font,
        shift: shift_up.max(clr),
    })
}

fn finish_superscript_branch(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    p: MemPtr,
    superscript: PreparedScript,
    mut shift_down: Scaled,
    delta: Scaled,
    style: MathStyle,
) -> Result<(), Abort> {
    let x = superscript.box_node;
    let shift_up = superscript.shift;
    let sub_empty = st.mem[q].subscr.math_type == EMPTY;
    if sub_empty {
        let _ = maybe_attach_script_kern(
            st,
            q,
            p,
            superscript.font,
            superscript.glyph,
            ScriptKind::Sup,
            shift_up,
        )?;
        st.mem[x].shift_amount = -shift_up;
        st.mem.append_to_new_hlist(q, x.ptr());
        return Ok(());
    }

    let PreparedScript {
        box_node: y,
        glyph: sub_g,
        font: sub_f,
        shift,
    } = prepare_subscript(st, q, ScriptBoxAdjustment::Sub2, shift_down, style)?;
    shift_down = shift;

    let cur_f = st.scanner.cur_f;
    let x_height_node = st.mem[x].depth;
    let y_depth = st.mem[y].height;
    let default_rule_thickness = st.fonts.default_rule_thickness(&st.eqtb, style.size());
    let mut clr = if st.fonts.font_is_ot_math(cur_f) {
        st.fonts.get_ot_math_constant(cur_f, SUBSUPERSCRIPTGAPMIN)
            - ((shift_up - x_height_node) - (y_depth - shift_down))
    } else {
        default_rule_thickness * 4 - ((shift_up - x_height_node) - (y_depth - shift_down))
    };

    let mut shift_up = shift_up;
    if clr > Scaled::ZERO {
        shift_down += clr;
        let cur_f = st.scanner.cur_f;
        let x_height_node = st.mem[x].depth;
        let x_height = st.fonts.math_x_height(&st.eqtb, style.size());
        clr = if st.fonts.font_is_ot_math(cur_f) {
            st.fonts
                .get_ot_math_constant(cur_f, SUPERSCRIPTBOTTOMMAXWITHSUBSCRIPT)
                - (shift_up - x_height_node)
        } else {
            (x_height * 4).abs() / 5 - (shift_up - x_height_node)
        };
        if clr > Scaled::ZERO {
            shift_up += clr;
            shift_down -= clr;
        }
    }

    finish_subsup_stack(
        st,
        SubSupStackInput {
            q,
            p,
            x,
            y,
            sub_f,
            sub_g,
            sup_f: superscript.font,
            sup_g: superscript.glyph,
            shift_down,
            shift_up,
            delta,
        },
    )?;
    Ok(())
}

impl MemState {
    /// Finds the first noad of a sub-mlist script field to probe for an OpenType
    /// glyph, tracking style changes on the way. Returns the field and its style.
    fn first_script_glyph_step(
        &self,
        script: MathSlot,
        initial_math_style: MathStyle,
    ) -> (Option<MathSlot>, MathStyle) {
        if self[script].math_type != SUB_MLIST {
            return (Some(script), initial_math_style);
        }
        let mut script_ptr = self[script].info();
        let mut found = None;
        let mut style = initial_math_style;
        while let Some(here) = script_ptr.filter(|&p| self.contains(Some(p))) {
            let node = self.classify(here);
            script_ptr = match node {
                Ok(AnyNode::Kern(_) | AnyNode::Glue(_)) => here.link(self),
                Ok(AnyNode::Style(sn)) => {
                    style = self[sn].style;
                    here.link(self)
                }
                Ok(AnyNode::Choice(cn)) => match style.level() {
                    0 => self[cn].display_mlist,
                    1 => self[cn].text_mlist,
                    2 => self[cn].script_mlist,
                    3 => self[cn].script_script_mlist,
                    _ => script_ptr,
                },
                Ok(AnyNode::Noad(any)) => {
                    if let NoadVariant::Noad(n) = any.variant(self)
                        && self[n].noad_type <= NodeType::PunctNoad
                    {
                        found = Some(MathSlot::Nucleus(n));
                    }
                    None
                }
                _ => None,
            };
        }
        (found, style)
    }
}

fn first_script_glyph(
    st: &mut EngineState,
    script_head: MathSlot,
    style: MathStyle,
) -> Result<(u16, FontId), Abort> {
    let mut script_g: u16 = 0;
    let mut script_f = NULL_FONT;

    let initial_math_style = style.sub();
    let (script_head, this_math_style) = st
        .mem
        .first_script_glyph_step(script_head, initial_math_style);
    let math_char = script_head.filter(|&h| st.mem[h].math_type == MATH_CHAR);

    if let Some(script_head) = math_char {
        let save_f = st.scanner.cur_f;
        let fetched = fetch(st, script_head, this_math_style.size())?;

        let cur_f_after_fetch = fetched.f;
        let cur_c_after_fetch = fetched.c;
        if st.fonts.font_is_ot_math(cur_f_after_fetch) {
            let script_c = crate::fonts::new_native_character(
                st,
                cur_f_after_fetch,
                cur_c_after_fetch as u32,
            )?;
            script_g = st.mem.get_native_glyph(script_c, 0);
            script_f = cur_f_after_fetch;
        }

        st.scanner.cur_f = save_f;
    }

    Ok((script_g, script_f))
}

pub(super) fn make_scripts(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    delta: Scaled,
    style: MathStyle,
) -> Result<(), Abort> {
    let p = st.mem[q.any()].new_hlist;
    let shifts = initial_script_shifts(st, p, style)?;

    if st.mem[q].supscr.math_type == EMPTY {
        finish_subscript_only(st, q, p, shifts.down, style)?;
    } else {
        let superscript = prepare_superscript(st, q, shifts.up, style)?;
        finish_superscript_branch(st, q, p, superscript, shifts.down, delta, style)?;
    }
    Ok(())
}
