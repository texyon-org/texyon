//! Named state digests used only to verify checkpoint restoration.

use crate::mem_types::{B32x2, ListStateRecord, MemoryWord};
use crate::state::{EngineState, InputState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComponentDigest {
    pub(crate) name: &'static str,
    pub(crate) value: u64,
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
        mem.lo_mem_max,
        mem.hi_mem_min,
        mem.avail,
        mem.mem_end,
        mem.rover,
        mem.var_used,
        mem.dyn_used,
        mem.line,
        mem.cur_synctex_tag,
        mem.temp_ptr.0,
    ]);
    for &word in &mem.data {
        h.word(memory_word(word));
    }
    for slot in mem.glyph_blobs.slots() {
        match slot {
            None => h.word(0),
            Some(blob) => {
                h.word(1);
                h.word(blob.locations.len() as u64);
                for location in &blob.locations {
                    h.i32(location.x.0);
                    h.i32(location.y.0);
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
        eqtb.hash_used,
        eqtb.hash_high,
        eqtb.prim_used,
        eqtb.cs_count,
        eqtb.hash_extra,
        eqtb.eqtb_top,
        eqtb.hash_top,
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
        strings.str_ptr,
        strings.max_strings,
        strings.init_str_ptr,
        strings.init_pool_ptr,
        strings.job_name.0,
        strings.texmf_log_name.0,
        strings.string_vacancies,
        strings.pool_size,
        strings.pool_ptr,
        strings.format_ident.0,
        strings.edit_name_start,
    ]);
    let start_top = strings.cur_start_slot();
    h.i32s(strings.str_start.iter().take(start_top.saturating_add(1)));
    for &value in strings
        .str_pool
        .iter()
        .take(strings.pool_ptr.max(0) as usize)
    {
        h.word(u64::from(value));
    }
    h.finish()
}

fn digest_save(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let save = &st.save_stack;
    h.i32(save.save_ptr);
    h.i32(save.max_save_stack);
    h.i32(save.save_size);
    h.word(u64::from(save.cur_level));
    h.word(u64::from(save.cur_group.as_raw()));
    h.i32(save.cur_boundary);
    for &word in save.data.iter().take(save.save_ptr.max(0) as usize) {
        h.word(memory_word(word));
    }
    h.finish()
}

fn digest_fonts(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let fonts = &st.fonts;
    h.i32s(&[
        i32::from(fonts.loaded_font_flags),
        fonts.loaded_font_letter_space.0,
        fonts.native_font_type_flag.0,
        fonts.font_mem_size,
        fonts.font_max.0,
        fonts.fmem_ptr,
        fonts.font_ptr.0,
    ]);
    h.word(u64::from(fonts.xtx_ligature_present));
    h.word(u64::from(fonts.gave_char_warning_help));
    for &word in fonts.font_info.iter().take(fonts.fmem_ptr.max(0) as usize) {
        h.word(memory_word(word));
    }
    let font_count = (fonts.font_ptr.0 + 1).max(0) as usize;
    macro_rules! hash_values {
        ($field:ident $(.$raw:tt)?) => {
            for &value in fonts.$field.as_slice().iter().take(font_count) {
                h.word(value$(.$raw)? as u64);
            }
        };
    }
    hash_values!(font_flags);
    hash_values!(font_letter_space.0);
    hash_values!(font_size.0);
    hash_values!(font_dsize.0);
    hash_values!(font_params);
    hash_values!(font_name.0);
    hash_values!(font_area.0);
    hash_values!(font_bc);
    hash_values!(font_ec);
    hash_values!(bchar_label);
    hash_values!(font_bchar);
    hash_values!(font_false_bchar);
    hash_values!(font_glue.0);
    hash_values!(font_used);
    hash_values!(char_base);
    hash_values!(width_base);
    hash_values!(height_base);
    hash_values!(depth_base);
    hash_values!(italic_base);
    hash_values!(lig_kern_base);
    hash_values!(kern_base);
    hash_values!(exten_base);
    hash_values!(param_base);
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
    for mapping in fonts.font_mapping.as_slice().iter().take(font_count) {
        h.word(u64::from(mapping.is_some()));
    }
    h.finish()
}

fn digest_input_state(h: &mut Fnv, input: &InputState) {
    h.word(u64::from(input.state));
    h.word(u64::from(input.index));
    h.i32(input.start);
    h.i32(input.loc);
    h.i32(input.limit);
    h.i32(input.name);
    h.i32(input.synctex_tag);
}

fn digest_list(h: &mut Fnv, list: &ListStateRecord) {
    h.i32(list.mode.into());
    h.i32(list.head.0);
    h.i32(list.tail.0);
    h.i32(list.e_tex_aux.0);
    h.i32(list.prev_graf);
    h.i32(list.mode_line);
    h.word(memory_word(list.aux));
}

fn digest_input(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let input = &st.input;
    h.i32s(&[
        input.open_parens,
        input.in_open,
        input.pseudo_files.0,
        input.nest_ptr,
        input.max_nest_stack,
        input.input_ptr,
        input.max_in_stack,
        input.base_ptr,
        input.par_loc,
        input.par_token.0,
        input.param_ptr,
        input.max_param_stack,
        input.first,
        input.last,
        input.max_buf_stack,
    ]);
    h.word(u64::from(input.force_eof));
    let buffer_end = input.buffer_in_use().max(0) as usize;
    h.i32s(input.buffer.iter().take(buffer_end));
    for value in input
        .input_stack
        .iter()
        .take((input.input_ptr + 1).max(0) as usize)
    {
        digest_input_state(&mut h, value);
    }
    digest_input_state(&mut h, &input.cur_input);
    for value in input.nest.iter().take((input.nest_ptr + 1).max(0) as usize) {
        digest_list(&mut h, value);
    }
    digest_list(&mut h, &input.cur_list);
    h.i32s(
        input
            .param_stack
            .iter()
            .take(input.param_ptr.max(0) as usize)
            .map(|p| &p.0),
    );
    h.i32s(&input.line_stack);
    h.i32s(&input.grp_stack);
    for &value in &input.if_stack {
        h.i32(value.0);
    }
    h.i32s(input.source_filename_stack.iter().map(|s| &s.0));
    h.i32s(input.full_source_filename_stack.iter().map(|s| &s.0));
    for &value in &input.eof_seen {
        h.word(u64::from(value));
    }
    h.finish()
}

fn digest_scanner(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let scanner = &st.scanner;
    for value in [
        scanner.logical_clock as i64,
        i64::from(scanner.align_state),
        i64::from(scanner.cond_ptr.0),
        i64::from(scanner.cur_cs),
        i64::from(scanner.cur_cmd),
        i64::from(scanner.cur_chr),
        i64::from(scanner.cur_tok.0),
        i64::from(scanner.cur_val),
        i64::from(st.shipout.page_tail.0),
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
    h.finish()
}

fn digest_hyph(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let hyph = &st.hyph;
    h.word(hyph.tables_gen);
    h.i32s(&[
        hyph.hf.0,
        hyph.max_hyph_char,
        hyph.trie_max,
        hyph.trie_size,
        hyph.trie_ptr,
        hyph.hyph_count,
        hyph.hyph_size,
        hyph.hyph_next,
        hyph.trie_op_ptr,
        hyph.hyph_start,
        hyph.hyph_index,
    ]);
    macro_rules! hash_values {
        ($field:ident $(.$raw:tt)?) => {
            for &value in &hyph.$field {
                h.word(value$(.$raw)? as u64);
            }
        };
    }
    hash_values!(hc);
    hash_values!(hu);
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
    hash_values!(hyph_word.0);
    hash_values!(hyph_list);
    hash_values!(hyph_link);
    hash_values!(trie_op_hash_array);
    h.finish()
}
