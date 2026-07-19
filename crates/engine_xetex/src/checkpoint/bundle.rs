//! Non-journaled state carried by a checkpoint rung.

use std::mem::{size_of, take};
use std::sync::Arc;

use crate::mem_types::{B16x4, MemoryWord};
use crate::state::{EngineState, GroupCode};

#[derive(Clone, Copy)]
pub(crate) struct StringWatermark {
    pool: usize,
    starts: usize,
}

impl StringWatermark {
    pub(crate) fn capture(st: &EngineState) -> Self {
        Self {
            pool: st.strings.pool_ptr.max(0) as usize,
            starts: (st.strings.str_ptr - crate::consts::TOO_BIG_CHAR).max(0) as usize,
        }
    }
}

pub(crate) struct EngineBundle {
    config: crate::state::ConfigState,
    errors: crate::state::ErrorState,
    shipout: crate::state::ShipoutState,
    scanner: crate::state::ScannerState,
    linebreak: crate::state::LinebreakState,
    texmf: crate::state::TexmfState,
    synctex: crate::state::SynctexCtxt,
    mainloop: crate::state::MainLoopState,
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
        Self {
            config: st.config,
            errors: st.errors.clone(),
            shipout: {
                let mut shipout = st.shipout.clone();
                // Published page facts belong to an operation, not replayable
                // engine state. Output marks will travel separately with rungs.
                shipout.page_manifest = Default::default();
                shipout
            },
            scanner: st.scanner.clone(),
            linebreak: st.linebreak.clone(),
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
        restore_shipout(&mut st.shipout, &self.shipout);
        restore_scanner(&mut st.scanner, &self.scanner);
        restore_linebreak(&mut st.linebreak, &self.linebreak);
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
        self.shipout.dvi_buf.capacity()
            + self
                .shipout
                .write_name
                .iter()
                .map(Vec::capacity)
                .sum::<usize>()
            + self
                .shipout
                .write_prefix
                .iter()
                .map(|prefix| prefix.capacity() * size_of::<i32>())
                .sum::<usize>()
            + self.scanner.native_text.capacity() * size_of::<u16>()
            + self.linebreak.hlist_stack.capacity() * size_of::<crate::units::MemPtr>()
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

fn restore_shipout(dst: &mut crate::state::ShipoutState, src: &crate::state::ShipoutState) {
    let mut buffer = take(&mut dst.dvi_buf);
    let manifest = take(&mut dst.page_manifest);
    let captured = &src.dvi_buf;
    *dst = src.clone();
    buffer.clone_from(captured);
    dst.dvi_buf = buffer;
    dst.page_manifest = manifest;
}

fn restore_scanner(dst: &mut crate::state::ScannerState, src: &crate::state::ScannerState) {
    let mut native_text = take(&mut dst.native_text);
    let captured = &src.native_text;
    *dst = src.clone();
    native_text.clone_from(captured);
    dst.native_text = native_text;
}

fn restore_linebreak(dst: &mut crate::state::LinebreakState, src: &crate::state::LinebreakState) {
    let mut stack = take(&mut dst.hlist_stack);
    let captured = &src.hlist_stack;
    *dst = src.clone();
    stack.clone_from(captured);
    dst.hlist_stack = stack;
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
    temp_ptr: crate::units::MemPtr,
    glyph_blobs: crate::glyph_arena::GlyphBlobArena,
}

impl MemBundle {
    fn capture(src: &crate::state::MemState) -> Self {
        let crate::state::MemState {
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

    fn restore(&self, dst: &mut crate::state::MemState) {
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
    fn capture(src: &crate::state::EqtbState) -> Self {
        let crate::state::EqtbState {
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

    fn restore(&self, dst: &mut crate::state::EqtbState) {
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
    job_name: i32,
    texmf_log_name: i32,
    string_vacancies: i32,
    pool_size: i32,
    pool_ptr: i32,
    format_ident: i32,
    edit_name_start: i32,
    pool_span: Vec<u16>,
    start_span: Vec<i32>,
}

impl StringsBundle {
    fn capture(src: &crate::state::StringsState, base: StringWatermark) -> Self {
        let crate::state::StringsState {
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
        let pool_top = (*pool_ptr).max(0) as usize;
        let start_top = (*str_ptr - crate::consts::TOO_BIG_CHAR).max(0) as usize;
        assert!(
            pool_top >= base.pool,
            "string pool dipped below checkpoint base"
        );
        assert!(
            start_top >= base.starts,
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
            pool_span: str_pool[base.pool..pool_top].to_vec(),
            start_span: if base.starts < str_start.len() {
                str_start[base.starts..=start_top.min(str_start.len() - 1)].to_vec()
            } else {
                Vec::new()
            },
        }
    }

    fn restore(&self, dst: &mut crate::state::StringsState) {
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
        dst.str_pool[self.base.pool..self.base.pool + self.pool_span.len()]
            .copy_from_slice(&self.pool_span);
        dst.str_start[self.base.starts..self.base.starts + self.start_span.len()]
            .copy_from_slice(&self.start_span);
    }

    fn allocated_bytes(&self) -> usize {
        self.pool_span.capacity() * size_of::<u16>() + self.start_span.capacity() * size_of::<i32>()
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
    fn capture(src: &crate::state::SaveStackState) -> Self {
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

    fn restore(&self, dst: &mut crate::state::SaveStackState) {
        dst.save_ptr = self.save_ptr;
        dst.max_save_stack = self.max_save_stack;
        dst.save_size = self.save_size;
        dst.cur_level = self.cur_level;
        dst.cur_group = self.cur_group;
        dst.cur_boundary = self.cur_boundary;
        dst.data[..self.data.len()].copy_from_slice(&self.data);
    }

    fn allocated_bytes(&self) -> usize {
        self.data.capacity() * size_of::<MemoryWord>()
    }
}

struct InputBundle {
    buffer: Vec<i32>,
    input_stack: Vec<crate::globals::InputState>,
    param_stack: Vec<i32>,
    nest: Vec<crate::mem_types::ListStateRecord>,
    line_stack: Vec<i32>,
    grp_stack: Vec<i32>,
    if_stack: Vec<crate::units::MemPtr>,
    source_filename_stack: Vec<i32>,
    full_source_filename_stack: Vec<i32>,
    eof_seen: Vec<bool>,
    cur_input: crate::globals::InputState,
    cur_list: crate::mem_types::ListStateRecord,
    open_parens: i32,
    in_open: i32,
    pseudo_files: crate::units::MemPtr,
    nest_ptr: i32,
    max_nest_stack: i32,
    input_ptr: i32,
    max_in_stack: i32,
    base_ptr: i32,
    par_loc: i32,
    par_token: i32,
    param_ptr: i32,
    max_param_stack: i32,
    force_eof: bool,
    first: i32,
    last: i32,
    max_buf_stack: i32,
    name_length: i32,
    name_length16: i32,
    read_open: [u8; 17],
    name_of_file: Vec<libc::c_char>,
    name_of_file16: Vec<u16>,
    name_of_input_file: Option<std::ffi::CString>,
    abspath_of_input_file: crate::state::AbsPathBuf,
}

impl InputBundle {
    fn capture(src: &crate::state::InputClusterState) -> Self {
        let buffer_end = (src.max_buf_stack.max(src.last).max(src.first) + 1).max(0) as usize;
        let input_end = (src.input_ptr + 1).max(0) as usize;
        let nest_end = (src.nest_ptr + 1).max(0) as usize;
        let param_end = src.param_ptr.max(0) as usize;
        Self {
            buffer: src.buffer[..buffer_end.min(src.buffer.len())].to_vec(),
            input_stack: src.input_stack[..input_end.min(src.input_stack.len())].to_vec(),
            param_stack: src.param_stack[..param_end.min(src.param_stack.len())].to_vec(),
            nest: src.nest[..nest_end.min(src.nest.len())].to_vec(),
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
            name_length: src.name_length,
            name_length16: src.name_length16,
            read_open: src.read_open,
            name_of_file: src.name_of_file.clone(),
            name_of_file16: src.name_of_file16.clone(),
            name_of_input_file: src.name_of_input_file.clone(),
            abspath_of_input_file: src.abspath_of_input_file,
        }
    }

    fn restore(&self, dst: &mut crate::state::InputClusterState) {
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
        dst.name_length = self.name_length;
        dst.name_length16 = self.name_length16;
        dst.read_open = self.read_open;
        dst.name_of_file.clone_from(&self.name_of_file);
        dst.name_of_file16.clone_from(&self.name_of_file16);
        dst.name_of_input_file.clone_from(&self.name_of_input_file);
        dst.abspath_of_input_file = self.abspath_of_input_file;
        // Open input/read slots are operation resources. Resume replay rebuilds
        // them; a rung never clones or assumes ownership of a live UFile.
    }

    fn allocated_bytes(&self) -> usize {
        self.buffer.capacity() * size_of::<i32>()
            + self.input_stack.capacity() * size_of::<crate::globals::InputState>()
            + self.param_stack.capacity() * size_of::<i32>()
            + self.nest.capacity() * size_of::<crate::mem_types::ListStateRecord>()
            + self.line_stack.capacity() * size_of::<i32>()
            + self.grp_stack.capacity() * size_of::<i32>()
            + self.if_stack.capacity() * size_of::<crate::units::MemPtr>()
            + self.source_filename_stack.capacity() * size_of::<i32>()
            + self.full_source_filename_stack.capacity() * size_of::<i32>()
            + self.eof_seen.capacity()
            + self.name_of_file.capacity() * size_of::<libc::c_char>()
            + self.name_of_file16.capacity() * size_of::<u16>()
    }
}

struct FontBundle {
    loaded_font_mapping: Option<Arc<crate::state::FontMappingHandle>>,
    loaded_font_flags: libc::c_char,
    saved_mapping_name: Option<std::ffi::CString>,
    loaded_font_letter_space: i32,
    xtx_ligature_present: bool,
    gave_char_warning_help: bool,
    native_font_type_flag: i32,
    font_mem_size: i32,
    font_max: i32,
    fmem_ptr: i32,
    font_ptr: i32,
    null_character: B16x4,
    font_flags: Vec<i8>,
    font_letter_space: Vec<i32>,
    font_check: Vec<B16x4>,
    font_size: Vec<i32>,
    font_dsize: Vec<i32>,
    font_params: Vec<i32>,
    font_name: Vec<i32>,
    font_area: Vec<i32>,
    font_bc: Vec<u16>,
    font_ec: Vec<u16>,
    bchar_label: Vec<i32>,
    font_bchar: Vec<i32>,
    font_false_bchar: Vec<i32>,
    font_glue: Vec<i32>,
    font_used: Vec<bool>,
    char_base: Vec<i32>,
    width_base: Vec<i32>,
    height_base: Vec<i32>,
    depth_base: Vec<i32>,
    italic_base: Vec<i32>,
    lig_kern_base: Vec<i32>,
    kern_base: Vec<i32>,
    exten_base: Vec<i32>,
    param_base: Vec<i32>,
    hyphen_char: Vec<i32>,
    skew_char: Vec<i32>,
    engines: Vec<Option<Arc<crate::state::FontEngineHandle>>>,
    mappings: Vec<Option<Arc<crate::state::FontMappingHandle>>>,
    xdv_buffer: Vec<u8>,
    mapped_text: Vec<u16>,
}

impl FontBundle {
    fn capture(src: &crate::state::FontState) -> Self {
        let crate::state::FontState {
            loaded_font_mapping,
            loaded_font_flags,
            saved_mapping_name,
            brk_iter: _,
            brk_locale_str_num: _,
            font_layout_engine,
            font_layout_engine_mirror: _,
            font_mapping,
            loaded_font_letter_space,
            xtx_ligature_present,
            gave_char_warning_help,
            native_font_type_flag,
            font_mem_size,
            font_max,
            fmem_ptr,
            font_ptr,
            font_flags,
            font_letter_space,
            null_character,
            font_check,
            font_size,
            font_dsize,
            font_params,
            font_name,
            font_area,
            font_bc,
            font_ec,
            bchar_label,
            font_bchar,
            font_false_bchar,
            font_glue,
            font_used,
            char_base,
            width_base,
            height_base,
            depth_base,
            italic_base,
            lig_kern_base,
            kern_base,
            exten_base,
            param_base,
            hyphen_char,
            skew_char,
            font_info: _,
            font_info_journal: _,
            fmem_epoch_base: _,
            xdv_buffer,
            mapped_text,
        } = src;
        let count = (*font_ptr + 1).max(0) as usize;
        macro_rules! live {
            ($field:ident) => {
                $field[..count.min($field.len())].to_vec()
            };
        }
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
            null_character: *null_character,
            font_flags: live!(font_flags),
            font_letter_space: live!(font_letter_space),
            font_check: live!(font_check),
            font_size: live!(font_size),
            font_dsize: live!(font_dsize),
            font_params: live!(font_params),
            font_name: live!(font_name),
            font_area: live!(font_area),
            font_bc: live!(font_bc),
            font_ec: live!(font_ec),
            bchar_label: live!(bchar_label),
            font_bchar: live!(font_bchar),
            font_false_bchar: live!(font_false_bchar),
            font_glue: live!(font_glue),
            font_used: live!(font_used),
            char_base: live!(char_base),
            width_base: live!(width_base),
            height_base: live!(height_base),
            depth_base: live!(depth_base),
            italic_base: live!(italic_base),
            lig_kern_base: live!(lig_kern_base),
            kern_base: live!(kern_base),
            exten_base: live!(exten_base),
            param_base: live!(param_base),
            hyphen_char: live!(hyphen_char),
            skew_char: live!(skew_char),
            engines: font_layout_engine[..count.min(font_layout_engine.len())].to_vec(),
            mappings: font_mapping[..count.min(font_mapping.len())].to_vec(),
            xdv_buffer: xdv_buffer.clone(),
            mapped_text: mapped_text.clone(),
        }
    }

    fn restore(&self, dst: &mut crate::state::FontState) {
        for f in (self.font_ptr + 1)..=dst.font_ptr {
            dst.set_font_layout_engine(f, None);
            dst.set_font_mapping(f, None);
        }
        for (f, engine) in self.engines.iter().enumerate() {
            dst.set_font_layout_engine(f as i32, engine.clone());
        }
        for (f, mapping) in self.mappings.iter().enumerate() {
            dst.set_font_mapping(f as i32, mapping.clone());
        }
        // The ICU iterator points into transient text and is rebuilt lazily.
        unsafe { crate::ext::release_linebreak_state(dst) };
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
        dst.null_character = self.null_character;
        macro_rules! restore {
            ($field:ident) => {
                dst.$field[..self.$field.len()].copy_from_slice(&self.$field)
            };
        }
        restore!(font_flags);
        restore!(font_letter_space);
        restore!(font_check);
        restore!(font_size);
        restore!(font_dsize);
        restore!(font_params);
        restore!(font_name);
        restore!(font_area);
        restore!(font_bc);
        restore!(font_ec);
        restore!(bchar_label);
        restore!(font_bchar);
        restore!(font_false_bchar);
        restore!(font_glue);
        restore!(font_used);
        restore!(char_base);
        restore!(width_base);
        restore!(height_base);
        restore!(depth_base);
        restore!(italic_base);
        restore!(lig_kern_base);
        restore!(kern_base);
        restore!(exten_base);
        restore!(param_base);
        restore!(hyphen_char);
        restore!(skew_char);
        dst.xdv_buffer.clone_from(&self.xdv_buffer);
        dst.mapped_text.clone_from(&self.mapped_text);
    }

    fn allocated_bytes(&self) -> usize {
        self.font_flags.capacity() * size_of::<i8>()
            + self.font_letter_space.capacity() * size_of::<i32>()
            + self.font_check.capacity() * size_of::<B16x4>()
            + self.font_size.capacity() * size_of::<i32>()
            + self.font_dsize.capacity() * size_of::<i32>()
            + self.font_params.capacity() * size_of::<i32>()
            + self.font_name.capacity() * size_of::<i32>()
            + self.font_area.capacity() * size_of::<i32>()
            + self.font_bc.capacity() * size_of::<u16>()
            + self.font_ec.capacity() * size_of::<u16>()
            + self.bchar_label.capacity() * size_of::<i32>()
            + self.font_bchar.capacity() * size_of::<i32>()
            + self.font_false_bchar.capacity() * size_of::<i32>()
            + self.font_glue.capacity() * size_of::<i32>()
            + self.font_used.capacity()
            + self.char_base.capacity() * size_of::<i32>()
            + self.width_base.capacity() * size_of::<i32>()
            + self.height_base.capacity() * size_of::<i32>()
            + self.depth_base.capacity() * size_of::<i32>()
            + self.italic_base.capacity() * size_of::<i32>()
            + self.lig_kern_base.capacity() * size_of::<i32>()
            + self.kern_base.capacity() * size_of::<i32>()
            + self.exten_base.capacity() * size_of::<i32>()
            + self.param_base.capacity() * size_of::<i32>()
            + self.hyphen_char.capacity() * size_of::<i32>()
            + self.skew_char.capacity() * size_of::<i32>()
            + self.engines.capacity() * size_of::<Option<Arc<crate::state::FontEngineHandle>>>()
            + self.mappings.capacity() * size_of::<Option<Arc<crate::state::FontMappingHandle>>>()
            + self.xdv_buffer.capacity()
            + self.mapped_text.capacity() * size_of::<u16>()
    }
}

pub(crate) struct HyphTables {
    trie_min: Vec<i32>,
    hyf_distance: Vec<i16>,
    hyf_num: Vec<i16>,
    hyf_next: Vec<u16>,
    op_start: Vec<i32>,
    trie_used: Vec<u16>,
    trie_op_lang: Vec<u8>,
    trie_op_val: Vec<u16>,
    trie_trl: Vec<i32>,
    trie_tro: Vec<i32>,
    trie_trc: Vec<u16>,
    trie_c: Vec<u16>,
    trie_o: Vec<u16>,
    trie_l: Vec<i32>,
    trie_r: Vec<i32>,
    trie_hash: Vec<i32>,
    trie_taken: Vec<bool>,
    hyph_word: Vec<i32>,
    hyph_list: Vec<i32>,
    hyph_link: Vec<u16>,
    trie_op_hash_array: Vec<i32>,
}

impl HyphTables {
    fn capture(src: &crate::state::HyphState) -> Self {
        Self {
            trie_min: src.trie_min.clone(),
            hyf_distance: src.hyf_distance.clone(),
            hyf_num: src.hyf_num.clone(),
            hyf_next: src.hyf_next.clone(),
            op_start: src.op_start.clone(),
            trie_used: src.trie_used.clone(),
            trie_op_lang: src.trie_op_lang.clone(),
            trie_op_val: src.trie_op_val.clone(),
            trie_trl: src.trie_trl.clone(),
            trie_tro: src.trie_tro.clone(),
            trie_trc: src.trie_trc.clone(),
            trie_c: src.trie_c.clone(),
            trie_o: src.trie_o.clone(),
            trie_l: src.trie_l.clone(),
            trie_r: src.trie_r.clone(),
            trie_hash: src.trie_hash.clone(),
            trie_taken: src.trie_taken.clone(),
            hyph_word: src.hyph_word.clone(),
            hyph_list: src.hyph_list.clone(),
            hyph_link: src.hyph_link.clone(),
            trie_op_hash_array: src.trie_op_hash_array.clone(),
        }
    }

    fn restore(&self, dst: &mut crate::state::HyphState) {
        macro_rules! restore {
            ($field:ident) => {
                dst.$field.clone_from(&self.$field)
            };
        }
        restore!(trie_min);
        restore!(hyf_distance);
        restore!(hyf_num);
        restore!(hyf_next);
        restore!(op_start);
        restore!(trie_used);
        restore!(trie_op_lang);
        restore!(trie_op_val);
        restore!(trie_trl);
        restore!(trie_tro);
        restore!(trie_trc);
        restore!(trie_c);
        restore!(trie_o);
        restore!(trie_l);
        restore!(trie_r);
        restore!(trie_hash);
        restore!(trie_taken);
        restore!(hyph_word);
        restore!(hyph_list);
        restore!(hyph_link);
        restore!(trie_op_hash_array);
    }

    fn allocated_bytes(&self) -> usize {
        self.trie_min.capacity() * size_of::<i32>()
            + self.hyf_distance.capacity() * size_of::<i16>()
            + self.hyf_num.capacity() * size_of::<i16>()
            + self.hyf_next.capacity() * size_of::<u16>()
            + self.op_start.capacity() * size_of::<i32>()
            + self.trie_used.capacity() * size_of::<u16>()
            + self.trie_op_lang.capacity()
            + self.trie_op_val.capacity() * size_of::<u16>()
            + self.trie_trl.capacity() * size_of::<i32>()
            + self.trie_tro.capacity() * size_of::<i32>()
            + self.trie_trc.capacity() * size_of::<u16>()
            + self.trie_c.capacity() * size_of::<u16>()
            + self.trie_o.capacity() * size_of::<u16>()
            + self.trie_l.capacity() * size_of::<i32>()
            + self.trie_r.capacity() * size_of::<i32>()
            + self.trie_hash.capacity() * size_of::<i32>()
            + self.trie_taken.capacity()
            + self.hyph_word.capacity() * size_of::<i32>()
            + self.hyph_list.capacity() * size_of::<i32>()
            + self.hyph_link.capacity() * size_of::<u16>()
            + self.trie_op_hash_array.capacity() * size_of::<i32>()
    }
}

struct HyphBundle {
    tables_gen: u64,
    is_hyph: bool,
    hf: i32,
    max_hyph_char: i32,
    init_list: crate::units::MemPtr,
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
    fn capture(src: &crate::state::HyphState, previous: Option<&Self>) -> Self {
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

    fn restore(&self, dst: &mut crate::state::HyphState) {
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
        self.hc.capacity() * size_of::<i32>()
            + self.hu.capacity() * size_of::<i32>()
            + self.hyf.capacity()
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
        st.input.input_stack = vec![crate::globals::InputState::default(); 4];
        st.input.param_stack = vec![0; 4];
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
        st.shipout.dvi_buf = vec![4, 5, 6];
        st.strings.str_pool[0] = 11;
        st.strings.pool_ptr = 1;
        st.save_stack.save_ptr = 1;
        st.save_stack.set_b32s1(0, 12);
        st.input.buffer[0] = 13;
        st.input.last = 1;
        st.hyph.hc[0] = 14;
        let glyph = st.mem.glyph_blobs.insert(GlyphBlob {
            locations: vec![FixedPoint { x: 15, y: 0 }],
            gids: vec![16],
        });
        let bundle = EngineBundle::capture(&st, base, None);

        let scanner_ptr = st.scanner.native_text.as_ptr();
        let dvi_ptr = st.shipout.dvi_buf.as_ptr();
        st.scanner.logical_clock = 70;
        st.scanner.native_text[0] = 70;
        st.shipout.total_pages = 20;
        st.shipout.dvi_buf[0] = 40;
        st.shipout.page_manifest.begin_operation();
        st.shipout.page_manifest.begin_page(0);
        st.shipout.page_manifest.record_byte(139);
        st.strings.str_pool[0] = 110;
        st.save_stack.set_b32s1(0, 120);
        st.input.buffer[0] = 130;
        st.hyph.hc[0] = 140;
        st.mem.glyph_blobs.get_mut(glyph).locations[0].x = 150;
        st.input.input_file.push(crate::state::InputSlot::Terminal);

        bundle.restore(&mut st);
        assert_eq!(st.scanner.logical_clock, 7);
        assert_eq!(st.scanner.native_text, [1, 2, 3]);
        assert_eq!(st.scanner.native_text.as_ptr(), scanner_ptr);
        assert_eq!(st.shipout.total_pages, 2);
        assert_eq!(st.shipout.dvi_buf, [4, 5, 6]);
        assert_eq!(st.shipout.dvi_buf.as_ptr(), dvi_ptr);
        st.shipout.page_manifest.record_byte(140);
        st.shipout.page_manifest.finish_page(8);
        assert_eq!(st.shipout.page_manifest.take_pages().len(), 1);
        assert_eq!(st.strings.str_pool[0], 11);
        assert_eq!(st.save_stack.b32s1(0), 12);
        assert_eq!(st.input.buffer[0], 13);
        assert_eq!(st.hyph.hc[0], 14);
        assert_eq!(st.mem.glyph_blobs.get(glyph).locations[0].x, 15);
        assert!(matches!(
            st.input.input_file.last(),
            Some(crate::state::InputSlot::Terminal)
        ));
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
