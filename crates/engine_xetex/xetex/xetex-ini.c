/* xetex-ini.c: WEB initialization code translated to C
   Copyright 2016-2022 The Tectonic Project
   Licensed under the MIT License.
*/

/* On Windows this can bring in a `#define INPUT` that clashes with
 * xetex_format.h, so include it first and sanitize: */
#include "teckit-c-Engine.h"
#ifdef INPUT
#undef INPUT
#endif

#include "xetex-core.h"
#include "xetex-xetexd.h"
#include "xetex-synctex.h"
#include "dpx-pdfobj.h" /* pdf_files_{init,close} */
#include "xetex_bindings.h" /* FORMAT_SERIAL */

/* All the following variables are declared in xetex-xetexd.h */
bool shell_escape_enabled = false;
memory_word *eqtb;
int32_t bad;
char *name_of_file;
UTF16_code *name_of_file16;
int32_t name_length;
int32_t name_length16;
UnicodeScalar *buffer;
int32_t first;
int32_t last;
int32_t max_buf_stack;
bool in_initex_mode;
int32_t error_line;
int32_t half_error_line;
int32_t max_print_line;
int32_t max_strings;
int32_t strings_free;
int32_t string_vacancies;
int32_t pool_size;
int32_t pool_free;
int32_t font_mem_size;
int32_t font_max;
int32_t hyph_size;
int32_t trie_size;
int32_t buf_size;
int32_t stack_size;
int32_t max_in_open;
int32_t param_size;
int32_t nest_size;
int32_t save_size;
int32_t expand_depth;
int file_line_error_style_p;
int halt_on_error_p;
bool quoted_filename;
bool insert_src_special_auto;
bool insert_src_special_every_par;
bool insert_src_special_every_math;
bool insert_src_special_every_vbox;
packed_UTF16_code *str_pool;
pool_pointer *str_start;
pool_pointer pool_ptr;
str_number str_ptr;
pool_pointer init_pool_ptr;
str_number init_str_ptr;
rust_output_handle_t rust_stdout;
rust_output_handle_t log_file;
selector_t selector;
unsigned char dig[23];
int32_t tally;
int32_t term_offset;
int32_t file_offset;
UTF16_code trick_buf[256];
int32_t trick_count;
int32_t first_count;
bool doing_special;
UTF16_code *native_text;
int32_t native_text_size;
int32_t native_len;
int32_t save_native_len;
unsigned char interaction;
bool deletions_allowed;
bool set_box_allowed;
tt_history_t history;
signed char error_count;
const char* help_line[6];
unsigned char help_ptr;
bool use_err_help;
bool arith_error;
scaled_t tex_remainder;
int32_t randoms[55];
unsigned char j_random;
scaled_t random_seed;
int32_t two_to_the[31];
int32_t spec_log[29];
int32_t temp_ptr;
memory_word *mem;
int32_t lo_mem_max;
int32_t hi_mem_min;
int32_t var_used, dyn_used;
int32_t avail;
int32_t mem_end;
int32_t rover;
int32_t last_leftmost_char;
int32_t last_rightmost_char;
int32_t hlist_stack[MAX_HLIST_STACK + 1];
short hlist_stack_level;
int32_t first_p;
int32_t global_prev_p;
int32_t font_in_short_display;
int32_t depth_threshold;
int32_t breadth_max;
list_state_record *nest;
int32_t nest_ptr;
int32_t max_nest_stack;
list_state_record cur_list;
short shown_mode;
unsigned char old_setting;
b32x2 *hash;
int32_t hash_used;
int32_t hash_extra;
int32_t hash_top;
int32_t eqtb_top;
int32_t hash_high;
bool no_new_control_sequence;
int32_t cs_count;
b32x2 prim[PRIM_SIZE + 1];
int32_t prim_used;
memory_word *save_stack;
int32_t save_ptr;
int32_t max_save_stack;
uint16_t cur_level;
group_code cur_group;
int32_t cur_boundary;
int32_t mag_set;
eight_bits cur_cmd;
int32_t cur_chr;
int32_t cur_cs;
int32_t cur_tok;
input_state_t *input_stack;
int32_t input_ptr;
int32_t max_in_stack;
input_state_t cur_input;
int32_t in_open;
int32_t open_parens;
UFILE **input_file;
int32_t line;
int32_t *line_stack;
str_number *source_filename_stack;
str_number *full_source_filename_stack;
unsigned char scanner_status;
int32_t warning_index;
int32_t def_ref;
int32_t *param_stack;
int32_t param_ptr;
int32_t max_param_stack;
int32_t align_state;
int32_t base_ptr;
int32_t par_loc;
int32_t par_token;
bool force_eof;
int32_t expand_depth_count;
bool is_in_csname;
int32_t cur_mark[5];
unsigned char long_state;
int32_t pstack[9];
int32_t cur_val;
int32_t cur_val1;
unsigned char cur_val_level;
small_number radix;
glue_ord cur_order;
UFILE *read_file[16];
unsigned char read_open[17];
int32_t cond_ptr;
unsigned char if_limit;
small_number cur_if;
int32_t if_line;
int32_t skip_line;
str_number cur_name;
str_number cur_area;
str_number cur_ext;
pool_pointer area_delimiter;
pool_pointer ext_delimiter;
UTF16_code file_name_quote_char;
int32_t format_default_length;
char *TEX_format_default;
bool name_in_progress;
str_number job_name;
bool log_opened;
const char* output_file_extension;
str_number texmf_log_name;
memory_word *font_info;
font_index fmem_ptr;
internal_font_number font_ptr;
b16x4 *font_check;
scaled_t *font_size;
scaled_t *font_dsize;
font_index *font_params;
str_number *font_name;
str_number *font_area;
UTF16_code *font_bc;
UTF16_code *font_ec;
int32_t *font_glue;
bool *font_used;
int32_t *hyphen_char;
int32_t *skew_char;
font_index *bchar_label;
nine_bits *font_bchar;
nine_bits *font_false_bchar;
void **font_layout_engine;
void **font_mapping;
char *font_flags;
scaled_t *font_letter_space;
void *loaded_font_mapping;
char loaded_font_flags;
scaled_t loaded_font_letter_space;
UTF16_code *mapped_text;
char *xdv_buffer;
int32_t *char_base;
int32_t *width_base;
int32_t *height_base;
int32_t *depth_base;
int32_t *italic_base;
int32_t *lig_kern_base;
int32_t *kern_base;
int32_t *exten_base;
int32_t *param_base;
b16x4 null_character;
int32_t total_pages;
scaled_t max_v;
scaled_t max_h;
int32_t max_push;
int32_t last_bop;
int32_t dead_cycles;
bool doing_leaders;
scaled_t rule_ht, rule_dp, rule_wd;
scaled_t cur_h, cur_v;
int32_t epochseconds;
int32_t microseconds;
scaled_t total_stretch[4], total_shrink[4];
int32_t last_badness;
int32_t adjust_tail;
int32_t pre_adjust_tail;
int32_t pack_begin_line;
b32x2 empty;
internal_font_number cur_f;
int32_t cur_c;
b16x4 cur_i;
int32_t cur_align;
int32_t cur_span;
int32_t cur_loop;
int32_t align_ptr;
int32_t cur_head, cur_tail;
int32_t cur_pre_head, cur_pre_tail;
int32_t just_box;
scaled_t active_width[7];
int32_t hc[4099];
internal_font_number hf;
int32_t hu[4097];
unsigned char cur_lang;
int32_t max_hyph_char;
unsigned char hyf[4097];
int32_t init_list;
bool init_lig;
bool init_lft;
small_number hyphen_passed;
int32_t cur_l, cur_r;
int32_t cur_q;
int32_t lig_stack;
bool ligature_present;
bool lft_hit, rt_hit;
trie_pointer *trie_trl;
trie_pointer *trie_tro;
uint16_t *trie_trc;
small_number hyf_distance[TRIE_OP_SIZE + 1];
small_number hyf_num[TRIE_OP_SIZE + 1];
trie_opcode hyf_next[TRIE_OP_SIZE + 1];
int32_t op_start[256];
str_number *hyph_word;
int32_t *hyph_list;
hyph_pointer *hyph_link;
int32_t hyph_count;
int32_t hyph_next;
trie_opcode trie_used[256];
unsigned char trie_op_lang[TRIE_OP_SIZE + 1];
trie_opcode trie_op_val[TRIE_OP_SIZE + 1];
int32_t trie_op_ptr;
trie_opcode max_op_used;
packed_UTF16_code *trie_c;
trie_opcode *trie_o;
trie_pointer *trie_l;
trie_pointer *trie_r;
trie_pointer trie_ptr;
trie_pointer *trie_hash;
bool *trie_taken;
trie_pointer trie_min[65536];
trie_pointer trie_max;
bool trie_not_ready;
scaled_t best_height_plus_depth;
internal_font_number main_f;
b16x4 main_i;
b16x4 main_j;
font_index main_k;
int32_t main_p;
int32_t main_pp, main_ppp;
int32_t main_h;
bool is_hyph;
int32_t space_class;
int32_t prev_class;
int32_t main_s;
int32_t bchar;
int32_t false_bchar;
bool cancel_boundary;
bool ins_disc;
int32_t cur_box;
int32_t after_token;
bool long_help_seen;
str_number format_ident;
rust_output_handle_t write_file[16];
bool write_open[18];
int32_t write_loc;
scaled_t cur_page_width;
scaled_t cur_page_height;
scaled_t cur_h_offset;
scaled_t cur_v_offset;
int32_t pdf_last_x_pos;
int32_t pdf_last_y_pos;
bool *eof_seen;
int32_t LR_ptr;
int32_t LR_problems;
small_number cur_dir;
int32_t pseudo_files;
save_pointer *grp_stack;
int32_t *if_stack;
int32_t max_reg_num;
const char* max_reg_help_line;
int32_t sa_root[8];
int32_t cur_ptr;
memory_word sa_null;
int32_t sa_chain;
uint16_t sa_level;
trie_pointer hyph_start;
trie_pointer hyph_index;
int32_t disc_ptr[4];
pool_pointer edit_name_start;
bool stop_at_space;
int32_t native_font_type_flag;
bool xtx_ligature_present;
scaled_t delta;
int synctex_enabled;
bool used_tectonic_coda_tokens;
bool semantic_pagination_enabled;
bool gave_char_warning_help;

/* These ought to live in xetex-pagebuilder.c but are shared a lot: */
int32_t page_tail;
unsigned char page_contents;
scaled_t page_so_far[8];
int32_t last_glue;
int32_t last_penalty;
scaled_t last_kern;
int32_t last_node_type;
int32_t insert_penalties;
bool output_active;

uint16_t _xeq_level_array[EQTB_SIZE - INT_BASE + 1];

#define NEG_TRIE_OP_SIZE -35111L
#define MAX_TRIE_OP 65535L

extern int32_t _trie_op_hash_array[TRIE_OP_SIZE - NEG_TRIE_OP_SIZE + 1];
#define TRIE_OP_HASH(i) _trie_op_hash_array[(i) - NEG_TRIE_OP_SIZE]

static b32x2 *yhash;

#define FORMAT_HEADER_MAGIC 0x54544E43 /* "TTNC" in ASCII */
#define FORMAT_FOOTER_MAGIC 0x0000029A

/* Read and write dump files.  As distributed, these files are
   architecture dependent; specifically, BigEndian and LittleEndian
   architectures produce different files.  These routines always output
   BigEndian files.  This still does not guarantee them to be
   architecture-independent, because it is possible to make a format
   that dumps a glue ratio, i.e., a floating-point number.  Fortunately,
   none of the standard formats do that.  */

#if !defined (WORDS_BIGENDIAN)

/* This macro is always invoked as a statement.  It assumes a variable
   `temp'.  */

#define SWAP(x, y) temp = (x); (x) = (y); (y) = temp


/* Make the NITEMS items pointed at by P, each of size SIZE, be the
   opposite-endianness of whatever they are now.  */

static void
swap_items (char *p, size_t nitems, size_t size)
{
    char temp;

    switch (size) {
    case 16:
        while (nitems--) {
            SWAP (p[0], p[15]);
            SWAP (p[1], p[14]);
            SWAP (p[2], p[13]);
            SWAP (p[3], p[12]);
            SWAP (p[4], p[11]);
            SWAP (p[5], p[10]);
            SWAP (p[6], p[9]);
            SWAP (p[7], p[8]);
            p += size;
        }
        break;
    case 8:
        while (nitems--) {
            SWAP (p[0], p[7]);
            SWAP (p[1], p[6]);
            SWAP (p[2], p[5]);
            SWAP (p[3], p[4]);
            p += size;
        }
        break;
    case 4:
        while (nitems--) {
            SWAP (p[0], p[3]);
            SWAP (p[1], p[2]);
            p += size;
        }
        break;
    case 2:
        while (nitems--) {
            SWAP (p[0], p[1]);
            p += size;
        }
        break;
    case 1:
        break; /* Nothing to do. */
    default:
        _tt_abort("can't swap a %"PRIuZ"-byte item for (un)dumping", size);
    }
}
#else /* not WORDS_BIGENDIAN */
#define swap_items(a,b,c) do {} while(0)
#endif


/* Here we write NITEMS items, each item being ITEM_SIZE bytes long.
   The pointer to the stuff to write is P, and we write to the file
   OUT_FILE.  */

static void
do_dump (char *p, size_t item_size, size_t nitems, rust_output_handle_t out_file)
{
    swap_items (p, nitems, item_size);

    ssize_t r = ttstub_output_write (out_file, p, item_size * nitems);
    if (r < 0 || (size_t) r != item_size * nitems)
        _tt_abort ("could not write %"PRIuZ" %"PRIuZ"-byte item(s) to %s",
                   nitems, item_size, name_of_file);

    /* Have to restore the old contents of memory, since some of it might
       get used again.  */
    swap_items (p, nitems, item_size);
}


/* Here is the dual of the writing routine.  */

static void
do_undump (char *p, size_t item_size, size_t nitems, rust_input_handle_t in_file)
{
    ssize_t r = ttstub_input_read (in_file, p, item_size * nitems);
    if (r < 0 || (size_t) r != item_size * nitems)
        _tt_abort("could not undump %"PRIuZ" %"PRIuZ"-byte item(s) from %s",
                  nitems, item_size, name_of_file);

    swap_items (p, nitems, item_size);
}


#define dump_things(base, len) \
    do_dump ((char *) &(base), sizeof (base), (size_t) (len), fmt_out)
#define undump_things(base, len) \
    do_undump ((char *) &(base), sizeof (base), (size_t) (len), fmt_in)

/* Like do_undump, but check each value against LOW and HIGH.  The
   slowdown isn't significant, and this improves the chances of
   detecting incompatible format files.  In fact, Knuth himself noted
   this problem with Web2c some years ago, so it seems worth fixing.  We
   can't make this a subroutine because then we lose the type of BASE.  */
#define undump_checked_things(low, high, base, len)                     \
    do {                                                                \
        int i;                                                     \
        undump_things (base, len);                                      \
        for (i = 0; i < (len); i++) {                                   \
            if ((&(base))[i] < (low) || (&(base))[i] > (high)) {        \
                _tt_abort ("item %u (=%" PRIdPTR ") of .fmt array at %" PRIxPTR \
                           " <%" PRIdPTR " or >%" PRIdPTR,              \
                           i, (uintptr_t) (&(base))[i], (uintptr_t) &(base), \
                           (uintptr_t) low, (uintptr_t) high);          \
            }                                                           \
        }                                                               \
    } while (0)

/* Like undump_checked_things, but only check the upper value. We use
   this when the base type is unsigned, and thus all the values will be
   greater than zero by definition.  */
#define undump_upper_check_things(high, base, len)                      \
    do {                                                                \
        int i;                                                     \
        undump_things (base, len);                                      \
        for (i = 0; i < (len); i++) {                                   \
            if ((&(base))[i] > (high)) {                                \
                _tt_abort ("Item %u (=%" PRIdPTR ") of .fmt array at %" PRIxPTR \
                           " >%" PRIdPTR,                               \
                           i, (uintptr_t) (&(base))[i], (uintptr_t) &(base), \
                           (uintptr_t) high);                           \
            }                                                           \
        }                                                               \
    } while (0)


/* Since dump_things is a macro with a sizeof(), these all work: */
#define dump_b64(x) dump_things(x, 1)
#define dump_b32(x) dump_things(x, 1)
#define undump_b64(x) undump_things(x, 1)
#define undump_b32(x) undump_things(x, 1)

/* `dump_int' is called with constant integers, so we put them into a
   variable first.  */
#define dump_int(x)            \
    do {                       \
        int32_t x_val = (x);   \
        dump_things(x_val, 1); \
    } while (0)

#define undump_int(x) undump_things(x, 1)


#define hash_offset 514
#define sup_max_strings 2097151L /* magic constant, origin unclear */
#define sup_font_mem_size 147483647L /* magic constant, origin unclear */
#define sup_pool_size 40000000L
#define sup_string_vacancies (sup_pool_size - 23000)
#define sup_hash_extra sup_max_strings
#define inf_expand_depth 10
#define sup_expand_depth 10000000L


/*:134*//*135: */

// translated to Rust: src/ini.rs sort_avail

/*:271*//*276: */

// translated to Rust: src/ini.rs primitive

/*:925*//*977: */

// translated to Rust: src/ini.rs new_trie_op

// translated to Rust: src/ini.rs trie_node

// translated to Rust: src/ini.rs compress_trie

// translated to Rust: src/ini.rs first_fit

// translated to Rust: src/ini.rs trie_pack

// translated to Rust: src/ini.rs trie_fix

// translated to Rust: src/ini.rs new_patterns

// translated to Rust: src/ini.rs init_trie

/*:1001*/

// translated to Rust: src/ini.rs new_hyph_exceptions


// translated to Rust: src/ini.rs prefixed_command


/*1337:*/
void
store_fmt_file(void)
{
    int32_t j, k, l;
    int32_t p, q;
    int32_t x;
    rust_output_handle_t fmt_out;

    if (save_ptr != 0) {
        error_here_with_diagnostic("You can't dump inside a group");
        capture_to_diagnostic(NULL);
        help_ptr = 1;
        help_line[0] = "`{...\\dump}' is a no-no.";

        if (interaction == ERROR_STOP_MODE)
            interaction = SCROLL_MODE;
        if (log_opened)
            error();

        history = HISTORY_FATAL_ERROR;
        close_files_and_terminate();
        ttstub_output_flush (rust_stdout);
        _tt_abort("\\dump inside a group");
    }

    selector = SELECTOR_NEW_STRING;
    print_cstr(" (preloaded format=");
    print(job_name);
    print_char(' ');
    print_int(INTPAR(year));
    print_char('.');
    print_int(INTPAR(month));
    print_char('.');
    print_int(INTPAR(day));
    print_char(')');

    if (interaction == BATCH_MODE)
        selector = SELECTOR_LOG_ONLY;
    else
        selector = SELECTOR_TERM_AND_LOG;

    if (pool_ptr + 1 > pool_size)
        overflow("pool size", pool_size - init_pool_ptr);

    format_ident = make_string();
    pack_job_name(".fmt");

    fmt_out = ttstub_output_open (name_of_file, 0);
    if (fmt_out == INVALID_HANDLE)
        _tt_abort ("cannot open format output file \"%s\"", name_of_file);

    print_nl_cstr("Beginning to dump on file ");
    print(make_name_string());

    str_ptr--;
    pool_ptr = str_start[str_ptr - TOO_BIG_CHAR];

    print_nl_cstr("");
    print(format_ident);

    /* Header */

    dump_int(FORMAT_HEADER_MAGIC);
    dump_int(FORMAT_SERIAL);
    dump_int(hash_high);

    while (pseudo_files != TEX_NULL)
        pseudo_close(); /* TODO: can we move this farther up in this function? */

    dump_int(MEM_TOP);
    dump_int(EQTB_SIZE);
    dump_int(HASH_PRIME);
    dump_int(HYPH_PRIME);

    /* string pool */

    dump_int(pool_ptr);
    dump_int(str_ptr);
    dump_things(str_start[0], str_ptr - TOO_BIG_CHAR + 1);
    dump_things(str_pool[0], pool_ptr);

    print_ln();
    print_int(str_ptr);
    print_cstr(" strings of total length ");
    print_int(pool_ptr);

    /* "memory locations" */

    sort_avail();
    var_used = 0;
    dump_int(lo_mem_max);
    dump_int(rover);

    for (k = INT_VAL; k <= INTER_CHAR_VAL; k++)
        dump_int(sa_root[k]);

    p = 0;
    q = rover;
    x = 0;
    do {
        dump_things(mem[p], q + 2 - p);
        x = x + q + 2 - p;
        var_used = var_used + q - p;
        p = q + mem[q].b32.s0;
        q = mem[q + 1].b32.s1;
    } while (q != rover);

    var_used = var_used + lo_mem_max - p;
    dyn_used = mem_end + 1 - hi_mem_min;
    dump_things(mem[p], lo_mem_max + 1 - p);

    x = x + lo_mem_max + 1 - p;
    dump_int(hi_mem_min);
    dump_int(avail);
    dump_things(mem[hi_mem_min], mem_end + 1 - hi_mem_min);

    x = x + mem_end + 1 - hi_mem_min;
    p = avail;
    while (p != TEX_NULL) {
        dyn_used--;
        p = LLIST_link(p);
    }

    dump_int(var_used);
    dump_int(dyn_used);

    print_ln();
    print_int(x);
    print_cstr(" memory locations dumped; current usage is ");
    print_int(var_used);
    print_char('&');
    print_int(dyn_used);

    /* equivalents table / primitive */

    k = ACTIVE_BASE;

    do {
        j = k;

        while (j < INT_BASE - 1) {
            if (eqtb[j].b32.s1 == eqtb[j + 1].b32.s1 &&
                eqtb[j].b16.s1 == eqtb[j + 1].b16.s1 &&
                eqtb[j].b16.s0 == eqtb[j + 1].b16.s0)
                goto found1;
            j++;
        }

        l = INT_BASE;
        goto done1;

    found1:
        j++;
        l = j;

        while (j < INT_BASE - 1) {
            if (eqtb[j].b32.s1 != eqtb[j + 1].b32.s1 ||
                eqtb[j].b16.s1 != eqtb[j + 1].b16.s1 ||
                eqtb[j].b16.s0 != eqtb[j + 1].b16.s0)
                goto done1;
            j++;
        }
    done1:

        dump_int(l - k);
        dump_things(eqtb[k], l - k);
        k = j + 1;
        dump_int(k - l);
    } while (k != INT_BASE); /*:1350*/

    do {
        j = k;

        while (j < EQTB_SIZE) {
            if (eqtb[j].b32.s1 == eqtb[j + 1].b32.s1)
                goto found2;
            j++;
        }

        l = EQTB_SIZE + 1;
        goto done2;

    found2:
        j++;
        l = j;

        while (j < EQTB_SIZE) {
            if (eqtb[j].b32.s1 != eqtb[j + 1].b32.s1)
                goto done2;
            j++;
        }

    done2:
        dump_int(l - k);
        dump_things(eqtb[k], l - k);
        k = j + 1;
        dump_int(k - l);
    } while (k <= EQTB_SIZE);

    if (hash_high > 0)
        dump_things(eqtb[EQTB_SIZE + 1], hash_high);

    dump_int(par_loc);
    dump_int(write_loc);

    for (p = 0; p <= PRIM_SIZE; p++)
        dump_b32(prim[p]);

    /* control sequences */

    dump_int(hash_used);
    cs_count = (FROZEN_CONTROL_SEQUENCE - 1) - hash_used + hash_high;

    for (p = HASH_BASE; p <= hash_used; p++) {
        if (hash[p].s1 != 0) {
            dump_int(p);
            dump_b32(hash[p]);
            cs_count++;
        }
    }

    dump_things(hash[hash_used + 1], (UNDEFINED_CONTROL_SEQUENCE - 1) - hash_used);
    if (hash_high > 0)
        dump_things(hash[EQTB_SIZE + 1], hash_high);

    dump_int(cs_count);

    print_ln();
    print_int(cs_count);
    print_cstr(" multiletter control sequences");

    /* fonts */

    dump_int(fmem_ptr);
    dump_things(font_info[0], fmem_ptr);
    dump_int(font_ptr);
    dump_things(font_check[FONT_BASE], font_ptr + 1);
    dump_things(font_size[FONT_BASE], font_ptr + 1);
    dump_things(font_dsize[FONT_BASE], font_ptr + 1);
    dump_things(font_params[FONT_BASE], font_ptr + 1);
    dump_things(hyphen_char[FONT_BASE], font_ptr + 1);
    dump_things(skew_char[FONT_BASE], font_ptr + 1);
    dump_things(font_name[FONT_BASE], font_ptr + 1);
    dump_things(font_area[FONT_BASE], font_ptr + 1);
    dump_things(font_bc[FONT_BASE], font_ptr + 1);
    dump_things(font_ec[FONT_BASE], font_ptr + 1);
    dump_things(char_base[FONT_BASE], font_ptr + 1);
    dump_things(width_base[FONT_BASE], font_ptr + 1);
    dump_things(height_base[FONT_BASE], font_ptr + 1);
    dump_things(depth_base[FONT_BASE], font_ptr + 1);
    dump_things(italic_base[FONT_BASE], font_ptr + 1);
    dump_things(lig_kern_base[FONT_BASE], font_ptr + 1);
    dump_things(kern_base[FONT_BASE], font_ptr + 1);
    dump_things(exten_base[FONT_BASE], font_ptr + 1);
    dump_things(param_base[FONT_BASE], font_ptr + 1);
    dump_things(font_glue[FONT_BASE], font_ptr + 1);
    dump_things(bchar_label[FONT_BASE], font_ptr + 1);
    dump_things(font_bchar[FONT_BASE], font_ptr + 1);
    dump_things(font_false_bchar[FONT_BASE], font_ptr + 1);

    for (k = FONT_BASE; k <= font_ptr; k++) {
        print_nl_cstr("\\font");
        print_esc(hash[FONT_ID_BASE + k].s1);
        print_char('=');

        if (font_area[k] == AAT_FONT_FLAG || font_area[k] == OTGR_FONT_FLAG || font_mapping[k] != NULL) {
            print_file_name(font_name[k], EMPTY_STRING, EMPTY_STRING);

            error_here_with_diagnostic("Can't \\dump a format with native fonts or font-mappings");
            capture_to_diagnostic(NULL);

            help_ptr = 3;
            help_line[2] = "You really, really don't want to do this.";
            help_line[1] = "It won't work, and only confuses me.";
            help_line[0] = "(Load them at runtime, not as part of the format file.)";
            error();
        } else {
            print_file_name(font_name[k], font_area[k], EMPTY_STRING);
        }

        if (font_size[k] != font_dsize[k]) {
            print_cstr(" at ");
            print_scaled(font_size[k]);
            print_cstr("pt");
        }
    }

    print_ln();
    print_int(fmem_ptr - 7);
    print_cstr(" words of font info for ");
    print_int(font_ptr - 0);
    if (font_ptr != FONT_BASE + 1)
        print_cstr(" preloaded fonts");
    else
        print_cstr(" preloaded font");

    /* hyphenation info */

    dump_int(hyph_count);
    if (hyph_next <= HYPH_PRIME)
        hyph_next = hyph_size;
    dump_int(hyph_next);

    for (k = 0; k <= hyph_size; k++) {
        if (hyph_word[k] != 0) {
            dump_int(k + 65536L * hyph_link[k]);
            dump_int(hyph_word[k]);
            dump_int(hyph_list[k]);
        }
    }

    print_ln();
    print_int(hyph_count);
    if (hyph_count != 1)
        print_cstr(" hyphenation exceptions");
    else
        print_cstr(" hyphenation exception");

    if (trie_not_ready)
        init_trie();

    dump_int(trie_max);
    dump_int(hyph_start);
    dump_things(trie_trl[0], trie_max + 1);
    dump_things(trie_tro[0], trie_max + 1);
    dump_things(trie_trc[0], trie_max + 1);
    dump_int(max_hyph_char);
    dump_int(trie_op_ptr);
    dump_things(hyf_distance[1], trie_op_ptr);
    dump_things(hyf_num[1], trie_op_ptr);
    dump_things(hyf_next[1], trie_op_ptr);

    print_nl_cstr("Hyphenation trie of length ");
    print_int(trie_max);
    print_cstr(" has ");
    print_int(trie_op_ptr);
    if (trie_op_ptr != 1)
        print_cstr(" ops");
    else
        print_cstr(" op");
    print_cstr(" out of ");
    print_int(TRIE_OP_SIZE);

    for (k = BIGGEST_LANG; k >= 0; k--) {
        if (trie_used[k] > 0) {
            print_nl_cstr("  ");
            print_int(trie_used[k]);
            print_cstr(" for language ");
            print_int(k);
            dump_int(k);
            dump_int(trie_used[k]);
        }
    }

    /* footer */

    dump_int(FORMAT_FOOTER_MAGIC);

    INTPAR(tracing_stats) = 0; /*:1361*/
    ttstub_output_close(fmt_out);
}


static void
pack_buffered_name(small_number n, int32_t a, int32_t b)
{
    free(name_of_file);
    name_of_file = xmalloc_array(UTF8_code, format_default_length + 1);

    strcpy(name_of_file, TEX_format_default);
    name_length = strlen(name_of_file);
}


static bool
load_fmt_file(void)
{
    int32_t j, k;
    int32_t p, q;
    int32_t x;
    rust_input_handle_t fmt_in;

    j = cur_input.loc;

    /* This is where a first line starting with "&" used to
     * trigger code that would change the format file. */

    pack_buffered_name(format_default_length - 4, 1, 0);

    fmt_in = ttstub_input_open(name_of_file, TTBC_FILE_FORMAT_FORMAT, 0);
    if (fmt_in == INVALID_HANDLE)
        _tt_abort("cannot open the format file \"%s\"", name_of_file);

    cur_input.loc = j;

    if (in_initex_mode) {
        free(font_info);
        free(str_pool);
        free(str_start);
        free(yhash);
        free(eqtb);
        free(mem);
        mem = NULL;
    }

    /* start reading the header */

    undump_int(x);
    if (x != FORMAT_HEADER_MAGIC)
        goto bad_fmt;

    undump_int(x);
    if (x != FORMAT_SERIAL)
        _tt_abort("format file \"%s\" is of the wrong version: expected %d, found %d",
                  name_of_file, FORMAT_SERIAL, x);

    /* hash table parameters */

    undump_int(hash_high);
    if (hash_high < 0 || hash_high > sup_hash_extra)
        goto bad_fmt;
    if (hash_extra < hash_high)
        hash_extra = hash_high;

    eqtb_top = EQTB_SIZE + hash_extra;
    if (hash_extra == 0)
        hash_top = UNDEFINED_CONTROL_SEQUENCE;
    else
        hash_top = eqtb_top;

    yhash = xmalloc_array(b32x2, 1 + hash_top - hash_offset);
    hash = yhash - hash_offset;
    hash[HASH_BASE].s0 = 0;
    hash[HASH_BASE].s1 = 0;

    for (x = HASH_BASE + 1; x <= hash_top; x++)
        hash[x] = hash[HASH_BASE];

    eqtb = xmalloc_array(memory_word, eqtb_top + 1);
    eqtb[UNDEFINED_CONTROL_SEQUENCE].b16.s1 = UNDEFINED_CS;
    eqtb[UNDEFINED_CONTROL_SEQUENCE].b32.s1 = TEX_NULL;
    eqtb[UNDEFINED_CONTROL_SEQUENCE].b16.s0 = LEVEL_ZERO;

    for (x = EQTB_SIZE + 1; x <= eqtb_top; x++)
        eqtb[x] = eqtb[UNDEFINED_CONTROL_SEQUENCE];

    max_reg_num = 32767;
    max_reg_help_line = "A register number must be between 0 and 32767.";

    /* "memory locations" */

    undump_int(x);
    if (x != MEM_TOP)
        goto bad_fmt;

    cur_list.head = CONTRIB_HEAD;
    cur_list.tail = CONTRIB_HEAD;
    page_tail = PAGE_HEAD;
    mem = xmalloc_array(memory_word, MEM_TOP + 1);

    undump_int(x);
    if (x != EQTB_SIZE)
        goto bad_fmt;

    undump_int(x);
    if (x != HASH_PRIME)
        goto bad_fmt;

    undump_int(x);
    if (x != HYPH_PRIME)
        goto bad_fmt;

    /* string pool */

    undump_int(x);
    if (x < 0)
        goto bad_fmt;
    if (x > sup_pool_size - pool_free)
        _tt_abort ("must increase string_pool_size");
    pool_ptr = x;

    if (pool_size < pool_ptr + pool_free)
        pool_size = pool_ptr + pool_free;

    undump_int(x);
    if (x < 0)
        goto bad_fmt;
    if (x > sup_max_strings - strings_free)
        _tt_abort ("must increase sup_strings");
    str_ptr = x;

    if (max_strings < str_ptr + strings_free)
        max_strings = str_ptr + strings_free;

    str_start = xmalloc_array(pool_pointer, max_strings);
    undump_checked_things(0, pool_ptr, str_start[0], str_ptr - TOO_BIG_CHAR + 1);
    str_pool = xmalloc_array(packed_UTF16_code, pool_size);

    undump_things(str_pool[0], pool_ptr);

    init_str_ptr = str_ptr;
    init_pool_ptr = pool_ptr; /*:1345 */

    /* "By sorting the list of available spaces in the variable-size portion
     * of |mem|, we are usually able to get by without having to dump very
     * much of the dynamic memory." */

    undump_int(x);
    if (x < 1019 || x > MEM_TOP - HI_MEM_STAT_USAGE)
        goto bad_fmt;
    else
        lo_mem_max = x;

    undump_int(x);
    if (x < 20 || x > lo_mem_max)
        goto bad_fmt;
    else
        rover = x;

    for (k = INT_VAL; k <= INTER_CHAR_VAL; k++) {
        undump_int(x);
        if (x < MIN_HALFWORD || x > lo_mem_max)
            goto bad_fmt;
        else
            sa_root[k] = x;
    }

    p = 0;
    q = rover;

    do {
        undump_things(mem[p], q + 2 - p);
        p = q + mem[q].b32.s0;
        if (p > lo_mem_max || (q >= mem[q + 1].b32.s1 && mem[q + 1].b32.s1 != rover))
            goto bad_fmt;
        q = mem[q + 1].b32.s1;
    } while (q != rover);

    undump_things(mem[p], lo_mem_max + 1 - p);

    undump_int(x);
    if (x < lo_mem_max + 1 || x > PRE_ADJUST_HEAD)
        goto bad_fmt;
    else
        hi_mem_min = x;

    undump_int(x);
    if (x < MIN_HALFWORD || x > MEM_TOP)
        goto bad_fmt;
    else
        avail = x;

    mem_end = MEM_TOP;

    undump_things(mem[hi_mem_min], mem_end + 1 - hi_mem_min);
    undump_int(var_used);
    undump_int(dyn_used);

    /* equivalents table / primitives
     *
     * "The table of equivalents usually contains repeated information, so we
     * dump it in compressed form: The sequence of $n + 2$ values
     * $(n, x_1, \ldots, x_n, m)$ in the format file represents $n + m$ consecutive
     * entries of |eqtb|, with |m| extra copies of $x_n$, namely
     * $(x_1, \ldots, x_n, x_n, \ldots, x_n)$"
     */

    k = ACTIVE_BASE;

    do {
        undump_int(x);
        if (x < 1 || k + x > EQTB_SIZE + 1)
            goto bad_fmt;

        undump_things(eqtb[k], x);
        k = k + x;

        undump_int(x);
        if (x < 0 || k + x > EQTB_SIZE + 1)
            goto bad_fmt;

        for (j = k; j <= k + x - 1; j++)
            eqtb[j] = eqtb[k - 1];

        k = k + x;
    } while (k <= EQTB_SIZE);

    if (hash_high > 0)
        undump_things(eqtb[EQTB_SIZE + 1], hash_high);

    undump_int(x);
    if (x < HASH_BASE || x > hash_top)
        goto bad_fmt;
    else
        par_loc = x;

    par_token = CS_TOKEN_FLAG + par_loc;

    undump_int(x);
    if (x < HASH_BASE || x > hash_top)
        goto bad_fmt;
    else
        write_loc = x;

    /* control sequence names
     *
     * "A different scheme is used to compress the hash table, since its lower
     * region is usually sparse. When |text(p) != 0| for |p <= hash_used|, we
     * output two words, |p| and |hash[p]|. The hash table is, of course,
     * densely packed for |p >= hash_used|, so the remaining entries are
     * output in a block."
     */

    for (p = 0; p <= PRIM_SIZE; p++)
        undump_b32(prim[p]);

    undump_int(x);
    if (x < HASH_BASE || x > FROZEN_CONTROL_SEQUENCE)
        goto bad_fmt;
    else
        hash_used = x;

    p = HASH_BASE - 1;

    do {
        undump_int(x);
        if (x < p + 1 || x > hash_used)
            goto bad_fmt;
        else
            p = x;
        undump_b32(hash[p]);
    } while (p != hash_used);

    undump_things(hash[hash_used + 1], (UNDEFINED_CONTROL_SEQUENCE - 1) - hash_used);

    if (hash_high > 0)
        undump_things(hash[EQTB_SIZE + 1], hash_high);

    undump_int(cs_count);

    /* font info */

    undump_int(x);
    if (x < 7)
        goto bad_fmt;
    if (x > sup_font_mem_size)
        _tt_abort ("must increase font_mem_size");

    fmem_ptr = x;
    if (fmem_ptr > font_mem_size)
        font_mem_size = fmem_ptr;

    font_info = xmalloc_array(memory_word, font_mem_size);
    undump_things(font_info[0], fmem_ptr);

    undump_int(x);
    if (x < FONT_BASE)
        goto bad_fmt;
    if (x > FONT_BASE + MAX_FONT_MAX)
        _tt_abort ("must increase font_max");

    font_ptr = x;

    font_mapping = xcalloc_array(void *, font_max);
    font_layout_engine = xcalloc_array(void *, font_max);
    font_flags = xmalloc_array(char, font_max);
    font_letter_space = xmalloc_array(scaled_t, font_max);
    font_check = xmalloc_array(b16x4, font_max);
    font_size = xmalloc_array(scaled_t, font_max);
    font_dsize = xmalloc_array(scaled_t, font_max);
    font_params = xmalloc_array(font_index, font_max);
    font_name = xmalloc_array(str_number, font_max);
    font_area = xmalloc_array(str_number, font_max);
    font_bc = xmalloc_array(UTF16_code, font_max);
    font_ec = xmalloc_array(UTF16_code, font_max);
    font_glue = xmalloc_array(int32_t, font_max);
    hyphen_char = xmalloc_array(int32_t, font_max);
    skew_char = xmalloc_array(int32_t, font_max);
    bchar_label = xmalloc_array(font_index, font_max);
    font_bchar = xmalloc_array(nine_bits, font_max);
    font_false_bchar = xmalloc_array(nine_bits, font_max);
    char_base = xmalloc_array(int32_t, font_max);
    width_base = xmalloc_array(int32_t, font_max);
    height_base = xmalloc_array(int32_t, font_max);
    depth_base = xmalloc_array(int32_t, font_max);
    italic_base = xmalloc_array(int32_t, font_max);
    lig_kern_base = xmalloc_array(int32_t, font_max);
    kern_base = xmalloc_array(int32_t, font_max);
    exten_base = xmalloc_array(int32_t, font_max);
    param_base = xmalloc_array(int32_t, font_max);

    for (k = FONT_BASE; k <= font_ptr; k++)
        font_mapping[k] = 0;

    undump_things(font_check[FONT_BASE], font_ptr + 1);
    undump_things(font_size[FONT_BASE], font_ptr + 1);
    undump_things(font_dsize[FONT_BASE], font_ptr + 1);
    undump_checked_things(MIN_HALFWORD, MAX_HALFWORD, font_params[FONT_BASE], font_ptr + 1);
    undump_things(hyphen_char[FONT_BASE], font_ptr + 1);
    undump_things(skew_char[FONT_BASE], font_ptr + 1);
    undump_upper_check_things(str_ptr, font_name[FONT_BASE], font_ptr + 1);
    undump_upper_check_things(str_ptr, font_area[FONT_BASE], font_ptr + 1);
    undump_things(font_bc[FONT_BASE], font_ptr + 1);
    undump_things(font_ec[FONT_BASE], font_ptr + 1);
    undump_things(char_base[FONT_BASE], font_ptr + 1);
    undump_things(width_base[FONT_BASE], font_ptr + 1);
    undump_things(height_base[FONT_BASE], font_ptr + 1);
    undump_things(depth_base[FONT_BASE], font_ptr + 1);
    undump_things(italic_base[FONT_BASE], font_ptr + 1);
    undump_things(lig_kern_base[FONT_BASE], font_ptr + 1);
    undump_things(kern_base[FONT_BASE], font_ptr + 1);
    undump_things(exten_base[FONT_BASE], font_ptr + 1);
    undump_things(param_base[FONT_BASE], font_ptr + 1);
    undump_checked_things(MIN_HALFWORD, lo_mem_max, font_glue[FONT_BASE], font_ptr + 1);
    undump_checked_things(0, fmem_ptr - 1, bchar_label[FONT_BASE], font_ptr + 1);
    undump_checked_things(0, TOO_BIG_CHAR, font_bchar[FONT_BASE], font_ptr + 1);
    undump_checked_things(0, TOO_BIG_CHAR, font_false_bchar[FONT_BASE], font_ptr + 1);

    /* hyphenations */

    undump_int(x);
    if (x < 0)
        goto bad_fmt;
    if (x > hyph_size)
        _tt_abort ("must increase hyph_size");
    hyph_count = x;

    undump_int(x);
    if (x < HYPH_PRIME)
        goto bad_fmt;
    if (x > hyph_size)
        _tt_abort ("must increase hyph_size");
    hyph_next = x;

    j = 0;

    for (k = 1; k <= hyph_count; k++) {
        undump_int(j);
        if (j < 0)
            goto bad_fmt;
        if (j > 65535L) {
            hyph_next = j / 65536L;
            j = j - hyph_next * 65536L;
        } else {
            hyph_next = 0;
        }

        if (j >= hyph_size || hyph_next > hyph_size)
            goto bad_fmt;

        hyph_link[j] = hyph_next;

        undump_int(x);
        if (x < 0 || x > str_ptr)
            goto bad_fmt;
        else
            hyph_word[j] = x;

        undump_int(x);
        if (x < MIN_HALFWORD || x > MAX_HALFWORD)
            goto bad_fmt;
        else
            hyph_list[j] = x;
    }

    j++;
    if (j < HYPH_PRIME)
        j = HYPH_PRIME;

    hyph_next = j;
    if (hyph_next >= hyph_size)
        hyph_next = HYPH_PRIME;
    else if (hyph_next >= HYPH_PRIME)
        hyph_next++;

    undump_int(x);
    if (x < 0)
        goto bad_fmt;
    if (x > trie_size)
        _tt_abort ("must increase trie_size");

    j = x;
    trie_max = j;

    undump_int(x);
    if (x < 0 || x > j)
        goto bad_fmt;
    else
        hyph_start = x;

    if (!trie_trl)
        trie_trl = xmalloc_array(trie_pointer, j + 1);
    undump_things(trie_trl[0], j + 1);

    if (!trie_tro)
        trie_tro = xmalloc_array(trie_pointer, j + 1);
    undump_things(trie_tro[0], j + 1);

    if (!trie_trc)
        trie_trc = xmalloc_array(uint16_t, j + 1);
    undump_things(trie_trc[0], j + 1);

    undump_int(max_hyph_char);

    undump_int(x);
    if (x < 0)
        goto bad_fmt;
    if (x > TRIE_OP_SIZE)
        _tt_abort ("must increase TRIE_OP_SIZE");

    j = x;
    trie_op_ptr = j;

    undump_things(hyf_distance[1], j);
    undump_things(hyf_num[1], j);
    undump_upper_check_things(MAX_TRIE_OP, hyf_next[1], j);

    for (k = 0; k <= BIGGEST_LANG; k++)
        trie_used[k] = 0;

    k = BIGGEST_LANG + 1;

    while (j > 0) {
        undump_int(x);
        if (x < 0 || x > k - 1)
            goto bad_fmt;
        else
            k = x;

        undump_int(x);
        if (x < 1 || x > j)
            goto bad_fmt;

        trie_used[k] = x;
        j = j - x;
        op_start[k] = j;
    }

    trie_not_ready = false;

    /* trailer */

    undump_int(x);
    if (x != FORMAT_FOOTER_MAGIC)
        goto bad_fmt;

    ttstub_input_close (fmt_in);
    return true;

bad_fmt:
    _tt_abort ("fatal format file error");
}


// translated to Rust: src/ini.rs final_cleanup


/* Engine initialization */

void init_io(void);
// translated to Rust: src/ini.rs init_io


// translated to Rust: src/ini.rs initialize_more_variables

// translated to Rust: src/ini.rs initialize_more_initex_variables


/*:1370*//*1371: */
const xetex_format_primitive_def_t xetex_primitives[] = { XETEX_FORMAT_PRIMITIVE_INITIALIZERS };
const size_t xetex_primitives_count = sizeof(xetex_primitives) / sizeof(xetex_primitives[0]);

// translated to Rust: src/ini.rs initialize_primitives


void get_strings_started(void);
// translated to Rust: src/ini.rs get_strings_started
/*:1001*/

// translated to Rust: src/ini.rs tt_cleanup
// Helper for tt_cleanup: free the file-static yhash pointer.
void
tt_cleanup_free_yhash(void) {
    free(yhash);
    yhash = NULL;
}

tt_history_t
tt_run_engine(const char *dump_name, const char *input_file_name, time_t build_date)
{
    int32_t font_k;

    /* Miscellaneous initializations that were mostly originally done in the
     * main() driver routines. */

    /* Get our stdout handle */

    rust_stdout = ttstub_output_open_stdout ();

    size_t len = strlen (dump_name);
    TEX_format_default = xmalloc (len + 1);
    strcpy (TEX_format_default, dump_name);
    format_default_length = len;

    /* Not sure why these get custom initializations. */

    if (file_line_error_style_p < 0)
        file_line_error_style_p = 0;

    /* These various parameters were configurable in web2c TeX. We don't
     * bother to allow that. */

    pool_size = 6250000L;
    string_vacancies = 90000L;
    pool_free = 47500L;
    max_strings = 565536L;
    strings_free = 100;
    font_mem_size = 8000000L;
    font_max = 9000;
    trie_size = 1000000L;
    hyph_size = 8191;
    buf_size = 200000L;
    nest_size = 500;
    max_in_open = 15;
    param_size = 10000;
    save_size = 80000L;
    stack_size = 5000;
    error_line = 79;
    half_error_line = 50;
    max_print_line = 79;
    hash_extra = 600000L;
    expand_depth = 10000;

    /* Allocate many of our big arrays. */

    buffer = xmalloc_array(UnicodeScalar, buf_size);
    nest = xmalloc_array(list_state_record, nest_size);
    save_stack = xmalloc_array(memory_word, save_size);
    input_stack = xmalloc_array(input_state_t, stack_size);
    input_file = xmalloc_array(UFILE *, max_in_open);
    line_stack = xmalloc_array(int32_t, max_in_open);
    eof_seen = xmalloc_array(bool, max_in_open);
    grp_stack = xmalloc_array(save_pointer, max_in_open);
    if_stack = xmalloc_array(int32_t, max_in_open);
    source_filename_stack = xmalloc_array(str_number, max_in_open);
    full_source_filename_stack = xmalloc_array(str_number, max_in_open);
    param_stack = xmalloc_array(int32_t, param_size);
    hyph_word = xmalloc_array(str_number, hyph_size);
    hyph_list = xmalloc_array(int32_t, hyph_size);
    hyph_link = xmalloc_array(hyph_pointer, hyph_size);

    /* First bit of initex handling: more allocations. */

    if (in_initex_mode) {
        mem = xmalloc_array(memory_word, MEM_TOP + 1);
        eqtb_top = EQTB_SIZE + hash_extra;

        if (hash_extra == 0)
            hash_top = UNDEFINED_CONTROL_SEQUENCE;
        else
            hash_top = eqtb_top;

        yhash = xmalloc_array(b32x2, 1 + hash_top - hash_offset);
        hash = yhash - hash_offset;
        hash[HASH_BASE].s0 = 0;
        hash[HASH_BASE].s1 = 0;

        for (hash_used = HASH_BASE + 1; hash_used <= hash_top; hash_used++)
            hash[hash_used] = hash[HASH_BASE];

        eqtb = xcalloc_array(memory_word, eqtb_top);
        str_start = xmalloc_array(pool_pointer, max_strings);
        str_pool = xmalloc_array(packed_UTF16_code, pool_size);
        font_info = xmalloc_array(memory_word, font_mem_size);
    }

    /* Sanity-check various invariants. */

    history = HISTORY_FATAL_ERROR;
    bad = 0;

    if (half_error_line < 30 || half_error_line > error_line - 15)
        bad = 1;
    if (max_print_line < 60)
        bad = 2;
    if (1100 > MEM_TOP)
        bad = 4;
    if (HASH_PRIME > HASH_SIZE)
        bad = 5;
    if (max_in_open >= 128)
        bad = 6;
    if (MEM_TOP < 267)
        bad = 7;
    if (MIN_HALFWORD > 0)
        bad = 12;
    if (MAX_FONT_MAX < MIN_HALFWORD || MAX_FONT_MAX > MAX_HALFWORD)
        bad = 15;
    if (font_max > FONT_BASE + 9000)
        bad = 16;
    if (save_size > MAX_HALFWORD || max_strings > MAX_HALFWORD)
        bad = 17;
    if (buf_size > MAX_HALFWORD)
        bad = 18;
    if (CS_TOKEN_FLAG + EQTB_SIZE + hash_extra > MAX_HALFWORD)
        bad = 21;
    if (hash_offset < 0 || hash_offset > HASH_BASE)
        bad = 42;
    if (format_default_length > INT32_MAX)
        bad = 31;
    if (2 * MAX_HALFWORD < MEM_TOP)
        bad = 41;

    if (bad > 0)
        _tt_abort ("failed internal consistency check #%d", bad);

    /* OK, ready to keep on initializing. */

    initialize_more_variables();

    if (in_initex_mode) {
        get_strings_started();
        initialize_more_initex_variables();
        initialize_primitives();
        init_str_ptr = str_ptr;
        init_pool_ptr = pool_ptr;
    }

    /*55:*/
    initialize_math_variables();
    initialize_pagebuilder_variables();
    initialize_shipout_variables();

    get_seconds_and_micros(&epochseconds, &microseconds);
    init_start_time(build_date);

    selector = SELECTOR_TERM_ONLY;
    tally = 0;
    term_offset = 0;
    file_offset = 0;
    job_name = 0;
    name_in_progress = false;
    log_opened = false;

    if (semantic_pagination_enabled)
        output_file_extension = ".spx";
    else
        output_file_extension = ".xdv";

    input_ptr = 0;
    max_in_stack = 0;
    source_filename_stack[0] = 0;
    full_source_filename_stack[0] = 0;
    in_open = 0;
    open_parens = 0;
    max_buf_stack = 0;
    grp_stack[0] = 0;
    if_stack[0] = TEX_NULL;
    param_ptr = 0;
    max_param_stack = 0;
    used_tectonic_coda_tokens = false;
    gave_char_warning_help = false;

    memset(buffer, 0, buf_size * sizeof(buffer[0]));
    first = 0;

    scanner_status = NORMAL;
    warning_index = TEX_NULL;
    first = 1;
    cur_input.state = NEW_LINE;
    cur_input.start = 1;
    cur_input.index = 0;
    line = 0;
    cur_input.name = 0;
    force_eof = false;
    align_state = 1000000L;

    init_io();

    if (in_initex_mode) {
        no_new_control_sequence = false;
        max_reg_num = 32767;
        max_reg_help_line = "A register number must be between 0 and 32767.";
    }

    no_new_control_sequence = true;

    if (!in_initex_mode) {
        if (!load_fmt_file())
            return history;
    }

    if (INTPAR(end_line_char) < 0 || INTPAR(end_line_char) > BIGGEST_CHAR)
        cur_input.limit--;
    else
        buffer[cur_input.limit] = INTPAR(end_line_char);

    if (in_initex_mode) {
        /* TeX initializes with the real date and time, but for format file
         * reproducibility we do this: */
        INTPAR(time) = 0;
        INTPAR(day) = 0;
        INTPAR(month) = 0;
        INTPAR(year) = 0;
    } else {
        get_date_and_time(build_date,
                          &(INTPAR(time)),
                          &(INTPAR(day)),
                          &(INTPAR(month)),
                          &(INTPAR(year)));
    }

    if (trie_not_ready) {
        trie_trl = xmalloc_array(trie_pointer, trie_size);
        trie_tro = xmalloc_array(trie_pointer, trie_size);
        trie_trc = xmalloc_array(uint16_t, trie_size);
        trie_c = xmalloc_array(packed_UTF16_code, trie_size);
        trie_o = xmalloc_array(trie_opcode, trie_size);
        trie_l = xmalloc_array(trie_pointer, trie_size);
        trie_r = xmalloc_array(trie_pointer, trie_size);
        trie_hash = xmalloc_array(trie_pointer, trie_size);
        trie_taken = xmalloc_array(bool, trie_size);
        trie_l[0] = 0;
        trie_c[0] = 0;
        trie_ptr = 0;
        trie_r[0] = 0;
        hyph_start = 0;
        font_mapping = xcalloc_array(void *, font_max);
        font_layout_engine = xcalloc_array(void *, font_max);
        font_flags = xcalloc_array(char, font_max);
        font_letter_space = xcalloc_array(scaled_t, font_max);
        font_check = xcalloc_array(b16x4, font_max);
        font_size = xcalloc_array(scaled_t, font_max);
        font_dsize = xcalloc_array(scaled_t, font_max);
        font_params = xcalloc_array(font_index, font_max);
        font_name = xcalloc_array(str_number, font_max);
        font_area = xcalloc_array(str_number, font_max);
        font_bc = xcalloc_array(UTF16_code, font_max);
        font_ec = xcalloc_array(UTF16_code, font_max);
        font_glue = xcalloc_array(int32_t, font_max);
        hyphen_char = xcalloc_array(int32_t, font_max);
        skew_char = xcalloc_array(int32_t, font_max);
        bchar_label = xcalloc_array(font_index, font_max);
        font_bchar = xcalloc_array(nine_bits, font_max);
        font_false_bchar = xcalloc_array(nine_bits, font_max);
        char_base = xcalloc_array(int32_t, font_max);
        width_base = xcalloc_array(int32_t, font_max);
        height_base = xcalloc_array(int32_t, font_max);
        depth_base = xcalloc_array(int32_t, font_max);
        italic_base = xcalloc_array(int32_t, font_max);
        lig_kern_base = xcalloc_array(int32_t, font_max);
        kern_base = xcalloc_array(int32_t, font_max);
        exten_base = xcalloc_array(int32_t, font_max);
        param_base = xcalloc_array(int32_t, font_max);
        font_ptr = FONT_BASE;
        fmem_ptr = 7;
        font_name[FONT_BASE] = maketexstring("nullfont");
        font_area[FONT_BASE] = EMPTY_STRING;
        hyphen_char[FONT_BASE] = '-';
        skew_char[FONT_BASE] = -1;
        bchar_label[FONT_BASE] = NON_ADDRESS;
        font_bchar[FONT_BASE] = TOO_BIG_CHAR;
        font_false_bchar[FONT_BASE] = TOO_BIG_CHAR;
        font_bc[FONT_BASE] = 1;
        font_ec[FONT_BASE] = 0;
        font_size[FONT_BASE] = 0;
        font_dsize[FONT_BASE] = 0;
        char_base[FONT_BASE] = 0;
        width_base[FONT_BASE] = 0;
        height_base[FONT_BASE] = 0;
        depth_base[FONT_BASE] = 0;
        italic_base[FONT_BASE] = 0;
        lig_kern_base[FONT_BASE] = 0;
        kern_base[FONT_BASE] = 0;
        exten_base[FONT_BASE] = 0;
        font_glue[FONT_BASE] = TEX_NULL;
        font_params[FONT_BASE] = 7;
        font_mapping[FONT_BASE] = 0;
        param_base[FONT_BASE] = -1;

        for (font_k = 0; font_k <= 6; font_k++)
            font_info[font_k].b32.s1 = 0;
    }

    font_used = xmalloc_array(bool, font_max);
    for (font_k = 0; font_k <= font_max; font_k++)
        font_used[font_k] = false;

    random_seed = (microseconds * 1000) + (epochseconds % 1000000L);
    init_randoms(random_seed);

    if (interaction == BATCH_MODE)
        selector = SELECTOR_NO_PRINT;
    else
        selector = SELECTOR_TERM_ONLY; /*:79*/

    if (semantic_pagination_enabled)
        INTPAR(xetex_generate_actual_text) = 1;

    pdf_files_init();
    synctex_init_command();
    start_input(input_file_name);
    history = HISTORY_SPOTLESS;
    main_control();
    final_cleanup();
    close_files_and_terminate();

    tt_cleanup();

    return history;
}
