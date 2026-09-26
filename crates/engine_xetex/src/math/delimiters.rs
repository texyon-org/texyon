//! Boxes for math characters and delimiters: `char_box`, `var_delimiter`, `rebox`.

use crate::consts::{BOX_NODE_SIZE, GLYPH_NODE_SIZE, NORMAL, NULL_FONT, SCRIPT_SIZE};
use crate::errors::Abort;
use crate::fonts::{CharInfo, CharTag, AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::mem::SS_GLUE;
use crate::node::{AnyBox, AnyNode, DelimiterField, NodePtr, WhatsitNode, WhatsitSubtype};
use crate::ot_math::GlyphAssembly;
use crate::packaging::{ADDITIONAL, EXACTLY};
use crate::params::DimenPar;
use crate::state::{EngineState, FontState, MemState};
use crate::units::{FontId, GlueSign, MemIdx, Scaled};

impl FontState {
    pub(super) fn measure_glyph(
        &self,
        mem: &mut MemState,
        p: NodePtr<WhatsitNode>,
    ) -> Result<(), Abort> {
        self.measure_native_glyph(mem, p, true)
    }
}

impl MemState {
    pub(super) fn native_char_box(&mut self, p: NodePtr<WhatsitNode>) -> Result<AnyBox, Abort> {
        let b = self.new_null_box()?;
        b.set_height(self, p.height(self));
        b.set_width(self, p.width(self));
        b.set_depth(self, p.depth(self).max(Scaled::ZERO));
        b.set_list_ptr(self, p.ptr());
        Ok(AnyBox::HList(b))
    }
}

fn char_box(st: &mut EngineState, f: FontId, c: i32) -> Result<AnyBox, Abort> {
    let area = st.fonts.font_area[f];
    if area == AAT_FONT_FLAG || area == OTGR_FONT_FLAG {
        let p = crate::fonts::new_native_character(st, f, c as u32)?;
        Ok(st.mem.native_char_box(p)?)
    } else {
        Ok(st.mem.char_box_tfm(&mut st.fonts, f, c as u16)?)
    }
}

impl MemState {
    pub(super) fn char_box_tfm(
        &mut self,
        fonts: &mut FontState,
        f: FontId,
        c: u16,
    ) -> Result<AnyBox, Abort> {
        let c_eff = fonts.effective_char(f, c);
        let q = fonts.char_info(f, c_eff);
        let width = fonts.char_width(f, q) + fonts.char_italic(f, q);
        let height = fonts.char_height(f, q);
        let depth = fonts.char_depth(f, q);

        let b = self.new_null_box()?;
        b.set_width(self, width);
        b.set_height(self, height);
        b.set_depth(self, depth);
        let p = self.new_char(f, c)?;
        b.set_list_ptr(self, p.ptr());
        Ok(AnyBox::HList(b))
    }

    fn stack_into_box(&mut self, b_box: AnyBox, p: AnyBox) {
        let link = b_box.list_ptr(self);
        p.set_link(self, link);
        b_box.set_list_ptr(self, p.ptr());
        let h = p.height(self);
        b_box.set_height(self, h);
    }
}

impl FontState {
    fn height_plus_depth(&mut self, f: FontId, c: u16) -> Scaled {
        let c = self.effective_char(f, c);
        let q = self.char_info(f, c);
        self.char_height(f, q) + self.char_depth(f, q)
    }
}

/// A measured glyph whatsit for glyph `g` of font `f`.
fn new_measured_glyph(
    st: &mut EngineState,
    f: FontId,
    g: i32,
) -> Result<NodePtr<WhatsitNode>, Abort> {
    let p = NodePtr::<WhatsitNode>::alloc(&mut st.mem, GLYPH_NODE_SIZE, WhatsitSubtype::Glyph)?;
    p.set_native_font(&mut st.mem, f);
    p.set_native_glyph(&mut st.mem, g as u16);
    st.fonts.measure_glyph(&mut st.mem, p)?;
    Ok(p)
}

fn stack_glyph_into_box(st: &mut EngineState, b: AnyBox, f: FontId, g: i32) -> Result<(), Abort> {
    let p = new_measured_glyph(st, f, g)?;
    let mem = &mut st.mem;
    if matches!(b, AnyBox::HList(_)) {
        let mut q = b.list_ptr(mem);
        if q.is_null() {
            b.set_list_ptr(mem, p.ptr());
        } else {
            while !q.link(mem).is_null() {
                q = q.link(mem);
            }
            q.set_link(mem, p.ptr());
            if b.height(mem) < p.height(mem) {
                b.set_height(mem, p.height(mem));
            }
            if b.depth(mem) < p.depth(mem) {
                b.set_depth(mem, p.depth(mem));
            }
        }
    } else {
        p.set_link(mem, b.list_ptr(mem));
        b.set_list_ptr(mem, p.ptr());
        b.set_height(mem, p.height(mem));
        if b.width(mem) < p.width(mem) {
            b.set_width(mem, p.width(mem));
        }
    }
    Ok(())
}

fn native_glyph_vbox(st: &mut EngineState, f: FontId, g: u16) -> Result<AnyBox, Abort> {
    let b = AnyBox::VList(st.mem.new_null_box()?.into_vlist(&mut st.mem));
    let p = new_measured_glyph(st, f, i32::from(g))?;
    let mem = &mut st.mem;
    b.set_list_ptr(mem, p.ptr());
    b.set_width(mem, p.width(mem));
    b.set_height(mem, p.height(mem));
    b.set_depth(mem, p.depth(mem));
    Ok(b)
}

pub(super) fn rebox(st: &mut EngineState, b: AnyBox, w: Scaled) -> Result<AnyBox, Abort> {
    let mut b = b;
    if b.width(&st.mem) != w && !b.list_ptr(&st.mem).is_null() && matches!(b, AnyBox::VList(_)) {
        b = crate::packaging::hpack(st, b.ptr(), Scaled::ZERO, ADDITIONAL)?;
    }
    if b.width(&st.mem) == w || b.list_ptr(&st.mem).is_null() {
        b.set_width(&mut st.mem, w);
        return Ok(b);
    }

    let mut p = b.list_ptr(&st.mem);
    if let Some(ch) = st.mem.as_char(p).filter(|_| p.link(&st.mem).is_null()) {
        let f = ch.font(&st.mem);
        let c = st.fonts.effective_char(f, ch.character(&st.mem));
        let q = st.fonts.char_info(f, c);
        let v = st.fonts.char_width(f, q);
        let bw = b.width(&st.mem);
        if v != bw {
            let k = st.mem.new_kern(bw - v)?;
            p.set_link(&mut st.mem, k.ptr());
        }
    }
    let mem = &mut st.mem;
    mem.free_node(b.ptr(), BOX_NODE_SIZE);
    let g = mem.new_glue(SS_GLUE)?;
    g.set_link(mem, p);
    while !p.link(mem).is_null() {
        p = p.link(mem);
    }
    let tail_glue = mem.new_glue(SS_GLUE)?;
    p.set_link(mem, tail_glue.ptr());
    crate::packaging::hpack(st, g.ptr(), w, EXACTLY)
}

pub(super) fn build_opentype_assembly(
    st: &mut EngineState,
    f: FontId,
    a: &GlyphAssembly,
    s: Scaled,
    horiz: bool,
) -> Result<AnyBox, Abort> {
    let b = if horiz {
        AnyBox::HList(st.mem.new_null_box()?)
    } else {
        AnyBox::VList(st.mem.new_null_box()?.into_vlist(&mut st.mem))
    };
    // Extenders repeat `n` times, other parts once.
    let repeats = |i: i32, n: i32| if a.is_extender(i) { n } else { 1 };
    let no_extenders = (0..a.count()).all(|i| !a.is_extender(i));
    let min_o = st.fonts.ot_min_connector_overlap(f);
    let mut n: i32 = -1;
    loop {
        n += 1;
        let mut s_max = Scaled::ZERO;
        let mut prev_o = Scaled::ZERO;
        for i in 0..a.count() {
            for _ in 0..repeats(i, n) {
                let o = a.start_connector(&st.fonts, f, i).min(min_o).min(prev_o);
                s_max = s_max - o + a.full_advance(&st.fonts, f, i);
                prev_o = a.end_connector(&st.fonts, f, i);
            }
        }
        if s_max >= s || no_extenders {
            break;
        }
    }
    let mut prev_o = Scaled::ZERO;
    for i in 0..a.count() {
        for _ in 0..repeats(i, n) {
            let oo = a.start_connector(&st.fonts, f, i).min(prev_o);
            if oo > Scaled::ZERO {
                st.mem.stack_glue_into_box(b, -oo, -oo.min(min_o))?;
            }
            stack_glyph_into_box(st, b, f, a.glyph(i))?;
            prev_o = a.end_connector(&st.fonts, f, i);
        }
    }
    let mut p = b.list_ptr(&st.mem);
    let mut nat = Scaled::ZERO;
    let mut str = Scaled::ZERO;
    while !p.is_null() {
        match st.mem.classify(MemIdx::new(p)) {
            Ok(AnyNode::Whatsit(pw)) => {
                if horiz {
                    nat += pw.width(&st.mem);
                } else {
                    nat += pw.height(&st.mem) + pw.depth(&st.mem);
                }
            }
            Ok(AnyNode::Glue(gn)) => {
                let gs = gn.glue_ptr(&st.mem);
                nat += gs.width(&st.mem);
                str += gs.stretch(&st.mem);
            }
            _ => {}
        }
        p = p.link(&st.mem);
    }
    if s > nat && str > Scaled::ZERO {
        let o = (s - nat).min(str);
        b.set_glue_order(&mut st.mem, NORMAL);
        b.set_glue_sign(&mut st.mem, GlueSign::Stretching);
        let ratio = f64::from(o) / f64::from(str);
        b.set_glue_set(&mut st.mem, ratio);
        let set_dim = nat + Scaled(crate::scaled_math::tex_round(f64::from(str) * ratio));
        if horiz {
            b.set_width(&mut st.mem, set_dim);
        } else {
            b.set_height(&mut st.mem, set_dim);
        }
    } else if horiz {
        b.set_width(&mut st.mem, nat);
    } else {
        b.set_height(&mut st.mem, nat);
    }
    Ok(b)
}

impl MemState {
    fn build_ext_delimiter_box(
        &mut self,
        fonts: &mut FontState,
        f: FontId,
        q: CharInfo,
        v: Scaled,
    ) -> Result<AnyBox, Abort> {
        let vb = AnyBox::VList(self.new_null_box()?.into_vlist(self));
        let r = fonts.ext_recipe(f, q);
        let u = fonts.height_plus_depth(f, r.rep);
        let effective_c = fonts.effective_char(f, r.rep);
        let qq = fonts.char_info(f, effective_c);
        let box_width = fonts.char_width(f, qq) + fonts.char_italic(f, qq);
        vb.set_width(self, box_width);
        let mut w = Scaled::ZERO;
        if r.bot != 0 {
            w += fonts.height_plus_depth(f, r.bot);
        }
        if r.mid != 0 {
            w += fonts.height_plus_depth(f, r.mid);
        }
        if r.top != 0 {
            w += fonts.height_plus_depth(f, r.top);
        }

        let mut n: i32 = 0;
        if u > Scaled::ZERO {
            while w < v {
                w += u;
                n += 1;
                if r.mid != 0 {
                    w += u;
                }
            }
        }
        if r.bot != 0 {
            let p = self.char_box_tfm(fonts, f, r.bot)?;
            self.stack_into_box(vb, p);
        }
        for _ in 1..=n {
            let p = self.char_box_tfm(fonts, f, r.rep)?;
            self.stack_into_box(vb, p);
        }
        if r.mid != 0 {
            let p = self.char_box_tfm(fonts, f, r.mid)?;
            self.stack_into_box(vb, p);
            for _ in 1..=n {
                let p = self.char_box_tfm(fonts, f, r.rep)?;
                self.stack_into_box(vb, p);
            }
        }
        if r.top != 0 {
            let p = self.char_box_tfm(fonts, f, r.top)?;
            self.stack_into_box(vb, p);
        }
        let height = vb.height(self);
        vb.set_depth(self, w - height);
        Ok(vb)
    }
}

pub(super) fn var_delimiter(
    st: &mut EngineState,
    d: NodePtr<DelimiterField>,
    s: i32,
    v: Scaled,
) -> Result<AnyBox, Abort> {
    let mut f = NULL_FONT;
    let mut ot_assembly: Option<GlyphAssembly> = None;
    let mut c: u16 = 0;
    let mut w = Scaled::ZERO;
    let mut q = CharInfo::NULL;
    let mut large_attempt = false;
    let (mut z, mut x) = d.small(&st.mem);
    'found: {
        loop {
            if z != 0 || x != 0 {
                z = z + s + 256;
                loop {
                    z -= 256;
                    let g = st.eqtb.math_font(z);
                    if g != NULL_FONT {
                        if st.fonts.font_area[g] == AAT_FONT_FLAG || st.fonts.font_uses_opentype(g)
                        {
                            x = st.fonts.map_char_to_glyph(g, x)?;
                            f = g;
                            c = x as u16;
                            w = Scaled::ZERO;
                            let mut n: i32 = 0;
                            let mut u = Scaled::ZERO;
                            loop {
                                let yv =
                                    st.fonts.get_ot_math_variant(g, x, n, &mut u, false) as u16;
                                if u > w {
                                    c = yv;
                                    w = u;
                                    if u >= v {
                                        break 'found;
                                    }
                                }
                                n += 1;
                                if u < Scaled::ZERO {
                                    break;
                                }
                            }
                            ot_assembly = st.fonts.get_ot_assembly(g, x, false);
                            if ot_assembly.is_some() {
                                break 'found;
                            }
                        } else {
                            let fonts = &st.fonts;
                            let mut y = x as u16;
                            if y >= fonts.font_bc[g] && y <= fonts.font_ec[g] {
                                loop {
                                    q = fonts.char_info(g, i32::from(y));
                                    if !q.exists() {
                                        break;
                                    }
                                    if q.tag() == CharTag::Ext {
                                        f = g;
                                        c = y;
                                        break 'found;
                                    }
                                    let u = fonts.char_height(g, q) + fonts.char_depth(g, q);
                                    if u > w {
                                        f = g;
                                        c = y;
                                        w = u;
                                        if u >= v {
                                            break 'found;
                                        }
                                    }
                                    if q.tag() != CharTag::List {
                                        break;
                                    }
                                    y = q.remainder();
                                }
                            }
                        }
                    }
                    if z < SCRIPT_SIZE {
                        break;
                    }
                }
            }
            if large_attempt {
                break 'found;
            }
            large_attempt = true;
            (z, x) = d.large(&st.mem);
        }
    }
    let b = if f == NULL_FONT {
        let b = st.mem.new_null_box()?;
        let width = st.eqtb.dimen_par(DimenPar::NullDelimiterSpace);
        b.set_width(&mut st.mem, width);
        AnyBox::HList(b)
    } else if !st.fonts.font_uses_opentype(f) {
        if q.tag() == CharTag::Ext {
            st.mem.build_ext_delimiter_box(&mut st.fonts, f, q, v)?
        } else {
            char_box(st, f, i32::from(c))?
        }
    } else if let Some(assembly) = &ot_assembly {
        build_opentype_assembly(st, f, assembly, v, false)?
    } else {
        native_glyph_vbox(st, f, c)?
    };
    let axis = st.fonts.axis_height(&st.eqtb, s);
    let shift = (b.height(&st.mem) - b.depth(&st.mem)).half() - axis;
    b.set_shift_amount(&mut st.mem, shift);
    Ok(b)
}
