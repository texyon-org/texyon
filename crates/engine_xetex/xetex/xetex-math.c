/* Copyright 2016-2018 The Tectonic Project
 * Licensed under the MIT License.
 */

#include "xetex-core.h"
#include "xetex-xetexd.h"
#include "xetex-synctex.h"
#include "tectonic_bridge_core.h"


scaled_t math_x_height(int32_t size_code);
scaled_t math_quad(int32_t size_code);
scaled_t num1(int32_t size_code);
scaled_t num2(int32_t size_code);
scaled_t num3(int32_t size_code);
scaled_t denom1(int32_t size_code);
scaled_t denom2(int32_t size_code);
scaled_t sup1(int32_t size_code);
scaled_t sup2(int32_t size_code);
scaled_t sup3(int32_t size_code);
scaled_t sub1(int32_t size_code);
scaled_t sub2(int32_t size_code);
scaled_t sup_drop(int32_t size_code);
scaled_t sub_drop(int32_t size_code);
scaled_t delim1(int32_t size_code);
scaled_t delim2(int32_t size_code);
scaled_t axis_height(int32_t size_code);
scaled_t default_rule_thickness(void);
scaled_t big_op_spacing1(void);
scaled_t big_op_spacing2(void);
scaled_t big_op_spacing3(void);
scaled_t big_op_spacing4(void);
scaled_t big_op_spacing5(void);
void scan_delimiter(int32_t p, bool r);
int32_t fraction_rule(scaled_t t);
int32_t overbar(int32_t b, scaled_t k, scaled_t t);
int32_t char_box(internal_font_number f, int32_t c);
void stack_into_box(int32_t b, internal_font_number f, uint16_t c);
scaled_t height_plus_depth(internal_font_number f, uint16_t c);
void stack_glyph_into_box(int32_t b, internal_font_number f, int32_t g);
void stack_glue_into_box(int32_t b, scaled_t min, scaled_t max);
int32_t build_opentype_assembly(internal_font_number f, void *a, scaled_t s, bool horiz);
int32_t var_delimiter(int32_t d, int32_t s, scaled_t v);
int32_t rebox(int32_t b, scaled_t w);
int32_t math_glue(int32_t g, scaled_t m);
void math_kern(int32_t p, scaled_t m);
int32_t clean_box(int32_t p, small_number s);
void fetch(int32_t a);
void make_over(int32_t q);
void make_under(int32_t q);
void make_vcenter(int32_t q);
void make_radical(int32_t q);
scaled_t compute_ot_math_accent_pos(int32_t p);
void make_math_accent(int32_t q);
void make_fraction(int32_t q);
scaled_t make_op(int32_t q);
void make_ord(int32_t q);
int32_t attach_hkern_to_new_hlist(int32_t q, scaled_t delta);
void make_scripts(int32_t q, scaled_t delta);
small_number make_left_right(int32_t q, small_number style, scaled_t max_d, scaled_t max_h);
void mlist_to_hlist(void);


/* cur_mlist, cur_style, cur_size, cur_mu, mlist_penalties moved to
 * EngineState.scanner (session 48). */

// translated to Rust: src/math.rs initialize_math_variables
// translated to Rust: src/math.rs make_math_accent


// translated to Rust: src/math.rs make_op


// translated to Rust: src/math.rs make_ord


// translated to Rust: src/math.rs attach_hkern_to_new_hlist
// translated to Rust: src/math.rs make_scripts


// translated to Rust: src/math.rs make_left_right
// translated to Rust: src/math.rs mlist_to_hlist


// translated to Rust: src/math.rs var_delimiter

// translated to Rust: src/math.rs char_box
// translated to Rust: src/math.rs stack_into_box
// translated to Rust: src/math.rs height_plus_depth
// translated to Rust: src/math.rs stack_glyph_into_box
// translated to Rust: src/math.rs stack_glue_into_box


// translated to Rust: src/math.rs build_opentype_assembly

// translated to Rust: src/math.rs rebox
