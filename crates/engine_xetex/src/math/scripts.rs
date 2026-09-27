//! Attaching subscripts and superscripts (`make_scripts`).

use crate::consts::{
    BOX_NODE_SIZE, EMPTY, MATH_CHAR, MAX_DIMEN, NULL_FONT, SCRIPT_SIZE, SUB_MLIST,
};
use crate::errors::Abort;
use crate::node::{AnyBox, AnyNode, MathField, Noad, NodePtr};
use crate::ot_math::ScriptKind;
use crate::packaging::ADDITIONAL;
use crate::params::DimenPar;
use crate::state::{EngineState, MemState};
use crate::units::{FontId, MemPtr, NodeType, Scaled};

use super::mlist::{clean_box, fetch, set_cur_size_and_mu};
use super::{SCRIPT_SCRIPT_SIZE, SCRIPT_STYLE, TEXT_STYLE};

const SUBSCRIPTTOPMAX: i32 = 9;
const SUPERSCRIPTBOTTOMMIN: i32 = 13;
const SUBSUPERSCRIPTGAPMIN: i32 = 15;
const SUPERSCRIPTBOTTOMMAXWITHSUBSCRIPT: i32 = 16;

impl MemState {
    fn append_to_new_hlist(&mut self, q: NodePtr<Noad>, node: MemPtr) {
        let head = q.new_hlist(self);
        if head.is_null() {
            q.set_new_hlist(self, node);
        } else {
            let mut tail = head;
            while !tail.link(self).is_null() {
                tail = tail.link(self);
            }
            tail.set_link(self, node);
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
        nucleus.native_font(&st.mem),
        i32::from(nucleus.native_glyph(&st.mem)),
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
        q.new_hlist(&st.mem)
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
    box_node: AnyBox,
    shift: Scaled,
}

fn clean_script_box(
    st: &mut EngineState,
    script_field: NodePtr<MathField>,
    clean_style: i16,
    saved_font: FontId,
    adjustment: ScriptBoxAdjustment,
) -> Result<ScriptBoxPrep, Abort> {
    let node = clean_box(st, script_field, clean_style)?;
    let shift = match adjustment {
        ScriptBoxAdjustment::Sub1 => st.fonts.sub1(&st.eqtb, st.scanner.cur_size),
        ScriptBoxAdjustment::Sub2 => st.fonts.sub2(&st.eqtb, st.scanner.cur_size),
        ScriptBoxAdjustment::Superscript => {
            if st.scanner.cur_style % 2 != 0 {
                st.fonts.sup3(&st.eqtb, st.scanner.cur_size)
            } else if st.scanner.cur_style < TEXT_STYLE {
                st.fonts.sup1(&st.eqtb, st.scanner.cur_size)
            } else {
                st.fonts.sup2(&st.eqtb, st.scanner.cur_size)
            }
        }
    };
    st.scanner.cur_f = saved_font;
    let w = node.width(&st.mem) + st.eqtb.dimen_par(DimenPar::ScriptSpace);
    node.set_width(&mut st.mem, w);
    Ok(ScriptBoxPrep {
        box_node: node,
        shift,
    })
}

struct ScriptShifts {
    up: Scaled,
    down: Scaled,
}

fn initial_script_shifts(st: &mut EngineState, p: MemPtr) -> Result<ScriptShifts, Abort> {
    if st.mem.is_char_node(p) || st.mem.glyph_whatsit(p).is_some() {
        Ok(ScriptShifts {
            up: Scaled::ZERO,
            down: Scaled::ZERO,
        })
    } else {
        let zb = crate::packaging::hpack(st, p, Scaled::ZERO, ADDITIONAL)?;
        let t = if st.scanner.cur_style < SCRIPT_STYLE {
            SCRIPT_SIZE
        } else {
            SCRIPT_SCRIPT_SIZE
        };
        let shifts = ScriptShifts {
            up: zb.height(&st.mem) - st.fonts.sup_drop(&st.eqtb, t),
            down: zb.depth(&st.mem) + st.fonts.sub_drop(&st.eqtb, t),
        };
        st.mem.free_node(zb.ptr(), BOX_NODE_SIZE);
        Ok(shifts)
    }
}

struct SubSupStackInput {
    q: NodePtr<Noad>,
    p: MemPtr,
    x: AnyBox,
    y: AnyBox,
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
        let sub_empty = input.q.subscr().math_type(&st.mem) == EMPTY;
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
        (input.shift_up - input.x.depth(&st.mem)) - (input.y.height(&st.mem) - input.shift_down);
    input
        .x
        .set_shift_amount(&mut st.mem, sup_kern + input.delta - sub_kern);
    let kern = st.mem.new_kern(kern_width)?;
    input.x.set_link(&mut st.mem, kern.ptr());
    kern.set_link(&mut st.mem, input.y.ptr());

    let packaged =
        crate::packaging::vpackage(st, input.x.ptr(), Scaled::ZERO, ADDITIONAL, MAX_DIMEN)?;
    packaged.set_shift_amount(&mut st.mem, input.shift_down);
    st.mem.append_to_new_hlist(input.q, packaged.ptr());
    Ok(())
}

struct PreparedScript {
    box_node: AnyBox,
    glyph: u16,
    font: FontId,
    shift: Scaled,
}

fn prepare_subscript(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    adjustment: ScriptBoxAdjustment,
    shift_down: Scaled,
) -> Result<PreparedScript, Abort> {
    let script_field = q.subscr();
    let (glyph, font) = first_script_glyph(st, script_field)?;
    let save_f = st.scanner.cur_f;
    let clean_style = 2 * (st.scanner.cur_style / 4) + 5;
    let prepared = clean_script_box(st, script_field, clean_style, save_f, adjustment)?;
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
) -> Result<(), Abort> {
    let PreparedScript {
        box_node: x,
        glyph: sub_g,
        font: sub_f,
        shift: shift_down,
    } = prepare_subscript(st, q, ScriptBoxAdjustment::Sub1, shift_down)?;

    let cur_f = st.scanner.cur_f;
    let box_depth = x.height(&st.mem);
    let x_height = st.fonts.math_x_height(&st.eqtb, st.scanner.cur_size);
    let clr = if st.fonts.font_is_ot_math(cur_f) {
        box_depth - st.fonts.get_ot_math_constant(cur_f, SUBSCRIPTTOPMAX)
    } else {
        box_depth - (x_height * 4).abs() / 5
    };

    let shift_down = shift_down.max(clr);
    x.set_shift_amount(&mut st.mem, shift_down);

    let _ = maybe_attach_script_kern(st, q, p, sub_f, sub_g, ScriptKind::Sub, shift_down)?;
    st.mem.append_to_new_hlist(q, x.ptr());
    Ok(())
}

fn prepare_superscript(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    mut shift_up: Scaled,
) -> Result<PreparedScript, Abort> {
    let (glyph, font) = first_script_glyph(st, q.supscr())?;
    let save_f = st.scanner.cur_f;
    let clean_style = 2 * (st.scanner.cur_style / 4) + 4 + (st.scanner.cur_style % 2);
    let prepared = clean_script_box(
        st,
        q.supscr(),
        clean_style,
        save_f,
        ScriptBoxAdjustment::Superscript,
    )?;
    shift_up = shift_up.max(prepared.shift);

    let cur_f = st.scanner.cur_f;
    let box_height = prepared.box_node.depth(&st.mem);
    let x_height = st.fonts.math_x_height(&st.eqtb, st.scanner.cur_size);
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
) -> Result<(), Abort> {
    let x = superscript.box_node;
    let shift_up = superscript.shift;
    let sub_empty = q.subscr().math_type(&st.mem) == EMPTY;
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
        x.set_shift_amount(&mut st.mem, -shift_up);
        st.mem.append_to_new_hlist(q, x.ptr());
        return Ok(());
    }

    let PreparedScript {
        box_node: y,
        glyph: sub_g,
        font: sub_f,
        shift,
    } = prepare_subscript(st, q, ScriptBoxAdjustment::Sub2, shift_down)?;
    shift_down = shift;

    let cur_f = st.scanner.cur_f;
    let x_height_node = x.depth(&st.mem);
    let y_depth = y.height(&st.mem);
    let default_rule_thickness = st
        .fonts
        .default_rule_thickness(&st.eqtb, st.scanner.cur_size);
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
        let x_height_node = x.depth(&st.mem);
        let x_height = st.fonts.math_x_height(&st.eqtb, st.scanner.cur_size);
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
        script: NodePtr<MathField>,
        initial_math_style: i16,
    ) -> (Option<NodePtr<MathField>>, i16) {
        if script.math_type(self) != SUB_MLIST {
            return (Some(script), initial_math_style);
        }
        let mut script_ptr = script.info(self);
        let mut found = None;
        let mut style = initial_math_style;
        while script_ptr.0 >= 0 && script_ptr.0 <= self.mem_end {
            let node = self.classify(script_ptr);
            script_ptr = match node {
                Ok(AnyNode::Kern(_) | AnyNode::Glue(_)) => script_ptr.link(self),
                Ok(AnyNode::Style(sn)) => {
                    style = sn.subtype(self) as i16;
                    script_ptr.link(self)
                }
                Ok(AnyNode::Choice(cn)) => match style / 2 {
                    0 => cn.display_mlist(self),
                    1 => cn.text_mlist(self),
                    2 => cn.script_mlist(self),
                    3 => cn.script_script_mlist(self),
                    _ => script_ptr,
                },
                Ok(AnyNode::Noad(n))
                    if n.noad_type(self).as_raw() <= NodeType::PunctNoad.as_raw() =>
                {
                    found = Some(n.nucleus());
                    MemPtr::NULL
                }
                _ => MemPtr::NULL,
            };
        }
        (found, style)
    }
}

fn first_script_glyph(
    st: &mut EngineState,
    script_head: NodePtr<MathField>,
) -> Result<(u16, FontId), Abort> {
    let mut script_g: u16 = 0;
    let mut script_f = NULL_FONT;

    let initial_math_style = 2 * (st.scanner.cur_style / 4) + 5;
    let (script_head, this_math_style) = st
        .mem
        .first_script_glyph_step(script_head, initial_math_style);
    let math_char = script_head.filter(|h| h.math_type(&st.mem) == MATH_CHAR);

    if let Some(script_head) = math_char {
        let save_f = st.scanner.cur_f;
        let saved_math_style = st.scanner.cur_style;
        st.scanner.cur_style = this_math_style;
        set_cur_size_and_mu(st);
        fetch(st, script_head)?;

        let cur_f_after_fetch = st.scanner.cur_f;
        let cur_c_after_fetch = st.scanner.cur_c;
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
        st.scanner.cur_style = saved_math_style;
        set_cur_size_and_mu(st);
    }

    Ok((script_g, script_f))
}

pub(super) fn make_scripts(
    st: &mut EngineState,
    q: NodePtr<Noad>,
    delta: Scaled,
) -> Result<(), Abort> {
    let p = q.new_hlist(&st.mem);
    let shifts = initial_script_shifts(st, p)?;

    if q.supscr().math_type(&st.mem) == EMPTY {
        finish_subscript_only(st, q, p, shifts.down)?;
    } else {
        let superscript = prepare_superscript(st, q, shifts.up)?;
        finish_superscript_branch(st, q, p, superscript, shifts.down, delta)?;
    }
    Ok(())
}
