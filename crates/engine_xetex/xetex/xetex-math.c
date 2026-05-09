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


int32_t cur_mlist;
small_number cur_style;
int32_t cur_size;
scaled_t cur_mu;
bool mlist_penalties;

// translated to Rust: src/math.rs initialize_math_variables
// translated to Rust: src/math.rs make_math_accent


// translated to Rust: src/math.rs make_op


// translated to Rust: src/math.rs make_ord


// translated to Rust: src/math.rs attach_hkern_to_new_hlist
// translated to Rust: src/math.rs make_scripts


// translated to Rust: src/math.rs make_left_right


void
mlist_to_hlist(void)
{
    int32_t mlist;
    bool penalties;
    small_number style;
    small_number save_style;
    int32_t q;
    int32_t r;
    small_number r_type;
    small_number t;
    int32_t p = TEX_NULL, x, y, z;
    int32_t pen;
    small_number s;
    scaled_t max_h, max_d;
    scaled_t delta;

    mlist = cur_mlist;
    penalties = mlist_penalties;
    style = cur_style;
    q = mlist;
    r = TEX_NULL;
    r_type = OP_NOAD;
    max_h = 0;
    max_d = 0;
    {
        if (cur_style < SCRIPT_STYLE)
            cur_size = TEXT_SIZE;
        else
            cur_size = SCRIPT_SIZE * ((cur_style - 2) / 2);
        cur_mu = x_over_n(math_quad(cur_size), 18);
    }
    while (q != TEX_NULL) {  /*753: */

    reswitch:
        delta = 0;
        switch (mem[q].b16.s1) {
        case BIN_NOAD:
            switch (r_type) {
            case BIN_NOAD:
            case OP_NOAD:
            case REL_NOAD:
            case OPEN_NOAD:
            case PUNCT_NOAD:
            case LEFT_NOAD:
                mem[q].b16.s1 = ORD_NOAD;
                goto reswitch;
            default:
                break;
            }
            break;
        case REL_NOAD:
        case CLOSE_NOAD:
        case PUNCT_NOAD:
        case RIGHT_NOAD:
            {
                if (r_type == BIN_NOAD)
                    mem[r].b16.s1 = ORD_NOAD /*:755 */ ;
                if (mem[q].b16.s1 == RIGHT_NOAD)
                    goto lab80;
            }
            break;
        case LEFT_NOAD:
            goto lab80;
            break;
        case FRACTION_NOAD:
            make_fraction(q);
            goto lab82;
        case OP_NOAD:
            delta = make_op(q);
            if (mem[q].b16.s0 == LIMITS)
                goto lab82;
            break;
        case ORD_NOAD:
            make_ord(q);
            break;
        case OPEN_NOAD:
        case INNER_NOAD:
            break;
        case RADICAL_NOAD:
            make_radical(q);
            break;
        case OVER_NOAD:
            make_over(q);
            break;
        case UNDER_NOAD:
            make_under(q);
            break;
        case ACCENT_NOAD:
            make_math_accent(q);
            break;
        case VCENTER_NOAD:
            make_vcenter(q);
            break;
        case STYLE_NODE:
            {
                cur_style = mem[q].b16.s0;
                {
                    if (cur_style < SCRIPT_STYLE)
                        cur_size = TEXT_SIZE;
                    else
                        cur_size = SCRIPT_SIZE * ((cur_style - 2) / 2);
                    cur_mu = x_over_n(math_quad(cur_size), 18);
                }
                goto lab81;
            }
            break;
        case CHOICE_NODE:
            {
                switch (cur_style / 2) {
                case 0:
                    {
                        p = mem[q + 1].b32.s0;
                        mem[q + 1].b32.s0 = TEX_NULL;
                    }
                    break;
                case 1:
                    {
                        p = mem[q + 1].b32.s1;
                        mem[q + 1].b32.s1 = TEX_NULL;
                    }
                    break;
                case 2:
                    {
                        p = mem[q + 2].b32.s0;
                        mem[q + 2].b32.s0 = TEX_NULL;
                    }
                    break;
                case 3:
                    {
                        p = mem[q + 2].b32.s1;
                        mem[q + 2].b32.s1 = TEX_NULL;
                    }
                    break;
                }
                flush_node_list(mem[q + 1].b32.s0);
                flush_node_list(mem[q + 1].b32.s1);
                flush_node_list(mem[q + 2].b32.s0);
                flush_node_list(mem[q + 2].b32.s1);
                NODE_type(q) = STYLE_NODE;
                mem[q].b16.s0 = cur_style;
                mem[q + 1].b32.s1 = 0;
                mem[q + 2].b32.s1 = 0;
                if (p != TEX_NULL) {
                    z = mem[q].b32.s1;
                    mem[q].b32.s1 = p;
                    while (mem[p].b32.s1 != TEX_NULL)
                        p = LLIST_link(p);
                    mem[p].b32.s1 = z;
                }
                goto lab81;
            }
            break;
        case INS_NODE:
        case MARK_NODE:
        case ADJUST_NODE:
        case WHATSIT_NODE:
        case PENALTY_NODE:
        case DISC_NODE:
            goto lab81;
        case RULE_NODE:
            if (mem[q + 3].b32.s1 > max_h)
                max_h = mem[q + 3].b32.s1;
            if (mem[q + 2].b32.s1 > max_d)
                max_d = mem[q + 2].b32.s1;
            goto lab81;
        case GLUE_NODE:
            {
                if (mem[q].b16.s0 == MU_GLUE) {
                    x = mem[q + 1].b32.s0;
                    y = math_glue(x, cur_mu);
                    delete_glue_ref(x);
                    mem[q + 1].b32.s0 = y;
                    mem[q].b16.s0 = NORMAL;
                } else if ((cur_size != TEXT_SIZE) && (mem[q].b16.s0 == COND_MATH_GLUE)) {
                    p = mem[q].b32.s1;
                    if (p != TEX_NULL) {

                        if ((NODE_type(p) == GLUE_NODE) || (NODE_type(p) == KERN_NODE)) {
                            mem[q].b32.s1 = mem[p].b32.s1;
                            mem[p].b32.s1 = TEX_NULL;
                            flush_node_list(p);
                        }
                    }
                }
                goto lab81;
            }
            break;
        case KERN_NODE:
            {
                math_kern(q, cur_mu);
                goto lab81;
            }
            break;
        default:
            confusion("mlist1");
            break;
        }
        switch (mem[q + 1].b32.s1) {
        case 1:
        case 4:
            {
                fetch(q + 1);
                if (((font_area[cur_f] == AAT_FONT_FLAG)
                     || (font_area[cur_f] == OTGR_FONT_FLAG))) {
                    z = new_native_character(cur_f, cur_c);
                    p = get_node(GLYPH_NODE_SIZE);
                    NODE_type(p) = WHATSIT_NODE;
                    mem[p].b16.s0 = GLYPH_NODE;
                    mem[p + 4].b16.s2 = cur_f;
                    mem[p + 4].b16.s1 = get_native_glyph(z, 0);
                    set_native_glyph_metrics(p, 1);
                    free_node(z, mem[z + 4].b16.s3);
                    delta = get_ot_math_ital_corr(cur_f, mem[p + 4].b16.s1);
                    if ((mem[q + 1].b32.s1 == MATH_TEXT_CHAR)
                        &&
                        (!((font_area[cur_f] == OTGR_FONT_FLAG)
                           && (isOpenTypeMathFont(font_layout_engine[cur_f]))) != 0))
                        delta = 0;
                    if ((mem[q + 3].b32.s1 == EMPTY) && (delta != 0)) {
                        mem[p].b32.s1 = new_kern(delta);
                        delta = 0;
                    }
                } else if ((cur_i.s3 > 0)) {
                    delta = FONT_CHARINFO_ITALCORR(cur_f, cur_i);
                    p = new_character(cur_f, cur_c);
                    if ((mem[q + 1].b32.s1 == MATH_TEXT_CHAR)
                        && (font_info[SPACE_CODE + param_base[cur_f]].b32.s1 != 0))
                        delta = 0;
                    if ((mem[q + 3].b32.s1 == EMPTY) && (delta != 0)) {
                        mem[p].b32.s1 = new_kern(delta);
                        delta = 0;
                    }
                } else
                    p = TEX_NULL;
            }
            break;
        case 0:
            p = TEX_NULL;
            break;
        case 2:
            p = mem[q + 1].b32.s0;
            break;
        case 3:
            {
                cur_mlist = mem[q + 1].b32.s0;
                save_style = cur_style;
                mlist_penalties = false;
                mlist_to_hlist();
                cur_style = save_style;
                {
                    if (cur_style < SCRIPT_STYLE)
                        cur_size = TEXT_SIZE;
                    else
                        cur_size = SCRIPT_SIZE * ((cur_style - 2) / 2);
                    cur_mu = x_over_n(math_quad(cur_size), 18);
                }
                p = hpack(mem[TEMP_HEAD].b32.s1, 0, ADDITIONAL);
            }
            break;
        default:
            confusion("mlist2");
            break;
        }
        mem[q + 1].b32.s1 = p;
        if ((mem[q + 3].b32.s1 == EMPTY) && (mem[q + 2].b32.s1 == EMPTY))
            goto lab82;
        make_scripts(q, delta);
 lab82:/*check_dimensions */ z = hpack(mem[q + 1].b32.s1, 0, ADDITIONAL);
        if (mem[z + 3].b32.s1 > max_h)
            max_h = mem[z + 3].b32.s1;
        if (mem[z + 2].b32.s1 > max_d)
            max_d = mem[z + 2].b32.s1;
        free_node(z, BOX_NODE_SIZE);
 lab80:                        /*done_with_noad */ r = q;
        r_type = mem[r].b16.s1;
        if (r_type == RIGHT_NOAD) {
            r_type = LEFT_NOAD;
            cur_style = style;
            {
                if (cur_style < SCRIPT_STYLE)
                    cur_size = TEXT_SIZE;
                else
                    cur_size = SCRIPT_SIZE * ((cur_style - 2) / 2);
                cur_mu = x_over_n(math_quad(cur_size), 18);
            }
        }
 lab81:                        /*done_with_node */ q = LLIST_link(q);
    }
    if (r_type == BIN_NOAD)
        mem[r].b16.s1 = 16 /*ord_noad *//*:755 */ ;
    p = TEMP_HEAD;
    mem[p].b32.s1 = TEX_NULL;
    q = mlist;
    r_type = 0;
    cur_style = style;
    {
        if (cur_style < SCRIPT_STYLE)
            cur_size = TEXT_SIZE;
        else
            cur_size = SCRIPT_SIZE * ((cur_style - 2) / 2);
        cur_mu = x_over_n(math_quad(cur_size), 18);
    }
    while (q != TEX_NULL) {

        t = ORD_NOAD;
        s = NOAD_SIZE;
        pen = INF_PENALTY;
        switch (mem[q].b16.s1) {
        case OP_NOAD:
        case OPEN_NOAD:
        case CLOSE_NOAD:
        case PUNCT_NOAD:
        case INNER_NOAD:
            t = mem[q].b16.s1;
            break;
        case BIN_NOAD:
            {
                t = BIN_NOAD;
                pen = INTPAR(bin_op_penalty);
            }
            break;
        case REL_NOAD:
            {
                t = REL_NOAD;
                pen = INTPAR(rel_penalty);
            }
            break;
        case ORD_NOAD:
        case VCENTER_NOAD:
        case OVER_NOAD:
        case UNDER_NOAD:
            break;
        case RADICAL_NOAD:
            s = RADICAL_NOAD_SIZE;
            break;
        case ACCENT_NOAD:
            s = ACCENT_NOAD_SIZE;
            break;
        case FRACTION_NOAD:
            s = FRACTION_NOAD_SIZE;
            break;
        case LEFT_NOAD:
        case RIGHT_NOAD:
            t = make_left_right(q, style, max_d, max_h);
            break;
        case STYLE_NODE:
            {
                cur_style = mem[q].b16.s0;
                s = STYLE_NODE_SIZE;
                {
                    if (cur_style < SCRIPT_STYLE)
                        cur_size = TEXT_SIZE;
                    else
                        cur_size = SCRIPT_SIZE * ((cur_style - 2) / 2);
                    cur_mu = x_over_n(math_quad(cur_size), 18);
                }
                goto lab83;
            }
            break;
        case WHATSIT_NODE:
        case PENALTY_NODE:
        case RULE_NODE:
        case DISC_NODE:
        case ADJUST_NODE:
        case INS_NODE:
        case MARK_NODE:
        case GLUE_NODE:
        case KERN_NODE:
            {
                mem[p].b32.s1 = q;
                p = q;
                q = LLIST_link(q);
                mem[p].b32.s1 = TEX_NULL;
                goto done;
            }
            break;
        default:
            confusion("mlist3");
            break;
        }
        if (r_type > 0) {
            const char* offset_table[] = {
                "02340001",
                "22*40001",
                "33**3**3",
                "44*04004",
                "00*00000",
                "02340001",
                "11*11111",
                "12341011"
            };
            // The inter-element spacing in math formulas depends on a 8x8 table.
            // The table indices range from ORD_NOAD to INNER_NOAD.
            // The chars of this table have the following significance:
            switch (offset_table[r_type - ORD_NOAD][t - ORD_NOAD]) {
            case '0': // no space
                x = 0;
                break;
            case '1': // a conditional thin space
                if (cur_style < SCRIPT_STYLE)
                    x = GLUE_PAR__thin_mu_skip;
                else
                    x = 0;
                break;
            case '2': // a thin space
                x = GLUE_PAR__thin_mu_skip;
                break;
            case '3': // a conditional medium space
                if (cur_style < SCRIPT_STYLE)
                    x = GLUE_PAR__med_mu_skip;
                else
                    x = 0;
                break;
            case '4': // a conditional thick space
                if (cur_style < SCRIPT_STYLE)
                    x = GLUE_PAR__thick_mu_skip;
                else
                    x = 0;
                break;
            default: // impossible
                confusion("mlist4");
                break;
            }
            if (x != 0) {
                y = math_glue(eqtb[GLUE_BASE + x].b32.s1, cur_mu);
                z = new_glue(y);
                mem[y].b32.s1 = TEX_NULL;
                mem[p].b32.s1 = z;
                p = z;
                mem[z].b16.s0 = x + 1;
            }
        }
        if (mem[q + 1].b32.s1 != TEX_NULL) {
            mem[p].b32.s1 = mem[q + 1].b32.s1;
            do {
                p = LLIST_link(p);
            } while (!(mem[p].b32.s1 == TEX_NULL));
        }
        if (penalties) {

            if (mem[q].b32.s1 != TEX_NULL) {

                if (pen < INF_PENALTY) {
                    r_type = mem[mem[q].b32.s1].b16.s1;
                    if (r_type != PENALTY_NODE) {

                        if (r_type != REL_NOAD) {
                            z = new_penalty(pen);
                            mem[p].b32.s1 = z;
                            p = z;
                        }
                    }
                }
            }
        }
        if (mem[q].b16.s1 == RIGHT_NOAD)
            t = OPEN_NOAD;
        r_type = t;
 lab83:                        /*delete_q */ r = q;
        q = LLIST_link(q);
        free_node(r, s);
    done:
        ;
    }
}


// translated to Rust: src/math.rs var_delimiter

// translated to Rust: src/math.rs char_box
// translated to Rust: src/math.rs stack_into_box
// translated to Rust: src/math.rs height_plus_depth
// translated to Rust: src/math.rs stack_glyph_into_box
// translated to Rust: src/math.rs stack_glue_into_box


// translated to Rust: src/math.rs build_opentype_assembly

// translated to Rust: src/math.rs rebox
