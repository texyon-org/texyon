//! Named state digests used only to verify checkpoint restoration.

use std::time::{Duration, Instant};

use crate::state::EngineState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComponentDigest {
    pub(crate) name: &'static str,
    pub(crate) value: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct DigestTiming {
    pub(crate) name: &'static str,
    pub(crate) elapsed: Duration,
}

pub(crate) fn digest_state(st: &EngineState) -> Vec<ComponentDigest> {
    digest_state_with_timings(st).0
}

/// Diagnostic timing probe. Its clock values never enter engine state or a
/// digest; callers may report them as observability data only.
pub(crate) fn digest_state_with_timings(
    st: &EngineState,
) -> (Vec<ComponentDigest>, Vec<DigestTiming>) {
    let groups: [(&'static str, fn(&EngineState) -> u64); 8] = [
        ("mem", digest_mem),
        ("eqtb", digest_eqtb),
        ("strings", digest_strings),
        ("save", digest_save),
        ("fonts", digest_fonts),
        ("input", digest_input),
        ("scanner", digest_scanner),
        ("hyph", digest_hyph),
    ];
    let mut digests = Vec::with_capacity(groups.len());
    let mut timings = Vec::with_capacity(groups.len());
    for (name, digest) in groups {
        let started = Instant::now();
        let value = digest(st);
        timings.push(DigestTiming {
            name,
            elapsed: started.elapsed(),
        });
        digests.push(ComponentDigest { name, value });
    }
    (digests, timings)
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
        self.word(value as u32 as u64);
    }

    fn finish(self) -> u64 {
        self.0
    }
}

fn memory_word(word: crate::mem_types::MemoryWord) -> u64 {
    // SAFETY: MemoryWord is the engine's exactly-eight-byte C union.
    unsafe { std::mem::transmute(word) }
}

fn digest_mem(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let mem = &st.mem;
    for value in [
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
    ] {
        h.i32(value);
    }
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
                    h.i32(location.x);
                    h.i32(location.y);
                }
                h.word(blob.gids.len() as u64);
                for &gid in &blob.gids {
                    h.word(gid as u64);
                }
            }
        }
    }
    h.finish()
}

fn digest_eqtb(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let eqtb = &st.eqtb;
    for value in [
        eqtb.hash_used,
        eqtb.hash_high,
        eqtb.prim_used,
        eqtb.cs_count,
        eqtb.hash_extra,
        eqtb.eqtb_top,
        eqtb.hash_top,
    ] {
        h.i32(value);
    }
    h.word(eqtb.no_new_control_sequence as u64);
    for &word in &eqtb.data {
        h.word(memory_word(word));
    }
    for &value in &eqtb.xeq_level {
        h.word(value as u64);
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
    for value in [
        strings.str_ptr,
        strings.max_strings,
        strings.init_str_ptr,
        strings.init_pool_ptr,
        strings.job_name,
        strings.texmf_log_name,
        strings.string_vacancies,
        strings.pool_size,
        strings.pool_ptr,
        strings.format_ident,
        strings.edit_name_start,
    ] {
        h.i32(value);
    }
    let start_top = (strings.str_ptr - crate::consts::TOO_BIG_CHAR).max(0) as usize;
    for &value in strings.str_start.iter().take(start_top.saturating_add(1)) {
        h.i32(value);
    }
    for &value in strings
        .str_pool
        .iter()
        .take(strings.pool_ptr.max(0) as usize)
    {
        h.word(value as u64);
    }
    h.finish()
}

fn digest_save(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let save = &st.save_stack;
    h.i32(save.save_ptr);
    h.i32(save.max_save_stack);
    h.i32(save.save_size);
    h.word(save.cur_level as u64);
    h.word(save.cur_group.as_raw() as u64);
    h.i32(save.cur_boundary);
    for &word in save.data.iter().take(save.save_ptr.max(0) as usize) {
        h.word(memory_word(word));
    }
    h.finish()
}

fn digest_fonts(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let fonts = &st.fonts;
    for value in [
        fonts.loaded_font_flags as i32,
        fonts.loaded_font_letter_space,
        fonts.native_font_type_flag,
        fonts.font_mem_size,
        fonts.font_max,
        fonts.fmem_ptr,
        fonts.font_ptr,
    ] {
        h.i32(value);
    }
    h.word(fonts.xtx_ligature_present as u64);
    h.word(fonts.gave_char_warning_help as u64);
    for &word in fonts.font_info.iter().take(fonts.fmem_ptr.max(0) as usize) {
        h.word(memory_word(word));
    }
    let font_count = (fonts.font_ptr + 1).max(0) as usize;
    macro_rules! hash_values {
        ($field:ident) => {
            for &value in fonts.$field.iter().take(font_count) {
                h.word(value as u64);
            }
        };
    }
    hash_values!(font_flags);
    hash_values!(font_letter_space);
    hash_values!(font_size);
    hash_values!(font_dsize);
    hash_values!(font_params);
    hash_values!(font_name);
    hash_values!(font_area);
    hash_values!(font_bc);
    hash_values!(font_ec);
    hash_values!(bchar_label);
    hash_values!(font_bchar);
    hash_values!(font_false_bchar);
    hash_values!(font_glue);
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
    for value in fonts.font_check.iter().take(font_count) {
        h.word(value.s0 as u64);
        h.word(value.s1 as u64);
        h.word(value.s2 as u64);
        h.word(value.s3 as u64);
    }
    for engine in fonts.font_layout_engine.iter().take(font_count) {
        h.word(engine.is_some() as u64);
    }
    for mapping in fonts.font_mapping.iter().take(font_count) {
        h.word(mapping.is_some() as u64);
    }
    h.finish()
}

fn digest_input_state(h: &mut Fnv, input: &crate::globals::InputState) {
    h.word(input.state as u64);
    h.word(input.index as u64);
    h.i32(input.start);
    h.i32(input.loc);
    h.i32(input.limit);
    h.i32(input.name);
    h.i32(input.synctex_tag);
}

fn digest_list(h: &mut Fnv, list: &crate::mem_types::ListStateRecord) {
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
    for value in [
        input.open_parens,
        input.in_open,
        input.pseudo_files.0,
        input.nest_ptr,
        input.max_nest_stack,
        input.input_ptr,
        input.max_in_stack,
        input.base_ptr,
        input.par_loc,
        input.par_token,
        input.param_ptr,
        input.max_param_stack,
        input.first,
        input.last,
        input.max_buf_stack,
    ] {
        h.i32(value);
    }
    h.word(input.force_eof as u64);
    let buffer_end = (input.max_buf_stack.max(input.last).max(input.first) + 1).max(0) as usize;
    for &value in input.buffer.iter().take(buffer_end) {
        h.i32(value);
    }
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
    for &value in input
        .param_stack
        .iter()
        .take(input.param_ptr.max(0) as usize)
    {
        h.i32(value);
    }
    for &value in &input.line_stack {
        h.i32(value);
    }
    for &value in &input.grp_stack {
        h.i32(value);
    }
    for &value in &input.if_stack {
        h.i32(value.0);
    }
    for &value in &input.source_filename_stack {
        h.i32(value);
    }
    for &value in &input.full_source_filename_stack {
        h.i32(value);
    }
    for &value in &input.eof_seen {
        h.word(value as u64);
    }
    h.finish()
}

fn digest_scanner(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let scanner = &st.scanner;
    for value in [
        scanner.logical_clock as i64,
        scanner.align_state as i64,
        scanner.cond_ptr.0 as i64,
        scanner.cur_cs as i64,
        scanner.cur_cmd as i64,
        scanner.cur_chr as i64,
        scanner.cur_tok as i64,
        scanner.cur_val as i64,
        st.shipout.page_tail.0 as i64,
        st.shipout.page_contents as i64,
        st.shipout.dead_cycles as i64,
        st.shipout.total_pages as i64,
        st.shipout.dvi_ptr as i64,
        st.shipout.dvi_offset as i64,
        st.shipout.dvi_gone as i64,
        st.synctex.synctex_tag_counter as i64,
    ] {
        h.word(value as u64);
    }
    for &value in &scanner.native_text {
        h.word(value as u64);
    }
    for &value in &st.shipout.dvi_buf {
        h.word(value as u64);
    }
    h.finish()
}

fn digest_hyph(st: &EngineState) -> u64 {
    let mut h = Fnv::new();
    let hyph = &st.hyph;
    h.word(hyph.tables_gen);
    for value in [
        hyph.hf,
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
    ] {
        h.i32(value);
    }
    macro_rules! hash_values {
        ($field:ident) => {
            for &value in &hyph.$field {
                h.word(value as u64);
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
    hash_values!(hyph_word);
    hash_values!(hyph_list);
    hash_values!(hyph_link);
    hash_values!(trie_op_hash_array);
    h.finish()
}
