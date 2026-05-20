/* Copyright 2016-2018 The Tectonic Project
 * Licensed under the MIT License.
 */

#include "xetex-core.h"
#include "xetex-xetexd.h"
#include "xetex-synctex.h"
#include "tectonic_bridge_core.h"


#define DVI_BUF_SIZE 16384
#define HALF_BUF 8192
#define FNT_NUM_0 171 /* DVI code */

/* Promoted to Rust statics in src/shipout.rs (session 111): */
extern rust_output_handle_t dvi_file;
extern str_number output_file_name;
extern eight_bits *dvi_buf;
extern int32_t dvi_limit;
extern int32_t dvi_ptr;
extern int32_t dvi_offset;
extern int32_t dvi_gone;
extern int32_t down_ptr, right_ptr;
extern int32_t cur_s;
/* Promoted to Rust statics in src/shipout.rs (session 113): */
extern scaled_t dvi_h;
extern scaled_t dvi_v;
/* Promoted to Rust statics in src/shipout.rs (session 115): */
extern internal_font_number dvi_f;
void hlist_out(void);
void vlist_out(void);
int32_t reverse(int32_t this_box, int32_t t, scaled_t * cur_g, double * cur_glue);
void movement(scaled_t w, eight_bits o);
void prune_movements(int32_t l);
void pic_out(int32_t p);
/* dvi_swap translated to Rust (only called from dvi_out, also in Rust) */
void write_to_dvi(int32_t a, int32_t b);
void dvi_out(eight_bits c);
void dvi_four(int32_t x);
void dvi_two(UTF16_code s);
void dvi_pop(int32_t l);
void dvi_font_def(internal_font_number f);


// translated to Rust: src/shipout.rs initialize_shipout_variables
// translated to Rust: src/shipout.rs deinitialize_shipout_variables
// translated to Rust: src/shipout.rs dvi_out


// translated to Rust: src/shipout.rs ship_out


// translated to Rust: src/shipout.rs hlist_out


// translated to Rust: src/shipout.rs vlist_out


/*1510: "The reverse function defined here is responsible for reversing the
 * nodes of an hlist (segment). this_box is the enclosing hlist_node; t is to
 * become the tail of the reversed list; and the global variable temp_ptr is
 * the head of the list to be reversed. cur_g and cur_glue are the current
 * glue rounding state variables, to be updated by this function. We remove
 * nodes from the original list and add them to the head of the new one."
 */
// translated to Rust: src/shipout.rs reverse


// translated to Rust: src/shipout.rs out_what

// translated to Rust: src/shipout.rs dvi_native_font_def
// translated to Rust: src/shipout.rs dvi_font_def


// translated to Rust: src/shipout.rs movement
// translated to Rust: src/shipout.rs special_out
// translated to Rust: src/shipout.rs write_out
// translated to Rust: src/shipout.rs pic_out


// translated to Rust: src/shipout.rs finalize_dvi_file

// translated to Rust: src/shipout.rs write_to_dvi
// translated to Rust: src/shipout.rs dvi_swap
// translated to Rust: src/shipout.rs dvi_four
// translated to Rust: src/shipout.rs dvi_two
// translated to Rust: src/shipout.rs dvi_pop
// translated to Rust: src/shipout.rs prune_movements
