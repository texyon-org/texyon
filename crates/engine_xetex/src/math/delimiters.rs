//! Boxes for math characters and delimiters: `char_box`, `var_delimiter`, `rebox`.

use crate::consts::{BOX_NODE_SIZE, NULL_FONT, SCRIPT_SIZE};
use crate::errors::Abort;
use crate::fonts::{CharInfo, CharTag, AAT_FONT_FLAG, OTGR_FONT_FLAG};
use crate::mem::SS_GLUE;
use crate::node::{
    AnyNode, BoxNode, DelimiterField, GlyphNode, NativeWordNode, NodePtr, WhatsitSubtype,
};
use crate::ot_math::GlyphAssembly;
use crate::packaging::PackSpec;
use crate::params::DimenPar;
use crate::state::{EngineState, FontState, MemState};
use crate::units::{FontId, GlueOrder, GlueSign, Scaled};

impl FontState {
    pub(super) fn measure_glyph(
        &self,
        mem: &mut MemState,
        p: NodePtr<GlyphNode>,
    ) -> Result<(), Abort> {
        self.measure_native_glyph(mem, p, true)
    }
}

impl MemState {
    pub(super) fn native_char_box(
        &mut self,
        p: NodePtr<NativeWordNode>,
    ) -> Result<NodePtr<BoxNode>, Abort> {
        let b = self.new_null_box()?;
        self[b].height = self[p].height;
        self[b].width = self[p].width;
        self[b].depth = self[p].depth.max(Scaled::ZERO);
        self[b].list_ptr = p.ptr();
        Ok(b)
    }
}

fn char_box(st: &mut EngineState, f: FontId, c: i32) -> Result<NodePtr<BoxNode>, Abort> {
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
    ) -> Result<NodePtr<BoxNode>, Abort> {
        let c_eff = fonts.effective_char(f, c);
        let q = fonts.char_info(f, c_eff);
        let width = fonts.char_width(f, q) + fonts.char_italic(f, q);
        let height = fonts.char_height(f, q);
        let depth = fonts.char_depth(f, q);

        let b = self.new_null_box()?;
        self[b].width = width;
        self[b].height = height;
        self[b].depth = depth;
        let p = self.new_char(f, c)?;
        self[b].list_ptr = p.ptr();
        Ok(b)
    }

    fn stack_into_box(&mut self, b_box: NodePtr<BoxNode>, p: NodePtr<BoxNode>) {
        let link = self[b_box].list_ptr;
        self[p].link = link;
        self[b_box].list_ptr = p.ptr();
        let h = self[p].height;
        self[b_box].height = h;
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
) -> Result<NodePtr<GlyphNode>, Abort> {
    let p = NodePtr::<GlyphNode>::alloc_whatsit(&mut st.mem, WhatsitSubtype::Glyph)?;
    let glyph = &mut st.mem[p];
    glyph.font = f;
    glyph.glyph = g as u16;
    st.fonts.measure_glyph(&mut st.mem, p)?;
    Ok(p)
}

fn stack_glyph_into_box(
    st: &mut EngineState,
    b: NodePtr<BoxNode>,
    f: FontId,
    g: i32,
) -> Result<(), Abort> {
    let p = new_measured_glyph(st, f, g)?;
    let mem = &mut st.mem;
    if mem[b].is_hlist() {
        let mut q = mem[b].list_ptr;
        if q.is_null() {
            mem[b].list_ptr = p.ptr();
        } else {
            while !q.link(mem).is_null() {
                q = q.link(mem);
            }
            q.set_link(mem, p.ptr());
            if mem[b].height < mem[p].height {
                mem[b].height = mem[p].height;
            }
            if mem[b].depth < mem[p].depth {
                mem[b].depth = mem[p].depth;
            }
        }
    } else {
        mem[p].link = mem[b].list_ptr;
        mem[b].list_ptr = p.ptr();
        mem[b].height = mem[p].height;
        if mem[b].width < mem[p].width {
            mem[b].width = mem[p].width;
        }
    }
    Ok(())
}

fn native_glyph_vbox(st: &mut EngineState, f: FontId, g: u16) -> Result<NodePtr<BoxNode>, Abort> {
    let b = st.mem.new_null_vbox()?;
    let p = new_measured_glyph(st, f, i32::from(g))?;
    let mem = &mut st.mem;
    mem[b].list_ptr = p.ptr();
    mem[b].width = mem[p].width;
    mem[b].height = mem[p].height;
    mem[b].depth = mem[p].depth;
    Ok(b)
}

pub(super) fn rebox(
    st: &mut EngineState,
    b: NodePtr<BoxNode>,
    w: Scaled,
) -> Result<NodePtr<BoxNode>, Abort> {
    let mut b = b;
    if st.mem[b].width != w && !st.mem[b].list_ptr.is_null() && st.mem[b].is_vlist() {
        b = crate::packaging::hpack(st, b.ptr(), PackSpec::NATURAL)?;
    }
    if st.mem[b].width == w || st.mem[b].list_ptr.is_null() {
        st.mem[b].width = w;
        return Ok(b);
    }

    let mut p = st.mem[b].list_ptr;
    if let Some(ch) = st.mem.as_char(p).filter(|_| p.link(&st.mem).is_null()) {
        let f = st.mem[ch].font;
        let c = st.fonts.effective_char(f, st.mem[ch].character);
        let q = st.fonts.char_info(f, c);
        let v = st.fonts.char_width(f, q);
        let bw = st.mem[b].width;
        if v != bw {
            let k = st.mem.new_kern(bw - v)?;
            p.set_link(&mut st.mem, k.ptr());
        }
    }
    let mem = &mut st.mem;
    mem.free_node(b.ptr(), BOX_NODE_SIZE);
    let g = mem.new_glue(SS_GLUE)?;
    mem[g].link = p;
    while !p.link(mem).is_null() {
        p = p.link(mem);
    }
    let tail_glue = mem.new_glue(SS_GLUE)?;
    p.set_link(mem, tail_glue.ptr());
    crate::packaging::hpack(st, g.ptr(), PackSpec::Exactly(w))
}

pub(super) fn build_opentype_assembly(
    st: &mut EngineState,
    f: FontId,
    a: &GlyphAssembly,
    s: Scaled,
    horiz: bool,
) -> Result<NodePtr<BoxNode>, Abort> {
    let b = if horiz {
        st.mem.new_null_box()?
    } else {
        st.mem.new_null_vbox()?
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
    let mut p = st.mem[b].list_ptr;
    let mut nat = Scaled::ZERO;
    let mut str = Scaled::ZERO;
    while !p.is_null() {
        match st.mem.classify(p) {
            Ok(AnyNode::Whatsit(pw)) => {
                if horiz {
                    nat += pw.width(&st.mem);
                } else {
                    nat += pw.height(&st.mem) + pw.depth(&st.mem);
                }
            }
            Ok(AnyNode::Glue(gn)) => {
                let gs = st.mem[gn].glue_ptr;
                nat += st.mem[gs].width;
                str += st.mem[gs].stretch;
            }
            _ => {}
        }
        p = p.link(&st.mem);
    }
    if s > nat && str > Scaled::ZERO {
        let o = (s - nat).min(str);
        st.mem[b].glue_order = GlueOrder::Normal.into();
        st.mem[b].glue_sign = GlueSign::Stretching.into();
        let ratio = f64::from(o) / f64::from(str);
        st.mem[b].glue_set = ratio;
        let set_dim = nat + Scaled::round(f64::from(str) * ratio);
        if horiz {
            st.mem[b].width = set_dim;
        } else {
            st.mem[b].height = set_dim;
        }
    } else if horiz {
        st.mem[b].width = nat;
    } else {
        st.mem[b].height = nat;
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
    ) -> Result<NodePtr<BoxNode>, Abort> {
        let vb = self.new_null_vbox()?;
        let r = fonts.ext_recipe(f, q);
        let u = fonts.height_plus_depth(f, r.rep);
        let effective_c = fonts.effective_char(f, r.rep);
        let qq = fonts.char_info(f, effective_c);
        let box_width = fonts.char_width(f, qq) + fonts.char_italic(f, qq);
        self[vb].width = box_width;
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
        let height = self[vb].height;
        self[vb].depth = w - height;
        Ok(vb)
    }
}

pub(super) fn var_delimiter(
    st: &mut EngineState,
    d: NodePtr<DelimiterField>,
    s: i32,
    v: Scaled,
) -> Result<NodePtr<BoxNode>, Abort> {
    let mut f = NULL_FONT;
    let mut ot_assembly: Option<GlyphAssembly> = None;
    let mut c: u16 = 0;
    let mut w = Scaled::ZERO;
    let mut q = CharInfo::NULL;
    let mut large_attempt = false;
    let (mut z, mut x) = st.mem[d].small();
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
            (z, x) = st.mem[d].large();
        }
    }
    let b = if f == NULL_FONT {
        let b = st.mem.new_null_box()?;
        let width = st.eqtb.dimen_par(DimenPar::NullDelimiterSpace);
        st.mem[b].width = width;
        b
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
    let shift = (st.mem[b].height - st.mem[b].depth).half() - axis;
    st.mem[b].shift_amount = shift;
    Ok(b)
}
