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
memory_word *eqtb;
int32_t bad;
char *name_of_file;
UTF16_code *name_of_file16;
int32_t name_length;
int32_t name_length16;
int32_t first;
int32_t last;
int32_t max_buf_stack;
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
packed_UTF16_code *str_pool;
pool_pointer *str_start;
pool_pointer pool_ptr;
str_number str_ptr;
pool_pointer init_pool_ptr;
str_number init_str_ptr;
rust_output_handle_t rust_stdout;
rust_output_handle_t log_file;
UTF16_code *native_text;
int32_t native_text_size;
int32_t native_len;
int32_t save_native_len;
bool set_box_allowed;
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
int32_t nest_ptr;
int32_t max_nest_stack;
list_state_record cur_list;
short shown_mode;
b32x2 *hash;
int32_t hash_extra;
int32_t hash_top;
int32_t eqtb_top;
b32x2 prim[PRIM_SIZE + 1];
int32_t mag_set;
eight_bits cur_cmd;
int32_t cur_chr;
int32_t cur_cs;
int32_t cur_tok;
int32_t input_ptr;
int32_t max_in_stack;
input_state_t cur_input;
int32_t in_open;
int32_t open_parens;
int32_t line;
unsigned char scanner_status;
int32_t warning_index;
int32_t def_ref;
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
int32_t format_default_length;
char *TEX_format_default;
str_number job_name;
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
bool doing_leaders;
scaled_t rule_ht, rule_dp, rule_wd;
scaled_t cur_h, cur_v;
scaled_t total_stretch[4], total_shrink[4];
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
str_number format_ident;
int32_t write_loc;
scaled_t cur_page_width;
scaled_t cur_page_height;
scaled_t cur_h_offset;
scaled_t cur_v_offset;
int32_t LR_ptr;
int32_t LR_problems;
small_number cur_dir;
int32_t pseudo_files;
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
int32_t native_font_type_flag;
bool xtx_ligature_present;
scaled_t delta;
bool used_tectonic_coda_tokens;
bool gave_char_warning_help;

uint16_t _xeq_level_array[EQTB_SIZE - INT_BASE + 1];

#define NEG_TRIE_OP_SIZE -35111L
#define MAX_TRIE_OP 65535L

extern int32_t _trie_op_hash_array[TRIE_OP_SIZE - NEG_TRIE_OP_SIZE + 1];
#define TRIE_OP_HASH(i) _trie_op_hash_array[(i) - NEG_TRIE_OP_SIZE]

static b32x2 *yhash;

#define FORMAT_HEADER_MAGIC 0x54544E43 /* "TTNC" in ASCII */
#define FORMAT_FOOTER_MAGIC 0x0000029A

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


// translated to Rust: src/ini.rs store_fmt_file
// translated to Rust: src/ini.rs pack_buffered_name
// translated to Rust: src/ini.rs load_fmt_file


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

// Helper for Rust tt_run_engine: allocate the file-static yhash pointer and
// wire the public hash pointer just as the C body did.
void
tt_run_engine_allocate_yhash(void) {
    yhash = xmalloc_array(b32x2, 1 + hash_top - hash_offset);
    hash = yhash - hash_offset;
}

// translated to Rust: src/ini.rs tt_run_engine
