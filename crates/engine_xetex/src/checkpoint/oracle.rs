//! Named state digests used only to verify checkpoint restoration.

use crate::mem_types::{B32x2, ListStateRecord, MemoryWord};
use crate::state::{EngineState, InputName, InputState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComponentDigest {
    pub(crate) name: &'static str,
    pub(crate) value: u64,
}

/// A table value widened to a hash word. Signed values sign-extend.
trait HashWord {
    fn hash_word(self) -> u64;
}

macro_rules! unsigned_hash_word {
    ($($t:ty),*) => {$(
        impl HashWord for $t {
            fn hash_word(self) -> u64 {
                u64::from(self)
            }
        }
    )*};
}
unsigned_hash_word!(bool, u8, u16, u32);

macro_rules! signed_hash_word {
    ($($t:ty),*) => {$(
        impl HashWord for $t {
            fn hash_word(self) -> u64 {
                i64::from(self) as u64
            }
        }
    )*};
}
signed_hash_word!(i8, i16, i32);

impl HashWord for usize {
    fn hash_word(self) -> u64 {
        u64::try_from(self).unwrap_or(u64::MAX)
    }
}

pub(crate) fn digest_state(st: &EngineState) -> Vec<ComponentDigest> {
    let component = |name, digest: fn(&EngineState) -> u64| ComponentDigest {
        name,
        value: digest(st),
    };
    vec![
        component("mem", digest_mem),
        component("eqtb", digest_eqtb),
        component("strings", digest_strings),
        component("save", digest_save),
        component("fonts", digest_fonts),
        component("input", digest_input),
        component("scanner", digest_scanner),
        component("hyph", digest_hyph),
    ]
}

#[derive(Clone, Copy)]
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn word(&mut self, value: u64) {
        self.0 = (self.0 ^ value).wrapping_mul(0x0000_0100_0000_01b3);
    }

    fn i32(&mut self, value: i32) {
        self.word(u64::from(value as u32));
    }

    fn i32s<'a>(&mut self, values: impl IntoIterator<Item = &'a i32>) {
        for &value in values {
            self.i32(value);
        }
    }

    fn finish(self) -> u64 {
        self.0
    }
}

fn memory_word(word: MemoryWord) -> u64 {
    let B32x2 { s0, s1 } = word.b32();
    u64::from(s0 as u32) | u64::from(s1 as u32) << 32
}

fn digest_mem(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let mem = &st.mem;
    h.i32s(&[
        i32::try_from(mem.data.len()).unwrap_or(i32::MAX),
        mem.line,
        mem.cur_synctex_tag,
        mem.temp_ptr.raw(),
    ]);
    h.i32s(
        mem.free
            .0
            .iter()
            .map(|p| p.raw())
            .collect::<Vec<_>>()
            .iter(),
    );
    for &word in &mem.data {
        h.word(u64::from(word));
    }
    for slot in mem.payloads.slots() {
        match slot {
            None => h.word(u64::MAX),
            Some(words) => {
                h.word(words.len() as u64);
                for &w in words {
                    h.word(u64::from(w));
                }
            }
        }
    }
    let glue = mem.glue.marks();
    h.word(glue.len as u64);
    h.i32(glue.free);
    for &spec in mem.glue.specs() {
        let words: [u32; 5] = bytemuck::cast(spec);
        for w in words {
            h.word(u64::from(w));
        }
    }
    let marks = mem.tokens.marks();
    for free in marks.free_runs {
        h.word(u64::from(free));
    }
    h.word(u64::from(marks.free_lists));
    h.word(mem.tokens.tokens().len() as u64);
    for &t in mem.tokens.tokens() {
        h.i32(t.0);
    }
    h.word(mem.tokens.lists().len() as u64);
    for &slot in mem.tokens.lists() {
        let words: [u32; 3] = bytemuck::cast(slot);
        for w in words {
            h.word(u64::from(w));
        }
    }
    for slot in mem.glyph_blobs.slots() {
        match slot {
            None => h.word(0),
            Some(blob) => {
                h.word(1);
                h.word(blob.locations.len() as u64);
                for location in &blob.locations {
                    h.i32(location.x.raw());
                    h.i32(location.y.raw());
                }
                h.word(blob.gids.len() as u64);
                for &gid in &blob.gids {
                    h.word(u64::from(gid));
                }
            }
        }
    }
    h.finish()
}

fn digest_eqtb(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let eqtb = &st.eqtb;
    h.i32s(&[
        eqtb.hash_used.raw(),
        eqtb.hash_high,
        eqtb.prim_used.raw(),
        eqtb.cs_count,
        eqtb.hash_extra,
        eqtb.cs_top.raw(),
    ]);
    h.word(u64::from(eqtb.no_new_control_sequence));
    for &word in &eqtb.data {
        h.word(memory_word(word));
    }
    for &value in &eqtb.xeq_level {
        h.word(u64::from(value));
    }
    for value in eqtb.hash.iter().chain(&eqtb.prim) {
        h.i32(value.s0);
        h.i32(value.s1);
    }
    h.finish()
}

fn digest_strings(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let strings = &st.strings;
    h.i32s(&[
        strings.str_ptr.raw(),
        strings.max_strings.raw(),
        strings.init_str_ptr.raw(),
        strings.init_pool_ptr.raw(),
        strings.job_name.raw(),
        strings.texmf_log_name.raw(),
        strings.string_vacancies,
        strings.pool_size.raw(),
        strings.pool_ptr.raw(),
        strings.format_ident.raw(),
    ]);
    let start_top = strings.cur_start_slot();
    for p in strings.str_start.iter().take(start_top.saturating_add(1)) {
        h.i32(p.raw());
    }
    for &value in strings.str_pool.iter().take(strings.pool_ptr.index()) {
        h.word(u64::from(value));
    }
    h.finish()
}

fn digest_save(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let save = &st.save_stack;
    h.i32(save.save_ptr.raw());
    h.i32(save.max_save_stack.raw());
    h.i32(depth(save.save_size));
    h.word(u64::from(save.cur_level));
    h.word(u64::from(save.cur_group.to_u16()));
    h.i32(save.cur_boundary.raw());
    for &word in save.data.iter().take(save.save_ptr.index()) {
        h.word(memory_word(word));
    }
    h.finish()
}

fn digest_fonts(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let fonts = &st.fonts;
    h.i32s(&[
        i32::from(fonts.loaded_font_flags),
        fonts.loaded_font_letter_space.raw(),
        fonts.native_font_type_flag.raw(),
        fonts.font_mem_size.raw(),
        fonts.font_max.raw(),
        fonts.fmem_ptr.raw(),
        fonts.font_ptr.raw(),
    ]);
    h.word(u64::from(fonts.xtx_ligature_present));
    h.word(u64::from(fonts.gave_char_warning_help));
    for &word in fonts.font_info.iter().take(fonts.fmem_ptr.index()) {
        h.word(memory_word(word));
    }
    let font_count = (fonts.font_ptr.raw() + 1).max(0) as usize;
    macro_rules! hash_values {
        ($field:ident $(.$raw:ident())?) => {
            for &value in fonts.$field.as_slice().iter().take(font_count) {
                h.word(HashWord::hash_word(value$(.$raw())?));
            }
        };
    }
    hash_values!(font_flags);
    hash_values!(font_letter_space.raw());
    hash_values!(font_size.raw());
    hash_values!(font_dsize.raw());
    hash_values!(font_params);
    hash_values!(font_name.raw());
    hash_values!(font_area.raw());
    hash_values!(font_bc);
    hash_values!(font_ec);
    hash_values!(bchar_label.raw());
    hash_values!(font_bchar.raw());
    hash_values!(font_false_bchar.raw());
    for &glue in fonts.font_glue.as_slice().iter().take(font_count) {
        h.i32(glue.map_or(crate::consts::TEX_NULL, crate::glue_store::GlueSpecId::raw));
    }
    hash_values!(font_used);
    hash_values!(char_base.raw());
    hash_values!(width_base.raw());
    hash_values!(height_base.raw());
    hash_values!(depth_base.raw());
    hash_values!(italic_base.raw());
    hash_values!(lig_kern_base.raw());
    hash_values!(kern_base.raw());
    hash_values!(exten_base.raw());
    hash_values!(param_base.raw());
    hash_values!(hyphen_char);
    hash_values!(skew_char);
    for value in fonts.font_check.as_slice().iter().take(font_count) {
        for &b in value {
            h.word(u64::from(b));
        }
    }
    for engine in fonts.font_layout_engine.as_slice().iter().take(font_count) {
        h.word(u64::from(engine.is_some()));
    }
    for (&(f, side, c), &value) in fonts.cp_codes.iter() {
        h.i32s(&[f.raw(), side as i32, value]);
        h.word(u64::from(c));
    }
    for mapping in fonts.font_mapping.as_slice().iter().take(font_count) {
        h.word(u64::from(mapping.is_some()));
    }
    h.finish()
}

fn digest_input_state(h: &mut Fnv, input: &InputState) {
    h.word(u64::from(input.state));
    h.word(u64::from(input.index));
    h.i32(input.start.raw());
    h.i32(input.loc.raw());
    h.i32(input.limit.raw());
    h.i32(input.token_start().raw());
    let (pos, end) = input.token_loc().parts();
    h.word(u64::from(pos));
    h.word(u64::from(end));
    h.i32(i32::try_from(input.param_start()).unwrap_or(i32::MAX));
    digest_input_name(h, input.name);
    h.i32(input.synctex_tag);
}

fn digest_input_name(h: &mut Fnv, name: InputName) {
    let (kind, value) = match name {
        InputName::Terminal => (0, 0),
        InputName::Read(n) => (1, i32::from(n)),
        InputName::ReadTerminal => (2, 0),
        InputName::Pseudo { traced } => (3, i32::from(traced)),
        InputName::File(s) => (4, s.raw()),
        InputName::Macro(cs) => (5, cs.raw()),
    };
    h.i32(kind);
    h.i32(value);
}

fn digest_list(h: &mut Fnv, list: &ListStateRecord) {
    h.i32(list.mode.into());
    h.i32(list.head.raw());
    h.i32(list.tail.raw());
    h.i32(list.e_tex_aux.raw());
    h.i32(list.prev_graf);
    h.i32(list.mode_line);
    h.word(memory_word(list.aux));
}

/// A stack depth as the digest's `i32` words.
fn depth(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

fn digest_input(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let input = &st.input;
    h.i32s(&[
        input.open_parens,
        depth(input.in_open),
        input.pseudo_files.raw(),
        depth(input.nest.len()),
        depth(input.nest.max_len()),
        depth(input.input_stack.len()),
        depth(input.input_stack.max_len()),
        depth(input.base_ptr),
        input.par_loc.raw(),
        input.par_token.0,
        depth(input.param_stack.len()),
        depth(input.param_stack.max_len()),
        input.first.raw(),
        input.last.raw(),
        input.max_buf_stack.raw(),
    ]);
    h.word(u64::from(input.force_eof));
    h.i32s(input.buffer.0.iter().take(input.buffer_in_use()));
    for value in input.input_stack.with_parked() {
        digest_input_state(&mut h, value);
    }
    digest_input_state(&mut h, &input.cur_input);
    for value in input.nest.with_parked() {
        digest_list(&mut h, value);
    }
    digest_list(&mut h, &input.cur_list);
    for p in input.param_stack.entries() {
        h.i32(p.list.raw());
    }
    for level in &input.open_levels {
        h.word(level.cond.map_or(u64::MAX, |c| u64::from(c.raw())));
        h.i32s(&[
            level.outer_line,
            level.grp.raw(),
            level.source_name.raw(),
            level.full_source_name.raw(),
        ]);
        h.word(u64::from(level.eof_seen));
    }
    h.finish()
}

fn digest_scanner(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let scanner = &st.scanner;
    for value in [
        scanner.logical_clock as i64,
        i64::from(scanner.align_state),
        i64::try_from(scanner.conds.depth()).unwrap_or(i64::MAX),
        i64::from(scanner.conds.next_id()),
        i64::from(scanner.cur_cs.raw()),
        i64::from(i32::from(scanner.cur.encode().0)),
        i64::from(scanner.cur.encode().1),
        i64::from(scanner.cur_tok.0),
        i64::from(st.shipout.page_tail.raw()),
        st.shipout.page_contents as i64,
        i64::from(st.shipout.dead_cycles),
        i64::from(st.shipout.total_pages),
        i64::from(st.synctex.synctex_tag_counter),
    ] {
        h.word(value as u64);
    }
    for &value in &scanner.native_text {
        h.word(u64::from(value));
    }
    let (position, buf) = st.shipout.dvi.digest_view();
    for value in position {
        h.word(i64::from(value) as u64);
    }
    for &value in buf {
        h.word(u64::from(value));
    }
    for cond in scanner.conds.iter_from_top() {
        h.word(u64::from(cond.id.raw()));
        h.i32s(&[
            i32::from(cond.cur_if),
            i32::from(cond.if_limit.raw()),
            cond.if_line,
        ]);
    }
    for a in &scanner.align_stack {
        h.i32s(&[
            a.cur_align.raw(),
            a.preamble.raw(),
            a.cur_span.raw(),
            a.cur_loop.raw(),
            a.align_state,
            a.cur_head.raw(),
            a.cur_tail.raw(),
            a.cur_pre_head.raw(),
            a.cur_pre_tail.raw(),
        ]);
    }
    for moves in [&st.shipout.down_moves, &st.shipout.right_moves] {
        h.word(moves.len() as u64);
        for m in moves {
            h.i32s(&[m.state, m.distance.raw(), m.location]);
        }
    }
    h.finish()
}

fn digest_hyph(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let hyph = &st.hyph;
    h.word(hyph.tables_gen);
    h.i32s(&[
        hyph.hf.raw(),
        hyph.max_hyph_char,
        hyph.trie_max.raw(),
        hyph.trie_size,
        hyph.trie_ptr,
        hyph.hyph_count,
        hyph.hyph_size,
        hyph.hyph_next,
        hyph.trie_op_ptr,
        hyph.hyph_start.raw(),
        hyph.hyph_index.map_or(0, |p| p.raw()),
    ]);
    macro_rules! hash_values {
        ($field:ident $(.$raw:ident())?) => {
            for &value in &hyph.$field {
                h.word(HashWord::hash_word(value$(.$raw())?));
            }
        };
    }
    hash_values!(hc);
    hash_values!(hu.raw());
    hash_values!(hyf);
    hash_values!(trie_min);
    hash_values!(hyf_distance);
    hash_values!(hyf_num);
    hash_values!(hyf_next);
    hash_values!(op_start);
    hash_values!(trie_used);
    hash_values!(trie_op_lang);
    hash_values!(trie_op_val);
    hash_values!(trie_trl);
    hash_values!(trie_tro);
    hash_values!(trie_trc);
    hash_values!(trie_c);
    hash_values!(trie_o);
    hash_values!(trie_l);
    hash_values!(trie_r);
    hash_values!(trie_hash);
    hash_values!(trie_taken);
    hash_values!(hyph_word.raw());
    hash_values!(hyph_list.raw());
    hash_values!(hyph_link);
    hash_values!(trie_op_hash_array);
    h.finish()
}
