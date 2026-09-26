//! Non-journaled state carried by a checkpoint rung.

use std::mem::{size_of, take};
use std::sync::Arc;

use crate::mem_types::{B16x4, ListStateRecord, MemoryWord};
use crate::state::{
    ConfigState, EngineState, EqtbState, ErrorState, FontEngineHandle, FontMappingHandle,
    FontState, GroupCode, HyphState, InputClusterState, InputState, LinebreakState, MainLoopState,
    MemState, SaveStackState, ScannerState, ShipoutState, StringsState, SynctexCtxt, TexmfState,
};
use crate::units::{FontId, MemPtr, Scaled, StrNumber};

fn vec_bytes<T>(values: &Vec<T>) -> usize {
    values.capacity() * size_of::<T>()
}

#[derive(Clone, Copy)]
pub(crate) struct StringWatermark {
    pool: usize,
    starts: usize,
}

impl StringWatermark {
    pub(crate) fn capture(st: &EngineState) -> Self {
        Self::of(&st.strings)
    }

    fn of(strings: &StringsState) -> Self {
        Self {
            pool: strings.pool_ptr.max(0) as usize,
            starts: strings.cur_start_slot(),
        }
    }
}

/// Large buffers are held outside their state structs so restore can reuse
/// the live allocation.
pub(crate) struct EngineBundle {
    config: ConfigState,
    errors: ErrorState,
    shipout: ShipoutState,
    dvi_buf: Vec<u8>,
    scanner: ScannerState,
    native_text: Vec<u16>,
    linebreak: LinebreakState,
    hlist_stack: Vec<MemPtr>,
    texmf: TexmfState,
    synctex: SynctexCtxt,
    mainloop: MainLoopState,
    mem: MemBundle,
    eqtb: EqtbBundle,
    strings: StringsBundle,
    save: SaveBundle,
    input: InputBundle,
    fonts: FontBundle,
    hyph: HyphBundle,
}

impl EngineBundle {
    pub(crate) fn capture(
        st: &EngineState,
        strings_base: StringWatermark,
        previous: Option<&Self>,
    ) -> Self {
        // Page facts belong to the operation, not to replayable state.
        let mut shipout = st.shipout.clone();
        shipout.dvi.manifest = Default::default();
        let dvi_buf = take(&mut shipout.dvi.buf);
        let mut scanner = st.scanner.clone();
        let native_text = take(&mut scanner.native_text);
        let mut linebreak = st.linebreak.clone();
        let hlist_stack = take(&mut linebreak.hlist_stack);
        Self {
            config: st.config,
            errors: st.errors.clone(),
            shipout,
            dvi_buf,
            scanner,
            native_text,
            linebreak,
            hlist_stack,
            texmf: st.texmf.clone(),
            synctex: st.synctex.clone(),
            mainloop: st.mainloop.clone(),
            mem: MemBundle::capture(&st.mem),
            eqtb: EqtbBundle::capture(&st.eqtb),
            strings: StringsBundle::capture(&st.strings, strings_base),
            save: SaveBundle::capture(&st.save_stack),
            input: InputBundle::capture(&st.input),
            fonts: FontBundle::capture(&st.fonts),
            hyph: HyphBundle::capture(&st.hyph, previous.map(|b| &b.hyph)),
        }
    }

    pub(crate) fn restore(&self, st: &mut EngineState) {
        st.config = self.config;
        st.errors.clone_from(&self.errors);

        let dvi_buf = take(&mut st.shipout.dvi.buf);
        let manifest = take(&mut st.shipout.dvi.manifest);
        st.shipout = self.shipout.clone();
        st.shipout.dvi.buf = dvi_buf;
        st.shipout.dvi.manifest = manifest;
        st.shipout.dvi.buf.clone_from(&self.dvi_buf);

        let native_text = take(&mut st.scanner.native_text);
        st.scanner = ScannerState {
            native_text,
            ..self.scanner.clone()
        };
        st.scanner.native_text.clone_from(&self.native_text);

        let hlist_stack = take(&mut st.linebreak.hlist_stack);
        st.linebreak = LinebreakState {
            hlist_stack,
            ..self.linebreak.clone()
        };
        st.linebreak.hlist_stack.clone_from(&self.hlist_stack);

        st.texmf.clone_from(&self.texmf);
        st.synctex.clone_from(&self.synctex);
        st.mainloop.clone_from(&self.mainloop);
        self.mem.restore(&mut st.mem);
        self.eqtb.restore(&mut st.eqtb);
        self.strings.restore(&mut st.strings);
        self.save.restore(&mut st.save_stack);
        self.input.restore(&mut st.input);
        self.fonts.restore(&mut st.fonts);
        self.hyph.restore(&mut st.hyph);
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        vec_bytes(&self.dvi_buf)
            + self
                .shipout
                .write_name
                .iter()
                .flatten()
                .map(vec_bytes)
                .sum::<usize>()
            + self
                .shipout
                .write_prefix
                .iter()
                .map(vec_bytes)
                .sum::<usize>()
            + vec_bytes(&self.native_text)
            + vec_bytes(&self.hlist_stack)
            + self.mem.allocated_bytes()
            + self.strings.allocated_bytes()
            + self.save.allocated_bytes()
            + self.input.allocated_bytes()
            + self.fonts.allocated_bytes()
            + self.hyph.private_allocated_bytes()
    }

    pub(crate) fn shared_hyph_tables(&self) -> (*const HyphTables, usize) {
        (
            Arc::as_ptr(&self.hyph.tables),
            self.hyph.tables.allocated_bytes(),
        )
    }
}

struct MemBundle {
    lo_mem_max: i32,
    hi_mem_min: i32,
    avail: i32,
    mem_end: i32,
    rover: i32,
    var_used: i32,
    dyn_used: i32,
    line: i32,
    cur_synctex_tag: i32,
    temp_ptr: MemPtr,
    glyph_blobs: crate::glyph_arena::GlyphBlobArena,
}

impl MemBundle {
    fn capture(src: &MemState) -> Self {
        let MemState {
            data: _,
            glyph_blobs,
            journal: _,
            lo_mem_max,
            hi_mem_min,
            avail,
            mem_end,
            rover,
            var_used,
            dyn_used,
            line,
            cur_synctex_tag,
            temp_ptr,
        } = src;
        Self {
            lo_mem_max: *lo_mem_max,
            hi_mem_min: *hi_mem_min,
            avail: *avail,
            mem_end: *mem_end,
            rover: *rover,
            var_used: *var_used,
            dyn_used: *dyn_used,
            line: *line,
            cur_synctex_tag: *cur_synctex_tag,
            temp_ptr: *temp_ptr,
            glyph_blobs: glyph_blobs.clone(),
        }
    }

    fn restore(&self, dst: &mut MemState) {
        dst.lo_mem_max = self.lo_mem_max;
        dst.hi_mem_min = self.hi_mem_min;
        dst.avail = self.avail;
        dst.mem_end = self.mem_end;
        dst.rover = self.rover;
        dst.var_used = self.var_used;
        dst.dyn_used = self.dyn_used;
        dst.line = self.line;
        dst.cur_synctex_tag = self.cur_synctex_tag;
        dst.temp_ptr = self.temp_ptr;
        dst.glyph_blobs.restore_from(&self.glyph_blobs);
    }

    fn allocated_bytes(&self) -> usize {
        self.glyph_blobs.allocated_bytes()
    }
}

struct EqtbBundle {
    hash_used: i32,
    hash_high: i32,
    no_new_control_sequence: bool,
    prim_used: i32,
    cs_count: i32,
    hash_extra: i32,
    eqtb_top: i32,
    hash_top: i32,
}

impl EqtbBundle {
    fn capture(src: &EqtbState) -> Self {
        let EqtbState {
            hash_used,
            hash_high,
            no_new_control_sequence,
            prim_used,
            cs_count,
            data: _,
            prim: _,
            xeq_level: _,
            hash: _,
            hash_extra,
            eqtb_top,
            hash_top,
            journal: _,
        } = src;
        Self {
            hash_used: *hash_used,
            hash_high: *hash_high,
            no_new_control_sequence: *no_new_control_sequence,
            prim_used: *prim_used,
            cs_count: *cs_count,
            hash_extra: *hash_extra,
            eqtb_top: *eqtb_top,
            hash_top: *hash_top,
        }
    }

    fn restore(&self, dst: &mut EqtbState) {
        dst.hash_used = self.hash_used;
        dst.hash_high = self.hash_high;
        dst.no_new_control_sequence = self.no_new_control_sequence;
        dst.prim_used = self.prim_used;
        dst.cs_count = self.cs_count;
        dst.hash_extra = self.hash_extra;
        dst.eqtb_top = self.eqtb_top;
        dst.hash_top = self.hash_top;
    }
}

struct StringsBundle {
    base: StringWatermark,
    str_ptr: i32,
    max_strings: i32,
    init_str_ptr: i32,
    init_pool_ptr: i32,
    job_name: StrNumber,
    texmf_log_name: StrNumber,
    string_vacancies: i32,
    pool_size: i32,
    pool_ptr: i32,
    format_ident: StrNumber,
    edit_name_start: i32,
    pool_span: Vec<u16>,
    start_span: Vec<i32>,
}

impl StringsBundle {
    fn capture(src: &StringsState, base: StringWatermark) -> Self {
        let top = StringWatermark::of(src);
        let StringsState {
            str_ptr,
            max_strings,
            init_str_ptr,
            init_pool_ptr,
            job_name,
            texmf_log_name,
            string_vacancies,
            pool_size,
            pool_ptr,
            format_ident,
            edit_name_start,
            str_start,
            str_pool,
        } = src;
        assert!(
            top.pool >= base.pool,
            "string pool dipped below checkpoint base"
        );
        assert!(
            top.starts >= base.starts,
            "string table dipped below checkpoint base"
        );
        Self {
            base,
            str_ptr: *str_ptr,
            max_strings: *max_strings,
            init_str_ptr: *init_str_ptr,
            init_pool_ptr: *init_pool_ptr,
            job_name: *job_name,
            texmf_log_name: *texmf_log_name,
            string_vacancies: *string_vacancies,
            pool_size: *pool_size,
            pool_ptr: *pool_ptr,
            format_ident: *format_ident,
            edit_name_start: *edit_name_start,
            pool_span: str_pool[base.pool..top.pool].to_vec(),
            start_span: if base.starts < str_start.len() {
                str_start[base.starts..=top.starts.min(str_start.len() - 1)].to_vec()
            } else {
                Vec::new()
            },
        }
    }

    fn restore(&self, dst: &mut StringsState) {
        dst.str_ptr = self.str_ptr;
        dst.max_strings = self.max_strings;
        dst.init_str_ptr = self.init_str_ptr;
        dst.init_pool_ptr = self.init_pool_ptr;
        dst.job_name = self.job_name;
        dst.texmf_log_name = self.texmf_log_name;
        dst.string_vacancies = self.string_vacancies;
        dst.pool_size = self.pool_size;
        dst.pool_ptr = self.pool_ptr;
        dst.format_ident = self.format_ident;
        dst.edit_name_start = self.edit_name_start;
        dst.str_pool[self.base.pool..][..self.pool_span.len()].copy_from_slice(&self.pool_span);
        dst.str_start[self.base.starts..][..self.start_span.len()]
            .copy_from_slice(&self.start_span);
    }

    fn allocated_bytes(&self) -> usize {
        vec_bytes(&self.pool_span) + vec_bytes(&self.start_span)
    }
}

struct SaveBundle {
    save_ptr: i32,
    max_save_stack: i32,
    save_size: i32,
    cur_level: u16,
    cur_group: GroupCode,
    cur_boundary: i32,
    data: Vec<MemoryWord>,
}

impl SaveBundle {
    fn capture(src: &SaveStackState) -> Self {
        let end = src.save_ptr.max(0) as usize;
        Self {
            save_ptr: src.save_ptr,
            max_save_stack: src.max_save_stack,
            save_size: src.save_size,
            cur_level: src.cur_level,
            cur_group: src.cur_group,
            cur_boundary: src.cur_boundary,
            data: src.data[..end].to_vec(),
        }
    }

    fn restore(&self, dst: &mut SaveStackState) {
        dst.save_ptr = self.save_ptr;
        dst.max_save_stack = self.max_save_stack;
        dst.save_size = self.save_size;
        dst.cur_level = self.cur_level;
        dst.cur_group = self.cur_group;
        dst.cur_boundary = self.cur_boundary;
        dst.data[..self.data.len()].copy_from_slice(&self.data);
    }

    fn allocated_bytes(&self) -> usize {
        vec_bytes(&self.data)
    }
}

/// Open input and read slots are operation resources that resume replay
/// rebuilds, so a rung never owns a live UFile.
struct InputBundle {
    buffer: Vec<i32>,
    input_stack: Vec<InputState>,
    param_stack: Vec<crate::units::MemPtr>,
    nest: Vec<ListStateRecord>,
    line_stack: Vec<i32>,
    grp_stack: Vec<i32>,
    if_stack: Vec<MemPtr>,
    source_filename_stack: Vec<StrNumber>,
    full_source_filename_stack: Vec<StrNumber>,
    eof_seen: Vec<bool>,
    cur_input: InputState,
    cur_list: ListStateRecord,
    open_parens: i32,
    in_open: i32,
    pseudo_files: MemPtr,
    nest_ptr: i32,
    max_nest_stack: i32,
    input_ptr: i32,
    max_in_stack: i32,
    base_ptr: i32,
    par_loc: i32,
    par_token: crate::token::Token,
    param_ptr: i32,
    max_param_stack: i32,
    force_eof: bool,
    first: i32,
    last: i32,
    max_buf_stack: i32,
    read_open: [u8; 17],
    name_of_file: Vec<u8>,
    name_of_input_file: Vec<u8>,
    abspath_of_input_file: Vec<u8>,
}

fn live_prefix<T: Clone>(values: &[T], len: i32) -> Vec<T> {
    values[..(len.max(0) as usize).min(values.len())].to_vec()
}

impl InputBundle {
    fn capture(src: &InputClusterState) -> Self {
        Self {
            buffer: live_prefix(&src.buffer, src.buffer_in_use()),
            input_stack: live_prefix(&src.input_stack, src.input_ptr + 1),
            param_stack: live_prefix(&src.param_stack, src.param_ptr),
            nest: live_prefix(&src.nest, src.nest_ptr + 1),
            line_stack: src.line_stack.clone(),
            grp_stack: src.grp_stack.clone(),
            if_stack: src.if_stack.clone(),
            source_filename_stack: src.source_filename_stack.clone(),
            full_source_filename_stack: src.full_source_filename_stack.clone(),
            eof_seen: src.eof_seen.clone(),
            cur_input: src.cur_input,
            cur_list: src.cur_list,
            open_parens: src.open_parens,
            in_open: src.in_open,
            pseudo_files: src.pseudo_files,
            nest_ptr: src.nest_ptr,
            max_nest_stack: src.max_nest_stack,
            input_ptr: src.input_ptr,
            max_in_stack: src.max_in_stack,
            base_ptr: src.base_ptr,
            par_loc: src.par_loc,
            par_token: src.par_token,
            param_ptr: src.param_ptr,
            max_param_stack: src.max_param_stack,
            force_eof: src.force_eof,
            first: src.first,
            last: src.last,
            max_buf_stack: src.max_buf_stack,
            read_open: src.read_open,
            name_of_file: src.name_of_file.clone(),
            name_of_input_file: src.name_of_input_file.clone(),
            abspath_of_input_file: src.abspath_of_input_file.clone(),
        }
    }

    fn restore(&self, dst: &mut InputClusterState) {
        dst.buffer[..self.buffer.len()].copy_from_slice(&self.buffer);
        dst.input_stack[..self.input_stack.len()].copy_from_slice(&self.input_stack);
        dst.param_stack[..self.param_stack.len()].copy_from_slice(&self.param_stack);
        dst.nest[..self.nest.len()].copy_from_slice(&self.nest);
        dst.line_stack.clone_from(&self.line_stack);
        dst.grp_stack.clone_from(&self.grp_stack);
        dst.if_stack.clone_from(&self.if_stack);
        dst.source_filename_stack
            .clone_from(&self.source_filename_stack);
        dst.full_source_filename_stack
            .clone_from(&self.full_source_filename_stack);
        dst.eof_seen.clone_from(&self.eof_seen);
        dst.cur_input = self.cur_input;
        dst.cur_list = self.cur_list;
        dst.open_parens = self.open_parens;
        dst.in_open = self.in_open;
        dst.pseudo_files = self.pseudo_files;
        dst.nest_ptr = self.nest_ptr;
        dst.max_nest_stack = self.max_nest_stack;
        dst.input_ptr = self.input_ptr;
        dst.max_in_stack = self.max_in_stack;
        dst.base_ptr = self.base_ptr;
        dst.par_loc = self.par_loc;
        dst.par_token = self.par_token;
        dst.param_ptr = self.param_ptr;
        dst.max_param_stack = self.max_param_stack;
        dst.force_eof = self.force_eof;
        dst.first = self.first;
        dst.last = self.last;
        dst.max_buf_stack = self.max_buf_stack;
        dst.read_open = self.read_open;
        dst.name_of_file.clone_from(&self.name_of_file);
        dst.name_of_input_file.clone_from(&self.name_of_input_file);
        dst.abspath_of_input_file
            .clone_from(&self.abspath_of_input_file);
    }

    fn allocated_bytes(&self) -> usize {
        vec_bytes(&self.buffer)
            + vec_bytes(&self.input_stack)
            + vec_bytes(&self.param_stack)
            + vec_bytes(&self.nest)
            + vec_bytes(&self.line_stack)
            + vec_bytes(&self.grp_stack)
            + vec_bytes(&self.if_stack)
            + vec_bytes(&self.source_filename_stack)
            + vec_bytes(&self.full_source_filename_stack)
            + vec_bytes(&self.eof_seen)
            + vec_bytes(&self.name_of_file)
    }
}

macro_rules! font_arrays {
    ($($field:ident: $ty:ty,)*) => {
        /// Per-font `Copy` tables, captured through `font_ptr`.
        struct FontArrays {
            $($field: Vec<$ty>,)*
        }

        impl FontArrays {
            fn capture(src: &FontState, count: i32) -> Self {
                Self {
                    $($field: live_prefix(src.$field.as_slice(), count),)*
                }
            }

            fn restore(&self, dst: &mut FontState) {
                $(dst.$field.as_mut_slice()[..self.$field.len()].copy_from_slice(&self.$field);)*
            }

            fn allocated_bytes(&self) -> usize {
                0 $(+ vec_bytes(&self.$field))*
            }
        }
    };
}

font_arrays! {
    font_flags: i8,
    font_letter_space: Scaled,
    font_check: B16x4,
    font_size: Scaled,
    font_dsize: Scaled,
    font_params: i32,
    font_name: StrNumber,
    font_area: StrNumber,
    font_bc: u16,
    font_ec: u16,
    bchar_label: i32,
    font_bchar: i32,
    font_false_bchar: i32,
    font_glue: MemPtr,
    font_used: bool,
    char_base: i32,
    width_base: i32,
    height_base: i32,
    depth_base: i32,
    italic_base: i32,
    lig_kern_base: i32,
    kern_base: i32,
    exten_base: i32,
    param_base: i32,
    hyphen_char: i32,
    skew_char: i32,
}

struct FontBundle {
    loaded_font_mapping: Option<Arc<FontMappingHandle>>,
    loaded_font_flags: i8,
    saved_mapping_name: Option<Vec<u8>>,
    loaded_font_letter_space: Scaled,
    xtx_ligature_present: bool,
    gave_char_warning_help: bool,
    native_font_type_flag: StrNumber,
    font_mem_size: i32,
    font_max: FontId,
    fmem_ptr: i32,
    font_ptr: FontId,
    arrays: FontArrays,
    engines: Vec<Option<Arc<FontEngineHandle>>>,
    mappings: Vec<Option<Arc<FontMappingHandle>>>,
    mapped_text: Vec<u16>,
}

impl FontBundle {
    fn capture(src: &FontState) -> Self {
        let FontState {
            loaded_font_mapping,
            loaded_font_flags,
            saved_mapping_name,
            brk_iter: _,
            brk_locale_str_num: _,
            font_layout_engine,
            font_mapping,
            loaded_font_letter_space,
            xtx_ligature_present,
            gave_char_warning_help,
            native_font_type_flag,
            font_mem_size,
            font_max,
            fmem_ptr,
            font_ptr,
            font_flags: _,
            font_letter_space: _,
            font_check: _,
            font_size: _,
            font_dsize: _,
            font_params: _,
            font_name: _,
            font_area: _,
            font_bc: _,
            font_ec: _,
            bchar_label: _,
            font_bchar: _,
            font_false_bchar: _,
            font_glue: _,
            font_used: _,
            char_base: _,
            width_base: _,
            height_base: _,
            depth_base: _,
            italic_base: _,
            lig_kern_base: _,
            kern_base: _,
            exten_base: _,
            param_base: _,
            hyphen_char: _,
            skew_char: _,
            font_info: _,
            font_info_journal: _,
            fmem_epoch_base: _,
            mapped_text,
        } = src;
        let count = font_ptr.0 + 1;
        Self {
            loaded_font_mapping: loaded_font_mapping.clone(),
            loaded_font_flags: *loaded_font_flags,
            saved_mapping_name: saved_mapping_name.clone(),
            loaded_font_letter_space: *loaded_font_letter_space,
            xtx_ligature_present: *xtx_ligature_present,
            gave_char_warning_help: *gave_char_warning_help,
            native_font_type_flag: *native_font_type_flag,
            font_mem_size: *font_mem_size,
            font_max: *font_max,
            fmem_ptr: *fmem_ptr,
            font_ptr: *font_ptr,
            arrays: FontArrays::capture(src, count),
            engines: live_prefix(font_layout_engine.as_slice(), count),
            mappings: live_prefix(font_mapping.as_slice(), count),
            mapped_text: mapped_text.clone(),
        }
    }

    fn restore(&self, dst: &mut FontState) {
        for f in (self.font_ptr.0 + 1..=dst.font_ptr.0).map(FontId) {
            dst.font_layout_engine[f] = None;
            dst.font_mapping[f] = None;
        }
        dst.font_layout_engine.as_mut_slice()[..self.engines.len()].clone_from_slice(&self.engines);
        dst.font_mapping.as_mut_slice()[..self.mappings.len()].clone_from_slice(&self.mappings);
        // The ICU iterator points into transient text and is rebuilt lazily.
        dst.release_linebreak_state();
        // Slots reused after the checkpoint may have emitted font definitions
        // in the abandoned timeline, so they must start unused.
        let restored_font_count = (self.font_ptr.0 + 1).max(0) as usize;
        if let Some(abandoned) = dst.font_used.as_mut_slice().get_mut(restored_font_count..) {
            abandoned.fill(false);
        }
        dst.loaded_font_mapping
            .clone_from(&self.loaded_font_mapping);
        dst.loaded_font_flags = self.loaded_font_flags;
        dst.saved_mapping_name.clone_from(&self.saved_mapping_name);
        dst.loaded_font_letter_space = self.loaded_font_letter_space;
        dst.xtx_ligature_present = self.xtx_ligature_present;
        dst.gave_char_warning_help = self.gave_char_warning_help;
        dst.native_font_type_flag = self.native_font_type_flag;
        dst.font_mem_size = self.font_mem_size;
        dst.font_max = self.font_max;
        dst.fmem_ptr = self.fmem_ptr;
        dst.font_ptr = self.font_ptr;
        self.arrays.restore(dst);
        dst.mapped_text.clone_from(&self.mapped_text);
    }

    fn allocated_bytes(&self) -> usize {
        self.arrays.allocated_bytes()
            + vec_bytes(&self.engines)
            + vec_bytes(&self.mappings)
            + vec_bytes(&self.mapped_text)
    }
}

macro_rules! hyph_tables {
    ($($field:ident: $ty:ty,)*) => {
        /// Pattern and exception tables, shared between rungs until
        /// `tables_gen` changes.
        pub(crate) struct HyphTables {
            $($field: Vec<$ty>,)*
        }

        impl HyphTables {
            fn capture(src: &HyphState) -> Self {
                Self {
                    $($field: src.$field.clone(),)*
                }
            }

            fn restore(&self, dst: &mut HyphState) {
                $(dst.$field.clone_from(&self.$field);)*
            }

            fn allocated_bytes(&self) -> usize {
                0 $(+ vec_bytes(&self.$field))*
            }
        }
    };
}

hyph_tables! {
    trie_min: i32,
    hyf_distance: i16,
    hyf_num: i16,
    hyf_next: u16,
    op_start: i32,
    trie_used: u16,
    trie_op_lang: u8,
    trie_op_val: u16,
    trie_trl: i32,
    trie_tro: i32,
    trie_trc: u16,
    trie_c: u16,
    trie_o: u16,
    trie_l: i32,
    trie_r: i32,
    trie_hash: i32,
    trie_taken: bool,
    hyph_word: StrNumber,
    hyph_list: i32,
    hyph_link: u16,
    trie_op_hash_array: i32,
}

struct HyphBundle {
    tables_gen: u64,
    is_hyph: bool,
    hf: FontId,
    max_hyph_char: i32,
    init_list: MemPtr,
    init_lig: bool,
    init_lft: bool,
    hyphen_passed: i16,
    trie_max: i32,
    trie_size: i32,
    trie_ptr: i32,
    hyph_count: i32,
    hyph_size: i32,
    hyph_next: i32,
    trie_op_ptr: i32,
    max_op_used: u16,
    trie_not_ready: bool,
    hyph_start: i32,
    hyph_index: i32,
    cur_lang: u8,
    hyf_char: i32,
    l_hyf: i32,
    r_hyf: i32,
    init_l_hyf: i32,
    init_r_hyf: i32,
    hyf_bchar: i32,
    hc: Vec<i32>,
    hu: Vec<i32>,
    hyf: Vec<u8>,
    tables: Arc<HyphTables>,
}

impl HyphBundle {
    fn capture(src: &HyphState, previous: Option<&Self>) -> Self {
        let tables = match previous.filter(|p| p.tables_gen == src.tables_gen) {
            Some(previous) => Arc::clone(&previous.tables),
            None => Arc::new(HyphTables::capture(src)),
        };
        Self {
            tables_gen: src.tables_gen,
            is_hyph: src.is_hyph,
            hf: src.hf,
            max_hyph_char: src.max_hyph_char,
            init_list: src.init_list,
            init_lig: src.init_lig,
            init_lft: src.init_lft,
            hyphen_passed: src.hyphen_passed,
            trie_max: src.trie_max,
            trie_size: src.trie_size,
            trie_ptr: src.trie_ptr,
            hyph_count: src.hyph_count,
            hyph_size: src.hyph_size,
            hyph_next: src.hyph_next,
            trie_op_ptr: src.trie_op_ptr,
            max_op_used: src.max_op_used,
            trie_not_ready: src.trie_not_ready,
            hyph_start: src.hyph_start,
            hyph_index: src.hyph_index,
            cur_lang: src.cur_lang,
            hyf_char: src.hyf_char,
            l_hyf: src.l_hyf,
            r_hyf: src.r_hyf,
            init_l_hyf: src.init_l_hyf,
            init_r_hyf: src.init_r_hyf,
            hyf_bchar: src.hyf_bchar,
            hc: src.hc.clone(),
            hu: src.hu.clone(),
            hyf: src.hyf.clone(),
            tables,
        }
    }

    fn restore(&self, dst: &mut HyphState) {
        dst.tables_gen = self.tables_gen;
        dst.is_hyph = self.is_hyph;
        dst.hf = self.hf;
        dst.max_hyph_char = self.max_hyph_char;
        dst.init_list = self.init_list;
        dst.init_lig = self.init_lig;
        dst.init_lft = self.init_lft;
        dst.hyphen_passed = self.hyphen_passed;
        dst.trie_max = self.trie_max;
        dst.trie_size = self.trie_size;
        dst.trie_ptr = self.trie_ptr;
        dst.hyph_count = self.hyph_count;
        dst.hyph_size = self.hyph_size;
        dst.hyph_next = self.hyph_next;
        dst.trie_op_ptr = self.trie_op_ptr;
        dst.max_op_used = self.max_op_used;
        dst.trie_not_ready = self.trie_not_ready;
        dst.hyph_start = self.hyph_start;
        dst.hyph_index = self.hyph_index;
        dst.cur_lang = self.cur_lang;
        dst.hyf_char = self.hyf_char;
        dst.l_hyf = self.l_hyf;
        dst.r_hyf = self.r_hyf;
        dst.init_l_hyf = self.init_l_hyf;
        dst.init_r_hyf = self.init_r_hyf;
        dst.hyf_bchar = self.hyf_bchar;
        dst.hc.clone_from(&self.hc);
        dst.hu.clone_from(&self.hu);
        dst.hyf.clone_from(&self.hyf);
        self.tables.restore(dst);
    }

    fn private_allocated_bytes(&self) -> usize {
        vec_bytes(&self.hc) + vec_bytes(&self.hu) + vec_bytes(&self.hyf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph_arena::{FixedPoint, GlyphBlob};

    fn state() -> EngineState {
        let mut st = EngineState::default();
        st.strings.str_pool = vec![0; 16];
        st.strings.str_start = vec![0; 8];
        st.input.buffer = vec![0; 8];
        st.input.input_stack = vec![crate::state::InputState::default(); 4];
        st.input.param_stack = vec![crate::units::MemPtr::default(); 4];
        st.input.nest = vec![crate::mem_types::ListStateRecord::default(); 4];
        st.save_stack.data = crate::mem_types::zeroed_vec(4);
        st
    }

    #[test]
    fn bundle_restores_non_journaled_state_without_moving_live_buffers() {
        let mut st = state();
        let base = StringWatermark::capture(&st);
        st.scanner.logical_clock = 7;
        st.scanner.native_text = vec![1, 2, 3];
        st.shipout.total_pages = 2;
        st.shipout.dvi.buf = vec![4, 5, 6];
        st.strings.str_pool[0] = 11;
        st.strings.pool_ptr = 1;
        st.save_stack.save_ptr = 1;
        st.save_stack.set_saved(-1, 12);
        st.input.buffer[0] = 13;
        st.input.last = 1;
        st.hyph.hc[0] = 14;
        let glyph = st.mem.glyph_blobs.insert(GlyphBlob {
            locations: vec![FixedPoint {
                x: Scaled(15),
                y: Scaled::ZERO,
            }],
            gids: vec![16],
        });
        let bundle = EngineBundle::capture(&st, base, None);

        let scanner_ptr = st.scanner.native_text.as_ptr();
        let dvi_ptr = st.shipout.dvi.buf.as_ptr();
        st.scanner.logical_clock = 70;
        st.scanner.native_text[0] = 70;
        st.shipout.total_pages = 20;
        st.shipout.dvi.buf[0] = 40;
        st.shipout.dvi.manifest.begin_operation();
        st.shipout.dvi.manifest.begin_page(0);
        st.shipout.dvi.manifest.record_byte(139);
        st.strings.str_pool[0] = 110;
        st.save_stack.set_saved(-1, 120);
        st.input.buffer[0] = 130;
        st.hyph.hc[0] = 140;
        st.mem.glyph_blobs.get_mut(glyph).locations[0].x = Scaled(150);
        st.input.input_file.push(crate::state::InputSlot::Terminal);

        bundle.restore(&mut st);
        assert_eq!(st.scanner.logical_clock, 7);
        assert_eq!(st.scanner.native_text, [1, 2, 3]);
        assert_eq!(st.scanner.native_text.as_ptr(), scanner_ptr);
        assert_eq!(st.shipout.total_pages, 2);
        assert_eq!(st.shipout.dvi.buf, [4, 5, 6]);
        assert_eq!(st.shipout.dvi.buf.as_ptr(), dvi_ptr);
        st.shipout.dvi.manifest.record_byte(140);
        st.shipout.dvi.manifest.finish_page(8);
        assert_eq!(st.shipout.dvi.manifest.take_pages().len(), 1);
        assert_eq!(st.strings.str_pool[0], 11);
        assert_eq!(st.save_stack.save_index(0), 12);
        assert_eq!(st.input.buffer[0], 13);
        assert_eq!(st.hyph.hc[0], 14);
        assert_eq!(st.mem.glyph_blobs.get(glyph).locations[0].x, Scaled(15));
        assert!(matches!(
            st.input.input_file.last(),
            Some(crate::state::InputSlot::Terminal)
        ));
    }

    #[test]
    fn font_restore_clears_use_flags_beyond_the_checkpoint() {
        let mut fonts = FontState {
            font_layout_engine: crate::state::PerFont::filled(4, None),
            font_mapping: crate::state::PerFont::filled(4, None),
            font_used: crate::state::PerFont::filled(4, false),
            font_ptr: FontId(1),
            ..Default::default()
        };
        let checkpoint = FontBundle::capture(&fonts);

        fonts.font_ptr = FontId(3);
        fonts.font_used[FontId(2)] = true;
        fonts.font_used[FontId(3)] = true;
        checkpoint.restore(&mut fonts);

        assert_eq!(fonts.font_ptr, FontId(1));
        assert_eq!(fonts.font_used.as_slice(), [false, false, false, false]);
    }

    #[test]
    fn unchanged_hyphen_tables_are_shared_between_rungs() {
        let st = state();
        let base = StringWatermark::capture(&st);
        let first = EngineBundle::capture(&st, base, None);
        let second = EngineBundle::capture(&st, base, Some(&first));
        assert_eq!(first.shared_hyph_tables().0, second.shared_hyph_tables().0);

        let mut changed = st;
        changed.hyph.tables_gen += 1;
        let third = EngineBundle::capture(&changed, base, Some(&second));
        assert_ne!(second.shared_hyph_tables().0, third.shared_hyph_tables().0);
        assert!(third.allocated_bytes() > 0);
    }
}
