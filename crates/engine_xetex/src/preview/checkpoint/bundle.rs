//! Non-journaled state carried by a checkpoint rung.

use std::mem::{size_of, take};
use std::rc::Rc;
use std::sync::Arc;

use crate::base::random::Random;
use crate::base::stack::TexStack;
use crate::base::token::{Token, TokenList, TokenStoreMarks};
use crate::base::units::{BufPos, FontId, MemPtr, PoolPos, PrimNum, SavePtr, Scaled, StrNumber};
use crate::build::hyphenation::{HyphState, TriePtr};
use crate::build::main_control::InterCharState;
use crate::ffi::teckit_icu::FontMappingHandle;
use crate::fonts::ext::SourceSpecials;
use crate::fonts::tfm::{FontMetrics, FontParams};
use crate::fonts::{FontEngineHandle, FontState, LigChar, ProtrusionCodes};
use crate::memory::glue_store::{GlueSpecId, GlueStoreMarks};
use crate::memory::glyph_arena::GlyphBlobArena;
use crate::memory::mem::{FreeLists, MemState};
use crate::memory::mem_types::ListStateRecord;
use crate::preview::sync_index::PreviewSyncIndex;
use crate::print::errors::ErrorState;
use crate::scanning::input::{InputClusterState, InputState, MacroParam, OpenLevel};
use crate::scanning::pgfmath::KnownCs;
use crate::scanning::scan::ScannerState;
use crate::shipout::ShipoutState;
use crate::shipout::dvi::DviWriter;
use crate::shipout::synctex::SynctexCtxt;
use crate::state::{ConfigState, EngineState};
use crate::tables::cs::{CsId, NameSlot};
use crate::tables::eqtb::{EqtbState, InterCharPair, SparseTable};
use crate::tables::save::{GroupCode, SaveEntry, SaveStackState};
use crate::tables::string_pool::StringsState;

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
            pool: strings.pool_ptr.index(),
            starts: strings.cur_start_slot(),
        }
    }
}

/// Large buffers are held outside their state structs so restore can reuse
/// the live allocation.
pub(crate) struct EngineBundle {
    config: ConfigState,
    errors: ErrorState,
    random: Random,
    shipout: ShipoutState,
    dvi: DviWriter,
    scanner: ScannerState,
    native_text: Vec<u16>,
    src_specials: SourceSpecials,
    synctex: SynctexCtxt,
    interchar: InterCharState,
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
        let dvi = take(&mut shipout.dvi).into_checkpoint();
        let mut scanner = st.scanner.clone();
        let native_text = take(&mut scanner.native_text);
        Self {
            config: st.config,
            errors: st.errors.clone(),
            random: st.random.clone(),
            shipout,
            dvi,
            scanner,
            native_text,
            src_specials: st.src_specials.clone(),
            synctex: st.synctex.clone(),
            interchar: st.interchar.clone(),
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
        st.random.clone_from(&self.random);

        let mut dvi = take(&mut st.shipout.dvi);
        // Operation-scoped, so a rewind after an abort still says where its transcript began.
        let log_resumed_at = st.shipout.log_resumed_at;
        st.shipout = self.shipout.clone();
        st.shipout.log_resumed_at = log_resumed_at;
        dvi.restore(&self.dvi);
        st.shipout.dvi = dvi;

        let native_text = take(&mut st.scanner.native_text);
        st.scanner = ScannerState {
            native_text,
            ..self.scanner.clone()
        };
        st.scanner.native_text.clone_from(&self.native_text);

        st.src_specials.clone_from(&self.src_specials);
        st.synctex.clone_from(&self.synctex);
        st.interchar.clone_from(&self.interchar);
        self.mem.restore(&mut st.mem);
        self.eqtb.restore(&mut st.eqtb);
        self.strings.restore(&mut st.strings);
        self.save.restore(&mut st.save_stack);
        self.input.restore(&mut st.input);
        self.fonts.restore(&mut st.fonts);
        self.hyph.restore(&mut st.hyph);
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.dvi.buffer_capacity()
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
            + self
                .synctex
                .preview_index
                .as_ref()
                .map_or(0, PreviewSyncIndex::allocated_bytes)
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
    words: usize,
    free: FreeLists,
    line: i32,
    cur_synctex_tag: i32,
    temp_ptr: MemPtr,
    glyph_blobs: GlyphBlobArena,
    tokens: TokenStoreMarks,
    glue: GlueStoreMarks,
}

impl MemBundle {
    fn capture(src: &MemState) -> Self {
        let MemState {
            data,
            glyph_blobs,
            tokens,
            glue,
            journal: _,
            free,
            line,
            cur_synctex_tag,
            temp_ptr,
        } = src;
        Self {
            words: data.len(),
            free: *free,
            line: *line,
            cur_synctex_tag: *cur_synctex_tag,
            temp_ptr: *temp_ptr,
            glyph_blobs: glyph_blobs.clone(),
            tokens: tokens.marks(),
            glue: glue.marks(),
        }
    }

    fn restore(&self, dst: &mut MemState) {
        dst.data.resize(self.words, Default::default());
        dst.free = self.free;
        if dst.journal.armed() {
            dst.journal.rebase(self.words);
        }
        dst.line = self.line;
        dst.cur_synctex_tag = self.cur_synctex_tag;
        dst.temp_ptr = self.temp_ptr;
        dst.glyph_blobs.restore_from(&self.glyph_blobs);
        dst.tokens.restore_marks(self.tokens);
        dst.glue.restore_marks(self.glue);
    }

    fn allocated_bytes(&self) -> usize {
        self.glyph_blobs.allocated_bytes()
    }
}

struct EqtbBundle {
    hash_used: NameSlot,
    hash_high: u32,
    no_new_control_sequence: bool,
    prim_used: PrimNum,
    cs_count: i32,
    hash_extra: u32,
    cs_top: CsId,
    pgfmath: KnownCs,
    inter_char: SparseTable<InterCharPair, TokenList>,
}

impl EqtbBundle {
    fn capture(src: &EqtbState) -> Self {
        let EqtbState {
            hash_used,
            hash_high,
            no_new_control_sequence,
            prim_used,
            cs_count,
            meanings: _,
            ints: _,
            dimens: _,
            glue: _,
            toks: _,
            shapes: _,
            boxes: _,
            fonts: _,
            cat_codes: _,
            lc_codes: _,
            uc_codes: _,
            sf_codes: _,
            math_codes: _,
            del_codes: _,
            inter_char,
            prim: _,
            hash: _,
            hash_extra,
            cs_top,
            journal: _,
            pgfmath,
        } = src;
        Self {
            hash_used: *hash_used,
            hash_high: *hash_high,
            no_new_control_sequence: *no_new_control_sequence,
            prim_used: *prim_used,
            cs_count: *cs_count,
            hash_extra: *hash_extra,
            cs_top: *cs_top,
            pgfmath: *pgfmath,
            inter_char: inter_char.clone(),
        }
    }

    fn restore(&self, dst: &mut EqtbState) {
        dst.hash_used = self.hash_used;
        dst.hash_high = self.hash_high;
        dst.no_new_control_sequence = self.no_new_control_sequence;
        dst.prim_used = self.prim_used;
        dst.cs_count = self.cs_count;
        dst.hash_extra = self.hash_extra;
        dst.cs_top = self.cs_top;
        dst.pgfmath = self.pgfmath;
        dst.inter_char.clone_from(&self.inter_char);
    }
}

struct StringsBundle {
    base: StringWatermark,
    str_ptr: StrNumber,
    max_strings: StrNumber,
    init_str_ptr: StrNumber,
    init_pool_ptr: PoolPos,
    job_name: StrNumber,
    texmf_log_name: StrNumber,
    string_vacancies: i32,
    pool_size: PoolPos,
    pool_ptr: PoolPos,
    format_ident: StrNumber,
    pool_span: Vec<u16>,
    start_span: Vec<PoolPos>,
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
        dst.str_pool[self.base.pool..][..self.pool_span.len()].copy_from_slice(&self.pool_span);
        dst.str_start[self.base.starts..][..self.start_span.len()]
            .copy_from_slice(&self.start_span);
    }

    fn allocated_bytes(&self) -> usize {
        vec_bytes(&self.pool_span) + vec_bytes(&self.start_span)
    }
}

struct SaveBundle {
    save_ptr: SavePtr,
    max_save_stack: SavePtr,
    save_size: usize,
    cur_level: u16,
    cur_group: GroupCode,
    cur_boundary: Option<SavePtr>,
    data: Vec<SaveEntry>,
}

impl SaveBundle {
    fn capture(src: &SaveStackState) -> Self {
        let end = src.save_ptr.index();
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
    input_stack: StackBundle<InputState>,
    param_stack: StackBundle<MacroParam>,
    nest: StackBundle<ListStateRecord>,
    open_levels: Vec<OpenLevel>,
    cur_input: InputState,
    cur_list: ListStateRecord,
    open_parens: i32,
    in_open: usize,
    pseudo_files: MemPtr,
    base_ptr: usize,
    par_loc: Option<CsId>,
    par_token: Token,
    force_eof: bool,
    first: BufPos,
    last: BufPos,
    max_buf_stack: BufPos,
    read_open: [u8; 17],
}

/// The live part of a [`TexStack`], with its depth and high-water mark.
struct StackBundle<T> {
    slots: Vec<T>,
    len: usize,
    max_len: usize,
}

impl<T: Copy + Default> StackBundle<T> {
    /// The entries and the parked top slot.
    fn with_parked(src: &TexStack<T>) -> Self {
        Self::of(src, src.with_parked())
    }

    fn entries(src: &TexStack<T>) -> Self {
        Self::of(src, src.entries())
    }

    fn of(src: &TexStack<T>, slots: &[T]) -> Self {
        StackBundle {
            slots: slots.to_vec(),
            len: src.len(),
            max_len: src.max_len(),
        }
    }

    fn restore(&self, dst: &mut TexStack<T>) {
        dst.restore(&self.slots, self.len, self.max_len);
    }
}

fn live_prefix<T: Clone>(values: &[T], len: i32) -> Vec<T> {
    values[..(len.max(0) as usize).min(values.len())].to_vec()
}

impl InputBundle {
    fn capture(src: &InputClusterState) -> Self {
        Self {
            buffer: src.buffer.0[..src.buffer_in_use().min(src.buffer.0.len())].to_vec(),
            input_stack: StackBundle::with_parked(&src.input_stack),
            param_stack: StackBundle::entries(&src.param_stack),
            nest: StackBundle::with_parked(&src.nest),
            open_levels: src.open_levels.clone(),
            cur_input: src.cur_input,
            cur_list: src.cur_list,
            open_parens: src.open_parens,
            in_open: src.in_open,
            pseudo_files: src.pseudo_files,
            base_ptr: src.base_ptr,
            par_loc: src.par_loc,
            par_token: src.par_token,
            force_eof: src.force_eof,
            first: src.first,
            last: src.last,
            max_buf_stack: src.max_buf_stack,
            read_open: src.read_open,
        }
    }

    fn restore(&self, dst: &mut InputClusterState) {
        dst.buffer.0[..self.buffer.len()].copy_from_slice(&self.buffer);
        self.input_stack.restore(&mut dst.input_stack);
        self.param_stack.restore(&mut dst.param_stack);
        self.nest.restore(&mut dst.nest);
        dst.open_levels.clone_from(&self.open_levels);
        dst.cur_input = self.cur_input;
        dst.cur_list = self.cur_list;
        dst.open_parens = self.open_parens;
        dst.in_open = self.in_open;
        dst.pseudo_files = self.pseudo_files;
        dst.base_ptr = self.base_ptr;
        dst.par_loc = self.par_loc;
        dst.par_token = self.par_token;
        dst.force_eof = self.force_eof;
        dst.first = self.first;
        dst.last = self.last;
        dst.max_buf_stack = self.max_buf_stack;
        dst.read_open = self.read_open;
    }

    fn allocated_bytes(&self) -> usize {
        vec_bytes(&self.buffer)
            + vec_bytes(&self.input_stack.slots)
            + vec_bytes(&self.param_stack.slots)
            + vec_bytes(&self.nest.slots)
            + vec_bytes(&self.open_levels)
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
    font_check: [u8; 4],
    font_size: Scaled,
    font_dsize: Scaled,
    font_name: StrNumber,
    font_area: StrNumber,
    font_bc: u16,
    font_ec: u16,
    font_bchar: LigChar,
    font_false_bchar: LigChar,
    font_glue: Option<GlueSpecId>,
    font_used: bool,
    hyphen_char: i32,
    skew_char: i32,
}

struct FontBundle {
    loaded_font_mapping: Option<Arc<FontMappingHandle>>,
    loaded_font_flags: i8,
    loaded_font_letter_space: Scaled,
    xtx_ligature_present: bool,
    gave_char_warning_help: bool,
    native_font_type_flag: StrNumber,
    font_mem_size: i32,
    font_max: FontId,
    fmem_ptr: i32,
    font_ptr: FontId,
    arrays: FontArrays,
    engines: Vec<Option<Rc<FontEngineHandle>>>,
    metrics: Vec<FontMetrics>,
    params: Vec<FontParams>,
    mappings: Vec<Option<Arc<FontMappingHandle>>>,
    mapped_text: Vec<u16>,
    cp_codes: ProtrusionCodes,
}

impl FontBundle {
    fn capture(src: &FontState) -> Self {
        let FontState {
            loaded_font_mapping,
            loaded_font_flags,
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
            font_name: _,
            font_area: _,
            font_bc: _,
            font_ec: _,
            font_bchar: _,
            font_false_bchar: _,
            font_glue: _,
            font_used: _,
            hyphen_char: _,
            skew_char: _,
            metrics,
            params,
            mapped_text,
            cp_codes,
            glyph_bboxes: _,
        } = src;
        let count = font_ptr.raw() + 1;
        Self {
            loaded_font_mapping: loaded_font_mapping.clone(),
            loaded_font_flags: *loaded_font_flags,
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
            metrics: live_prefix(metrics.as_slice(), count),
            params: live_prefix(params.as_slice(), count),
            mappings: live_prefix(font_mapping.as_slice(), count),
            mapped_text: mapped_text.clone(),
            cp_codes: cp_codes.clone(),
        }
    }

    fn restore(&self, dst: &mut FontState) {
        for f in (self.font_ptr.get() + 1..=dst.font_ptr.get()).map(FontId::new) {
            dst.font_layout_engine[f] = None;
            dst.font_mapping[f] = None;
            dst.metrics[f] = FontMetrics::default();
            dst.params[f] = FontParams::default();
        }
        dst.font_layout_engine.as_mut_slice()[..self.engines.len()].clone_from_slice(&self.engines);
        dst.font_mapping.as_mut_slice()[..self.mappings.len()].clone_from_slice(&self.mappings);
        dst.metrics.as_mut_slice()[..self.metrics.len()].clone_from_slice(&self.metrics);
        dst.params.as_mut_slice()[..self.params.len()].clone_from_slice(&self.params);
        // The ICU iterator points into transient text and is rebuilt lazily.
        dst.release_linebreak_state();
        // Slots reused after the checkpoint may have emitted font definitions
        // in the abandoned timeline, so they must start unused.
        let restored_font_count = (self.font_ptr.raw() + 1).max(0) as usize;
        if let Some(abandoned) = dst.font_used.as_mut_slice().get_mut(restored_font_count..) {
            abandoned.fill(false);
        }
        dst.loaded_font_mapping
            .clone_from(&self.loaded_font_mapping);
        dst.loaded_font_flags = self.loaded_font_flags;
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
        dst.cp_codes.clone_from(&self.cp_codes);
        // Font slots past the restored `font_ptr` may be reloaded with other fonts.
        dst.glyph_bboxes
            .get_mut()
            .retain(|&(f, _), _| f <= self.font_ptr);
    }

    fn allocated_bytes(&self) -> usize {
        self.arrays.allocated_bytes()
            + vec_bytes(&self.engines)
            + vec_bytes(&self.metrics)
            + vec_bytes(&self.params)
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
    trie_min: u32,
    hyf_distance: u16,
    hyf_num: u16,
    hyf_next: u16,
    op_start: usize,
    trie_used: u16,
    trie_op_lang: u8,
    trie_op_val: u16,
    trie_trl: i32,
    trie_tro: i32,
    trie_trc: u16,
    trie_c: u16,
    trie_o: u16,
    trie_l: u32,
    trie_r: u32,
    trie_hash: u32,
    trie_taken: bool,
    hyph_word: StrNumber,
    hyph_list: MemPtr,
    hyph_link: u16,
    trie_op_hash_array: i32,
}

struct HyphBundle {
    tables_gen: u64,
    hf: FontId,
    max_hyph_char: i32,
    init_list: MemPtr,
    init_lig: bool,
    init_lft: bool,
    hyphen_passed: usize,
    trie_max: TriePtr,
    trie_size: i32,
    trie_ptr: i32,
    hyph_count: i32,
    hyph_size: i32,
    hyph_next: i32,
    trie_op_ptr: i32,
    max_op_used: u16,
    trie_not_ready: bool,
    hyph_start: TriePtr,
    hyph_index: Option<TriePtr>,
    cur_lang: u8,
    hyf_char: LigChar,
    l_hyf: i32,
    r_hyf: i32,
    hyf_bchar: LigChar,
    hc: Vec<i32>,
    hu: Vec<LigChar>,
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
            hyf_bchar: src.hyf_bchar,
            hc: src.hc.clone(),
            hu: src.hu.clone(),
            hyf: src.hyf.clone(),
            tables,
        }
    }

    fn restore(&self, dst: &mut HyphState) {
        dst.tables_gen = self.tables_gen;
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
    use crate::base::units::Scaled;
    use crate::build::main_control::BoxContext;
    use crate::fonts::{PerFont, Side};
    use crate::memory::glyph_arena::{FixedPoint, GlyphBlob};
    use crate::memory::mem::RESERVED_WORDS;
    use crate::memory::mem_types::zeroed_vec;
    use crate::scanning::input::{InputBuffer, InputSlot};
    use crate::tables::save::Saved;

    const BOX_12: BoxContext = BoxContext::Shift(Scaled::from_raw(12));

    fn state() -> EngineState {
        let mut st = EngineState::default();
        st.strings.str_pool = vec![0; 16];
        st.strings.str_start = vec![PoolPos::ZERO; 8];
        st.input.buffer = InputBuffer(vec![0; 8]);
        st.input.input_stack = TexStack::with_size(3);
        st.input.param_stack = TexStack::with_size(3);
        st.input.nest = TexStack::with_size(3);
        st.save_stack.alloc(4);
        st
    }

    #[test]
    fn bundle_restores_non_journaled_state_without_moving_live_buffers() {
        let mut st = state();
        let base = StringWatermark::capture(&st);
        st.scanner.logical_clock = 7;
        st.scanner.native_text = vec![1, 2, 3];
        st.shipout.total_pages = 2;
        st.shipout.dvi.init();
        for c in [4, 5, 6] {
            st.shipout.dvi.set_char(c).unwrap();
        }
        st.strings.str_pool[0] = 11;
        st.strings.pool_ptr = PoolPos::from_raw(1);
        st.save_stack.push_saved(Saved::BoxContext(BOX_12));
        st.input.buffer.0[0] = 13;
        st.input.last = BufPos::from_raw(1);
        st.hyph.hc[0] = 14;
        let glyph = st.mem.glyph_blobs.insert(GlyphBlob {
            locations: vec![FixedPoint {
                x: Scaled::from_raw(15),
                y: Scaled::ZERO,
            }],
            gids: vec![16],
        });
        let bundle = EngineBundle::capture(&st, base, None);

        let scanner_ptr = st.scanner.native_text.as_ptr();
        let dvi_ptr = st.shipout.dvi.digest_view().1.as_ptr();
        st.scanner.logical_clock = 70;
        st.scanner.native_text[0] = 70;
        st.shipout.total_pages = 20;
        st.shipout.dvi.set_char(40).unwrap();
        st.shipout.dvi.manifest.begin_operation();
        st.shipout.dvi.manifest.begin_page(0);
        st.shipout.dvi.manifest.record_byte(139);
        st.strings.str_pool[0] = 110;
        st.save_stack.data[0] =
            SaveEntry::Saved(Saved::BoxContext(BoxContext::Shift(Scaled::from_raw(120))));
        st.input.buffer.0[0] = 130;
        st.hyph.hc[0] = 140;
        st.mem.glyph_blobs.get_mut(glyph).locations[0].x = Scaled::from_raw(150);
        st.input.input_file.push(InputSlot::Terminal);

        bundle.restore(&mut st);
        assert_eq!(st.scanner.logical_clock, 7);
        assert_eq!(st.scanner.native_text, [1, 2, 3]);
        assert_eq!(st.scanner.native_text.as_ptr(), scanner_ptr);
        assert_eq!(st.shipout.total_pages, 2);
        let (position, buf) = st.shipout.dvi.digest_view();
        assert_eq!(position, [3, 0, 0]);
        assert_eq!(buf[..4], [4, 5, 6, 0]);
        assert_eq!(buf.as_ptr(), dvi_ptr);
        st.shipout.dvi.manifest.record_byte(140);
        st.shipout.dvi.manifest.finish_page(8);
        assert_eq!(st.shipout.dvi.manifest.take_pages().len(), 1);
        assert_eq!(st.strings.str_pool[0], 11);
        assert_eq!(st.save_stack.saved(-1), Some(Saved::BoxContext(BOX_12)));
        assert_eq!(st.input.buffer.0[0], 13);
        assert_eq!(st.hyph.hc[0], 14);
        assert_eq!(
            st.mem.glyph_blobs.get(glyph).locations[0].x,
            Scaled::from_raw(15)
        );
        assert!(matches!(
            st.input.input_file.last(),
            Some(InputSlot::Terminal)
        ));
    }

    #[test]
    fn mem_restore_drops_grown_words_and_restores_free_lists() {
        let mut st = state();
        st.mem.data = zeroed_vec(RESERVED_WORDS);
        let kept = st.mem.get_node(3).unwrap();
        st.mem.free_node(kept, 3);
        let bundle = MemBundle::capture(&st.mem);

        let reused = st.mem.get_node(3).unwrap();
        let grown = st.mem.get_node(5).unwrap();
        assert_eq!(reused, kept);
        bundle.restore(&mut st.mem);

        assert_eq!(st.mem.data.len(), RESERVED_WORDS + 3);
        assert_eq!(st.mem.free.0[3], Some(kept));
        assert_eq!(st.mem.get_node(5).unwrap(), grown);
    }

    #[test]
    fn font_restore_clears_use_flags_beyond_the_checkpoint() {
        let mut fonts = FontState {
            font_layout_engine: PerFont::filled(4, None),
            font_mapping: PerFont::filled(4, None),
            metrics: PerFont::filled(4, FontMetrics::default()),
            params: PerFont::filled(4, FontParams::default()),
            font_used: PerFont::filled(4, false),
            font_ptr: FontId::new(1),
            ..Default::default()
        };
        let checkpoint = FontBundle::capture(&fonts);

        fonts.font_ptr = FontId::new(3);
        fonts.font_used[FontId::new(2)] = true;
        fonts.font_used[FontId::new(3)] = true;
        checkpoint.restore(&mut fonts);

        assert_eq!(fonts.font_ptr, FontId::new(1));
        assert_eq!(fonts.font_used.as_slice(), [false, false, false, false]);
    }

    #[test]
    fn font_restore_rolls_back_protrusion_codes_and_stale_glyph_bounds() {
        use texyon_xetex_layout::GlyphBBox;

        let mut fonts = FontState {
            font_layout_engine: PerFont::filled(4, None),
            font_mapping: PerFont::filled(4, None),
            metrics: PerFont::filled(4, FontMetrics::default()),
            params: PerFont::filled(4, FontParams::default()),
            font_used: PerFont::filled(4, false),
            font_ptr: FontId::new(1),
            ..Default::default()
        };
        let (f1, f2) = (FontId::new(1), FontId::new(2));
        fonts.cp_codes.set(f1, Side::Left, 65, 50);
        let checkpoint = FontBundle::capture(&fonts);

        fonts.font_ptr = f2;
        fonts.cp_codes.set(f1, Side::Left, 65, 70);
        fonts.cp_codes.set(f1, Side::Right, 65, 80);
        let bbox = GlyphBBox {
            y_max: 1.0,
            ..GlyphBBox::default()
        };
        fonts.glyph_bboxes.get_mut().insert((f1, 3), bbox);
        fonts.glyph_bboxes.get_mut().insert((f2, 3), bbox);
        checkpoint.restore(&mut fonts);

        assert_eq!(fonts.cp_codes.get(f1, Side::Left, 65), 50);
        assert_eq!(fonts.cp_codes.get(f1, Side::Right, 65), 0);
        let cached: Vec<_> = fonts.glyph_bboxes.get_mut().keys().copied().collect();
        assert_eq!(cached, [(f1, 3)]);
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
