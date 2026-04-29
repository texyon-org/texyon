/* xetex-xetex0.c: bulk of the WEB code translated to C
   Copyright 2016-2022 The Tectonic Project
   Licensed under the MIT License.
*/

#include "xetex-core.h"
#include "xetex-xetexd.h"
#include "xetex-synctex.h"

#include <stdio.h> /* for EOF */

#define IS_LC_HEX(c) (((c) >= '0' && (c) <= '9' ) || ((c) >= 'a' && (c) <= 'f' ))

/* int_error translated to Rust (src/errors.rs). */


/*:112*/
/* show_token_list: translated to Rust in src/output.rs */
#if 0
/*118:*/
void
show_token_list(int32_t p, int32_t q, int32_t l)
{
    int32_t m, c;
    int32_t match_chr;
    UTF16_code n;

    match_chr = '#' ;
    n = '0' ;
    tally = 0;

    while (p != TEX_NULL && tally < l) {
        /*332:*/
        if (p == q) {
            first_count = tally;
            trick_count = tally + 1 + error_line - half_error_line;
            if (trick_count < error_line)
                trick_count = error_line;
        }

        if (p < hi_mem_min || p > mem_end) {
            print_esc_cstr("CLOBBERED.");
            return;
        }

        if (mem[p].b32.s0 >= CS_TOKEN_FLAG) {
            print_cs(mem[p].b32.s0 - CS_TOKEN_FLAG);
        } else {
            m = mem[p].b32.s0 / MAX_CHAR_VAL;
            c = mem[p].b32.s0 % MAX_CHAR_VAL;

            if (mem[p].b32.s0 < 0) {
                print_esc_cstr("BAD.");
            } else {
                /*306:*/
                switch (m) {
                case LEFT_BRACE:
                case RIGHT_BRACE:
                case MATH_SHIFT:
                case TAB_MARK:
                case SUP_MARK:
                case SUB_MARK:
                case SPACER:
                case LETTER:
                case OTHER_CHAR:
                    print_char(c);
                    break;
                case MAC_PARAM:
                    print_char(c);
                    print_char(c);
                    break;
                case OUT_PARAM:
                    print_char(match_chr);
                    if (c <= 9) {
                        print_char(c + 48);
                    } else {
                        print_char('!');
                        return;
                    }
                    break;
                case MATCH:
                    match_chr = c;
                    print_char(c);
                    n++;
                    print_char(n);
                    if (n > '9' )
                        return;
                    break;
                case END_MATCH:
                    if (c == 0)
                        print_cstr("->");
                    break;
                default:
                    print_esc_cstr("BAD.");
                    break;
                }
            }
        }

        p = LLIST_link(p);
    }

    if (p != TEX_NULL)
        print_esc_cstr("ETC.");
}
#endif


/* runaway: translated to Rust in src/scan.rs */


/*:165*/

/* prev_rightmost translated to Rust (src/mem.rs). */
/* get_microinterval translated to Rust (src/texmfmp.rs). */

/* short_display: translated to Rust in src/scan.rs */
#if 0
void
short_display(int32_t p)
{
    int32_t n;

    while (p > 0) {
        if (is_char_node(p)) {
            if (p <= mem_end) {
                if (mem[p].b16.s1 != font_in_short_display) {
                    if (mem[p].b16.s1 > font_max)
                        print_char('*');
                    else /*279:*/
                        print_esc(hash[FONT_ID_BASE + mem[p].b16.s1].s1);
                    print_char(' ');
                    font_in_short_display = mem[p].b16.s1;
                }
                print(mem[p].b16.s0);
            }
        } else {
            /*183:*/
            switch (mem[p].b16.s1) {
            case HLIST_NODE:
            case VLIST_NODE:
            case INS_NODE:
            case MARK_NODE:
            case ADJUST_NODE:
            case UNSET_NODE:
                print_cstr("[]");
                break;
            case WHATSIT_NODE:
                switch (mem[p].b16.s0) {
                case NATIVE_WORD_NODE:
                case NATIVE_WORD_NODE_AT:
                    if (mem[p + 4].b16.s2 != font_in_short_display) {
                        print_esc(hash[FONT_ID_BASE + mem[p + 4].b16.s2].s1);
                        print_char(' ');
                        font_in_short_display = mem[p + 4].b16.s2;
                    }
                    print_native_word(p);
                    break;
                default:
                    print_cstr("[]");
                    break;
                }
                break;
            case RULE_NODE:
                print_char('|');
                break;
            case GLUE_NODE:
                if (mem[p + 1].b32.s0 != 0)
                    print_char(' ');
                break;
            case MATH_NODE:
                if (mem[p].b16.s0 >= L_CODE)
                    print_cstr("[]");
                else
                    print_char('$');
                break;
            case LIGATURE_NODE:
                short_display(mem[p + 1].b32.s1);
                break;
            case DISC_NODE:
                short_display(mem[p + 1].b32.s0);
                short_display(mem[p + 1].b32.s1);
                n = mem[p].b16.s0;

                while (n > 0) {
                    if (mem[p].b32.s1 != TEX_NULL)
                        p = LLIST_link(p);
                    n--;
                }
                break;
            default:
                break;
            }
        }

        p = LLIST_link(p);
    }
}
#endif


/* print_font_and_char translated to Rust (src/output.rs). */
/* print_mark translated to Rust (src/output.rs). */


/* print_rule_dimen translated to Rust (src/output.rs). */
/* print_glue translated to Rust (src/output.rs). */

/* print_spec translated to Rust (src/output.rs). */

/* print_fam_and_char translated to Rust (src/output.rs). */
/* print_delimiter translated to Rust (src/output.rs). */


void
print_subsidiary_data(int32_t p, UTF16_code c)
{

    if (cur_length() >= depth_threshold) {
        if (mem[p].b32.s1 != EMPTY)
            print_cstr(" []");
    } else {
        str_pool[pool_ptr] = c;
        pool_ptr++;
        temp_ptr = p;

        switch (mem[p].b32.s1) {
        case MATH_CHAR:
            print_ln();
            print_current_string();
            print_fam_and_char(p);
            break;
        case SUB_BOX:
            show_info();
            break;
        case SUB_MLIST:
            if (mem[p].b32.s0 == TEX_NULL) {
                print_ln();
                print_current_string();
                print_cstr("{}");
            } else {
                show_info();
            }
            break;
        default:
            break;
        }

        pool_ptr--;
    }
}


/* print_style translated to Rust (src/output.rs). */

/* print_skip_param translated to Rust (src/output.rs). */


void
show_node_list(int32_t p)
{
    int32_t n;
    int32_t i;
    double g;

    if (cur_length() > depth_threshold) {
        if (p > TEX_NULL)
            print_cstr(" []");
        return;
    }

    n = 0;

    while (p > 0) {
        print_ln();
        print_current_string();

        if (p > mem_end) {
            print_cstr("Bad link, display aborted.");
            return;
        }

        n++;

        if (n > breadth_max) {
            print_cstr("etc.");
            return;
        }

        if (is_char_node(p)) {
            print_font_and_char(p);
        } else {
            switch (mem[p].b16.s1) {
            case HLIST_NODE:
            case VLIST_NODE:
            case UNSET_NODE:
                if (NODE_type(p) == HLIST_NODE)
                    print_esc('h' );
                else if (NODE_type(p) == VLIST_NODE)
                    print_esc('v' );
                else
                    print_esc_cstr("unset");

                print_cstr("box(");
                print_scaled(mem[p + 3].b32.s1);
                print_char('+');
                print_scaled(mem[p + 2].b32.s1);
                print_cstr(")x");
                print_scaled(mem[p + 1].b32.s1);

                if (NODE_type(p) == UNSET_NODE) { /*193:*/
                    if (mem[p].b16.s0 != 0) {
                        print_cstr(" (");
                        print_int(mem[p].b16.s0 + 1);
                        print_cstr(" columns)");
                    }
                    if (mem[p + 6].b32.s1 != 0) {
                        print_cstr(", stretch ");
                        print_glue(mem[p + 6].b32.s1, mem[p + 5].b16.s0, NULL);
                    }
                    if (mem[p + 4].b32.s1 != 0) {
                        print_cstr(", shrink ");
                        print_glue(mem[p + 4].b32.s1, mem[p + 5].b16.s1, NULL);
                    }
                } else {
                    g = BOX_glue_set(p);

                    if (g != 0.0 && mem[p + 5].b16.s1 != NORMAL) {
                        print_cstr(", glue set ");
                        if (mem[p + 5].b16.s1 == SHRINKING)
                            print_cstr("- ");

                        if (fabs(g) > 20000.0) {
                            if (g > 0.0)
                                print_char('>');
                            else
                                print_cstr("< -");
                            print_glue(20000 * 65536L, mem[p + 5].b16.s0, NULL);
                        } else {
                            print_glue(tex_round(65536L * g), mem[p + 5].b16.s0, NULL);
                        }
                    }

                    if (mem[p + 4].b32.s1 != 0) {
                        print_cstr(", shifted ");
                        print_scaled(mem[p + 4].b32.s1);
                    }

                    /*1491:*/
                    if (NODE_type(p) == HLIST_NODE && mem[p].b16.s0 == DLIST)
                        print_cstr(", display");
                }

                str_pool[pool_ptr] = '.' ;
                pool_ptr++;
                show_node_list(mem[p + 5].b32.s1);
                pool_ptr--;
                break;

            case RULE_NODE:
                print_esc_cstr("rule(");
                print_rule_dimen(mem[p + 3].b32.s1);
                print_char('+');
                print_rule_dimen(mem[p + 2].b32.s1);
                print_cstr(")x");
                print_rule_dimen(mem[p + 1].b32.s1);
                break;

            case INS_NODE:
                print_esc_cstr("insert");
                print_int(mem[p].b16.s0);
                print_cstr(", natural size ");
                print_scaled(mem[p + 3].b32.s1);
                print_cstr("; split(");
                print_spec(mem[p + 4].b32.s1, NULL);
                print_char(',');
                print_scaled(mem[p + 2].b32.s1);
                print_cstr("); float cost ");
                print_int(mem[p + 1].b32.s1);
                str_pool[pool_ptr] = '.' ;
                pool_ptr++;
                show_node_list(mem[p + 4].b32.s0);
                pool_ptr--;
                break;

            case WHATSIT_NODE:
                switch (mem[p].b16.s0) {
                case OPEN_NODE:
                    print_write_whatsit("openout", p);
                    print_char('=');
                    print_file_name(mem[p + 1].b32.s1, mem[p + 2].b32.s0, mem[p + 2].b32.s1);
                    break;
                case WRITE_NODE:
                    print_write_whatsit("write", p);
                    print_mark(mem[p + 1].b32.s1);
                    break;
                case CLOSE_NODE:
                    print_write_whatsit("closeout", p);
                    break;
                case SPECIAL_NODE:
                    print_esc_cstr("special");
                    print_mark(mem[p + 1].b32.s1);
                    break;
                case LANGUAGE_NODE:
                    print_esc_cstr("setlanguage");
                    print_int(mem[p + 1].b32.s1);
                    print_cstr(" (hyphenmin ");
                    print_int(mem[p + 1].b16.s1);
                    print_char(',');
                    print_int(mem[p + 1].b16.s0);
                    print_char(')');
                    break;
                case PDF_SAVE_POS_NODE:
                    print_esc_cstr("pdfsavepos");
                    break;
                case NATIVE_WORD_NODE:
                case NATIVE_WORD_NODE_AT:
                    print_esc(hash[FONT_ID_BASE + mem[p + 4].b16.s2].s1);
                    print_char(' ');
                    print_native_word(p);
                    break;
                case GLYPH_NODE:
                    print_esc(hash[FONT_ID_BASE + mem[p + 4].b16.s2].s1);
                    print_cstr(" glyph#");
                    print_int(mem[p + 4].b16.s1);
                    break;
                case PIC_NODE:
                case PDF_NODE:
                    if (mem[p].b16.s0 == PIC_NODE)
                        print_esc_cstr("XeTeXpicfile");
                    else
                        print_esc_cstr("XeTeXpdffile");

                    print_cstr("( ");
                    for (i = 0; i < PIC_NODE_path_len(p); i++)
                        print_raw_char(PIC_NODE_path(p)[i], true);
                    print('"');
                    break;
                default:
                    print_cstr("whatsit?");
                    break;
                }
                break; /* WHATSIT_NODE */

            case GLUE_NODE:
                if (mem[p].b16.s0 >= A_LEADERS) {      /*198: */
                    print_esc_cstr("");
                    if (mem[p].b16.s0 == C_LEADERS)
                        print_char('c');
                    else if (mem[p].b16.s0 == X_LEADERS)
                        print_char('x');
                    print_cstr("leaders ");
                    print_spec(mem[p + 1].b32.s0, NULL);
                    str_pool[pool_ptr] = '.' ;
                    pool_ptr++;
                    show_node_list(mem[p + 1].b32.s1);
                    pool_ptr--;
                } else {
                    print_esc_cstr("glue");

                    if (GLUE_SPEC_shrink_order(p) != NORMAL) {
                        print_char('(');
                        if (mem[p].b16.s0 < COND_MATH_GLUE)
                            print_skip_param(mem[p].b16.s0 - 1);
                        else if (mem[p].b16.s0 == COND_MATH_GLUE)
                            print_esc_cstr("nonscript");
                        else
                            print_esc_cstr("mskip");
                        print_char(')');
                    }

                    if (mem[p].b16.s0 != COND_MATH_GLUE) {
                        print_char(' ');
                        if (mem[p].b16.s0 < COND_MATH_GLUE)
                            print_spec(mem[p + 1].b32.s0, NULL);
                        else
                            print_spec(mem[p + 1].b32.s0, "mu");
                    }
                }
                break;

            case KERN_NODE:
                if (mem[p].b16.s0 != MU_GLUE) {
                    print_esc_cstr("kern");
                    if (mem[p].b16.s0 != NORMAL)
                        print_char(' ');
                    print_scaled(mem[p + 1].b32.s1);
                    if (NODE_subtype(p) == ACC_KERN)
                        print_cstr(" (for accent)");
                    else if (NODE_subtype(p) == SPACE_ADJUSTMENT)
                        print_cstr(" (space adjustment)");
                } else {
                    print_esc_cstr("mkern");
                    print_scaled(mem[p + 1].b32.s1);
                    print_cstr("mu");
                }
                break;

            case MARGIN_KERN_NODE:
                print_esc_cstr("kern");
                print_scaled(mem[p + 1].b32.s1);
                if (mem[p].b16.s0 == 0)
                    print_cstr(" (left margin)");
                else
                    print_cstr(" (right margin)");
                break;

            case MATH_NODE:
                if (mem[p].b16.s0 > AFTER) {
                    if (odd(mem[p].b16.s0))
                        print_esc_cstr("end");
                    else
                        print_esc_cstr("begin");
                    if (mem[p].b16.s0 > R_CODE)
                        print_char('R');
                    else if (mem[p].b16.s0 > L_CODE)
                        print_char('L');
                    else
                        print_char('M');
                } else {
                    print_esc_cstr("math");
                    if (mem[p].b16.s0 == BEFORE)
                        print_cstr("on");
                    else
                        print_cstr("off");
                    if (mem[p + 1].b32.s1 != 0) {
                        print_cstr(", surrounded ");
                        print_scaled(mem[p + 1].b32.s1);
                    }
                }
                break;

            case LIGATURE_NODE:
                print_font_and_char(p + 1);
                print_cstr(" (ligature ");
                if (mem[p].b16.s0 > 1)
                    print_char('|');
                font_in_short_display = mem[p + 1].b16.s1;
                short_display(mem[p + 1].b32.s1);
                if (odd(mem[p].b16.s0))
                    print_char('|');
                print_char(')');
                break;

            case PENALTY_NODE:
                print_esc_cstr("penalty ");
                print_int(mem[p + 1].b32.s1);
                break;

            case DISC_NODE:
                print_esc_cstr("discretionary");
                if (mem[p].b16.s0 > 0) {
                    print_cstr(" replacing ");
                    print_int(mem[p].b16.s0);
                }

                str_pool[pool_ptr] = '.' ;
                pool_ptr++;
                show_node_list(mem[p + 1].b32.s0);
                pool_ptr--;
                str_pool[pool_ptr] = '|' ;
                pool_ptr++;
                show_node_list(mem[p + 1].b32.s1);
                pool_ptr--;
                break;

            case MARK_NODE:
                print_esc_cstr("mark");
                if (mem[p + 1].b32.s0 != 0) {
                    print_char('s');
                    print_int(mem[p + 1].b32.s0);
                }
                print_mark(mem[p + 1].b32.s1);
                break;

            case ADJUST_NODE:
                print_esc_cstr("vadjust");
                if (mem[p].b16.s0 != 0)
                    print_cstr(" pre ");

                str_pool[pool_ptr] = '.' ;
                pool_ptr++;
                show_node_list(mem[p + 1].b32.s1);
                pool_ptr--;
                break;

            case STYLE_NODE:
                print_style(mem[p].b16.s0);
                break;

            case CHOICE_NODE:
                print_esc_cstr("mathchoice");
                str_pool[pool_ptr] = 'D' ;
                pool_ptr++;
                show_node_list(mem[p + 1].b32.s0);
                pool_ptr--;
                str_pool[pool_ptr] = 'T' ;
                pool_ptr++;
                show_node_list(mem[p + 1].b32.s1);
                pool_ptr--;
                str_pool[pool_ptr] = 'S' ;
                pool_ptr++;
                show_node_list(mem[p + 2].b32.s0);
                pool_ptr--;
                str_pool[pool_ptr] = 's' ;
                pool_ptr++;
                show_node_list(mem[p + 2].b32.s1);
                pool_ptr--;
                break;

            case ORD_NOAD:
            case OP_NOAD:
            case BIN_NOAD:
            case REL_NOAD:
            case OPEN_NOAD:
            case CLOSE_NOAD:
            case PUNCT_NOAD:
            case INNER_NOAD:
            case RADICAL_NOAD:
            case OVER_NOAD:
            case UNDER_NOAD:
            case VCENTER_NOAD:
            case ACCENT_NOAD:
            case LEFT_NOAD:
            case RIGHT_NOAD:
                {
                    switch (mem[p].b16.s1) {
                    case ORD_NOAD:
                        print_esc_cstr("mathord");
                        break;
                    case OP_NOAD:
                        print_esc_cstr("mathop");
                        break;
                    case BIN_NOAD:
                        print_esc_cstr("mathbin");
                        break;
                    case REL_NOAD:
                        print_esc_cstr("mathrel");
                        break;
                    case OPEN_NOAD:
                        print_esc_cstr("mathopen");
                        break;
                    case CLOSE_NOAD:
                        print_esc_cstr("mathclose");
                        break;
                    case PUNCT_NOAD:
                        print_esc_cstr("mathpunct");
                        break;
                    case INNER_NOAD:
                        print_esc_cstr("mathinner");
                        break;
                    case OVER_NOAD:
                        print_esc_cstr("overline");
                        break;
                    case UNDER_NOAD:
                        print_esc_cstr("underline");
                        break;
                    case VCENTER_NOAD:
                        print_esc_cstr("vcenter");
                        break;
                    case RADICAL_NOAD:
                        print_esc_cstr("radical");
                        print_delimiter(p + 4);
                        break;
                    case ACCENT_NOAD:
                        print_esc_cstr("accent");
                        print_fam_and_char(p + 4);
                        break;
                    case LEFT_NOAD:
                        print_esc_cstr("left");
                        print_delimiter(p + 1);
                        break;
                    case RIGHT_NOAD:
                        if (mem[p].b16.s0 == NORMAL)
                            print_esc_cstr("right");
                        else
                            print_esc_cstr("middle");
                        print_delimiter(p + 1);
                        break;
                    }

                    if (mem[p].b16.s1 < LEFT_NOAD) {
                        if (mem[p].b16.s0 != NORMAL) {
                            if (mem[p].b16.s0 == LIMITS)
                                print_esc_cstr("limits");
                            else
                                print_esc_cstr("nolimits");
                        }
                        print_subsidiary_data(p + 1, '.' );
                    }

                    print_subsidiary_data(p + 2, '^' );
                    print_subsidiary_data(p + 3, '_' );
                }
                break; /* many math noads */

            case FRACTION_NOAD:
                print_esc_cstr("fraction, thickness ");
                if (mem[p + 1].b32.s1 == DEFAULT_CODE)
                    print_cstr("= default");
                else
                    print_scaled(mem[p + 1].b32.s1);

                if (mem[p + 4].b16.s3 % 256 != 0 ||
                    (mem[p + 4].b16.s2 + (mem[p + 4].b16.s3 / 256) * 65536L) != 0 ||
                    mem[p + 4].b16.s1 % 256 != 0 ||
                    (mem[p + 4].b16.s0 + (mem[p + 4].b16.s1 / 256) * 65536L) != 0) {
                    print_cstr(", left-delimiter ");
                    print_delimiter(p + 4);
                }

                if (mem[p + 5].b16.s3 % 256 != 0 ||
                    (mem[p + 5].b16.s2 + (mem[p + 5].b16.s3 / 256) * 65536L) != 0 ||
                    mem[p + 5].b16.s1 % 256 != 0 ||
                    (mem[p + 5].b16.s0 + (mem[p + 5].b16.s1 / 256) * 65536L) != 0) {
                    print_cstr(", right-delimiter ");
                    print_delimiter(p + 5);
                }

                print_subsidiary_data(p + 2, '\\' );
                print_subsidiary_data(p + 3, '/' );
                break;

            default:
                print_cstr("Unknown node type!");
                break;
            }
        }

        p = LLIST_link(p);
    }
}


/* show_box: translated to Rust in src/scan.rs */
#if 0
void show_box(int32_t p)
{

    depth_threshold = INTPAR(show_box_depth);
    breadth_max = INTPAR(show_box_breadth) /*:244 */ ;
    if (breadth_max <= 0)
        breadth_max = 5;
    if (pool_ptr + depth_threshold >= pool_size)
        depth_threshold = pool_size - pool_ptr - 1;
    show_node_list(p);
    print_ln();
}
#endif

/* short_display_n: translated to Rust in src/scan.rs */
#if 0
void short_display_n(int32_t p, int32_t m)
{
    breadth_max = m;
    depth_threshold = pool_size - pool_ptr - 1;
    show_node_list(p);
}
#endif

/* delete_token_ref translated to Rust (src/mem.rs). */
/* delete_glue_ref translated to Rust (src/mem.rs). */

/* flush_node_list: translated to Rust in src/scan.rs */
#if 0
void
flush_node_list(int32_t p)
{
    int32_t q;

    while (p != TEX_NULL) {
        q = mem[p].b32.s1;

        if (is_char_node(p)) {
            mem[p].b32.s1 = avail;
            avail = p;
        } else {
            switch (mem[p].b16.s1) {
            case HLIST_NODE:
            case VLIST_NODE:
            case UNSET_NODE:
                flush_node_list(mem[p + 5].b32.s1);
                free_node(p, BOX_NODE_SIZE);
                goto done;
                break;

            case RULE_NODE:
                free_node(p, RULE_NODE_SIZE);
                goto done;
                break;

            case INS_NODE:
                flush_node_list(mem[p + 4].b32.s0);
                delete_glue_ref(INSERTION_NODE_split_top_ptr(p));
                free_node(p, INS_NODE_SIZE);
                goto done;
                break;

            case WHATSIT_NODE:
                switch (mem[p].b16.s0) {
                case OPEN_NODE:
                    free_node(p, OPEN_NODE_SIZE);
                    break;
                case WRITE_NODE:
                case SPECIAL_NODE:
                    delete_token_ref(mem[p + 1].b32.s1);
                    free_node(p, WRITE_NODE_SIZE);
                    goto done;
                    break;
                case CLOSE_NODE:
                case LANGUAGE_NODE:
                    free_node(p, SMALL_NODE_SIZE);
                    break;
                case NATIVE_WORD_NODE:
                case NATIVE_WORD_NODE_AT:
                    if (NATIVE_NODE_glyph_info_ptr(p) != NULL) {
                        NATIVE_NODE_glyph_info_ptr(p) = mfree(NATIVE_NODE_glyph_info_ptr(p));
                        NATIVE_NODE_glyph_count(p) = 0;
                    }
                    free_node(p, NATIVE_NODE_size(p));
                    break;
                case GLYPH_NODE:
                    free_node(p, GLYPH_NODE_SIZE);
                    break;
                case PIC_NODE:
                case PDF_NODE:
                    free_node(p,
                              (PIC_NODE_SIZE +
                               (mem[p + 4].b16.s1 + sizeof(memory_word) - 1) / sizeof(memory_word)));
                    break;
                case PDF_SAVE_POS_NODE:
                    free_node(p, SMALL_NODE_SIZE);
                    break;
                default:
                    confusion("ext3");
                    break;
                }
                goto done;
                break;

            case GLUE_NODE:
                if (mem[mem[p + 1].b32.s0].b32.s1 == TEX_NULL)
                    free_node(mem[p + 1].b32.s0, GLUE_SPEC_SIZE);
                else
                    mem[mem[p + 1].b32.s0].b32.s1--;

                if (mem[p + 1].b32.s1 != TEX_NULL)
                    flush_node_list(mem[p + 1].b32.s1);
                free_node(p, MEDIUM_NODE_SIZE);
                goto done;
                break;

            case KERN_NODE:
            case MATH_NODE:
            case PENALTY_NODE:
                free_node(p, MEDIUM_NODE_SIZE);
                goto done;
                break;

            case MARGIN_KERN_NODE:
                free_node(p, MARGIN_KERN_NODE_SIZE);
                goto done;
                break;

            case LIGATURE_NODE:
                flush_node_list(mem[p + 1].b32.s1);
                break;

            case MARK_NODE:
                delete_token_ref(mem[p + 1].b32.s1);
                break;

            case DISC_NODE:
                flush_node_list(mem[p + 1].b32.s0);
                flush_node_list(mem[p + 1].b32.s1);
                break;

            case ADJUST_NODE:
                flush_node_list(mem[p + 1].b32.s1);
                break;

            case STYLE_NODE:
                free_node(p, STYLE_NODE_SIZE);
                goto done;
                break;

            case CHOICE_NODE:
                flush_node_list(mem[p + 1].b32.s0);
                flush_node_list(mem[p + 1].b32.s1);
                flush_node_list(mem[p + 2].b32.s0);
                flush_node_list(mem[p + 2].b32.s1);
                free_node(p, STYLE_NODE_SIZE);
                goto done;
                break;

            case ORD_NOAD:
            case OP_NOAD:
            case BIN_NOAD:
            case REL_NOAD:
            case OPEN_NOAD:
            case CLOSE_NOAD:
            case PUNCT_NOAD:
            case INNER_NOAD:
            case RADICAL_NOAD:
            case OVER_NOAD:
            case UNDER_NOAD:
            case VCENTER_NOAD:
            case ACCENT_NOAD:
                if (mem[p + 1].b32.s1 >= SUB_BOX)
                    flush_node_list(mem[p + 1].b32.s0);
                if (mem[p + 2].b32.s1 >= SUB_BOX)
                    flush_node_list(mem[p + 2].b32.s0);
                if (mem[p + 3].b32.s1 >= SUB_BOX)
                    flush_node_list(mem[p + 3].b32.s0);
                if (mem[p].b16.s1 == RADICAL_NOAD)
                    free_node(p, RADICAL_NOAD_SIZE);
                else if (mem[p].b16.s1 == ACCENT_NOAD)
                    free_node(p, ACCENT_NOAD_SIZE);
                else
                    free_node(p, NOAD_SIZE);
                goto done;
                break;

            case LEFT_NOAD:
            case RIGHT_NOAD:
                free_node(p, NOAD_SIZE);
                goto done;
                break;

            case FRACTION_NOAD:
                flush_node_list(mem[p + 2].b32.s0);
                flush_node_list(mem[p + 3].b32.s0);
                free_node(p, FRACTION_NOAD_SIZE);
                goto done;
                break;

            default:
                confusion("flushing");
                break;
            }

            free_node(p, SMALL_NODE_SIZE);
        done:
            ;
        }

        p = q;
    }
}
#endif


/* copy_node_list: translated to Rust in src/scan.rs */
#if 0
int32_t
copy_node_list(int32_t p)
{
    int32_t h;
    int32_t q;
    int32_t r;
    unsigned char words;

    h = get_avail();
    q = h;

    while (p != TEX_NULL) {
        words = 1;
        if (is_char_node(p)) {
            r = get_avail();
        } else { /*214:*/
            switch (mem[p].b16.s1) {
            case HLIST_NODE:
            case VLIST_NODE:
            case UNSET_NODE:
                r = get_node(BOX_NODE_SIZE);
                SYNCTEX_tag(r, BOX_NODE_SIZE) = SYNCTEX_tag(p, BOX_NODE_SIZE);
                SYNCTEX_line(r, BOX_NODE_SIZE) = SYNCTEX_line(p, BOX_NODE_SIZE);
                mem[r + 6] = mem[p + 6];
                mem[r + 5] = mem[p + 5];
                mem[r + 5].b32.s1 = copy_node_list(mem[p + 5].b32.s1);
                words = 5;
                break;

            case RULE_NODE:
                r = get_node(RULE_NODE_SIZE);
                words = (RULE_NODE_SIZE - 1);
                break;

            case INS_NODE:
                r = get_node(INS_NODE_SIZE);
                mem[r + 4] = mem[p + 4];
                GLUE_SPEC_ref_count(mem[p + 4].b32.s1)++;
                mem[r + 4].b32.s0 = copy_node_list(mem[p + 4].b32.s0);
                words = (INS_NODE_SIZE - 1);
                break;

            case WHATSIT_NODE:
                switch (mem[p].b16.s0) {
                case OPEN_NODE:
                    r = get_node(OPEN_NODE_SIZE);
                    words = OPEN_NODE_SIZE;
                    break;
                case WRITE_NODE:
                case SPECIAL_NODE:
                    r = get_node(WRITE_NODE_SIZE);
                    mem[mem[p + 1].b32.s1].b32.s0++;
                    words = WRITE_NODE_SIZE;
                    break;
                case CLOSE_NODE:
                case LANGUAGE_NODE:
                    r = get_node(SMALL_NODE_SIZE);
                    words = SMALL_NODE_SIZE;
                    break;
                case NATIVE_WORD_NODE:
                case NATIVE_WORD_NODE_AT:
                    words = NATIVE_NODE_size(p);
                    r = get_node(words);

                    while (words > 0) {
                        words--;
                        mem[r + words] = mem[p + words];
                    }

                    NATIVE_NODE_glyph_info_ptr(r) = NULL;
                    NATIVE_NODE_glyph_count(r) = 0;
                    copy_native_glyph_info(p, r);
                    break;
                case GLYPH_NODE:
                    r = get_node(GLYPH_NODE_SIZE);
                    words = GLYPH_NODE_SIZE;
                    break;
                case PIC_NODE:
                case PDF_NODE:
                    words =
                        (PIC_NODE_SIZE +
                         (mem[p + 4].b16.s1 + sizeof(memory_word) - 1) / sizeof(memory_word));
                    r = get_node(words);
                    break;
                case PDF_SAVE_POS_NODE:
                    r = get_node(SMALL_NODE_SIZE);
                    break;
                default:
                    confusion("ext2");
                    break;
                }
                break;

            case GLUE_NODE:
                r = get_node(MEDIUM_NODE_SIZE);
                GLUE_SPEC_ref_count(mem[p + 1].b32.s0)++;
                mem[r + 2].b32.s0 = mem[p + 2].b32.s0;
                mem[r + 2].b32.s1 = mem[p + 2].b32.s1;
                mem[r + 1].b32.s0 = mem[p + 1].b32.s0;
                mem[r + 1].b32.s1 = copy_node_list(mem[p + 1].b32.s1);
                break;

            case KERN_NODE:
            case MATH_NODE:
            case PENALTY_NODE:
                r = get_node(MEDIUM_NODE_SIZE);
                words = MEDIUM_NODE_SIZE;
                break;

            case MARGIN_KERN_NODE:
                r = get_node(MARGIN_KERN_NODE_SIZE);
                words = MARGIN_KERN_NODE_SIZE;
                break;

            case LIGATURE_NODE:
                r = get_node(SMALL_NODE_SIZE);
                mem[r + 1] = mem[p + 1];
                mem[r + 1].b32.s1 = copy_node_list(mem[p + 1].b32.s1);
                break;

            case DISC_NODE:
                r = get_node(SMALL_NODE_SIZE);
                mem[r + 1].b32.s0 = copy_node_list(mem[p + 1].b32.s0);
                mem[r + 1].b32.s1 = copy_node_list(mem[p + 1].b32.s1);
                break;

            case MARK_NODE:
                r = get_node(SMALL_NODE_SIZE);
                mem[mem[p + 1].b32.s1].b32.s0++;
                words = SMALL_NODE_SIZE;
                break;

            case ADJUST_NODE:
                r = get_node(SMALL_NODE_SIZE);
                mem[r + 1].b32.s1 = copy_node_list(mem[p + 1].b32.s1);
                break;

            default:
                confusion("copying");
                break;
            }
        }

        while (words > 0) {
            words--;
            mem[r + words] = mem[p + words];
        }

        mem[q].b32.s1 = r;
        q = r;
        p = LLIST_link(p);
    }

    mem[q].b32.s1 = TEX_NULL;
    q = mem[h].b32.s1;
    mem[h].b32.s1 = avail;
    avail = h;
    return q;
}
#endif


/* print_mode translated to Rust (src/output.rs). */
/* print_in_mode translated to Rust (src/output.rs). */

/* push_nest translated to Rust (src/mem.rs). */
/* pop_nest translated to Rust (src/mem.rs). */

void show_activities(void)
{
    int32_t p;
    short /*mmode */ m;
    memory_word a;
    int32_t q, r;
    int32_t t;

    nest[nest_ptr] = cur_list;
    print_nl_cstr("");
    print_ln();
    {
        register int32_t for_end;
        p = nest_ptr;
        for_end = 0;
        if (p >= for_end)
            do {
                m = nest[p].mode;
                a = nest[p].aux;
                print_nl_cstr("### ");
                print_mode(m);
                print_cstr(" entered at line ");
                print_int(abs(nest[p].mode_line));
                if (m == HMODE) {

                    if (nest[p].prev_graf != 0x830000) {
                        print_cstr(" (language");
                        print_int(nest[p].prev_graf % 65536L);
                        print_cstr(":hyphenmin");
                        print_int(nest[p].prev_graf / 0x400000);
                        print_char(',');
                        print_int((nest[p].prev_graf / 65536L) % 64);
                        print_char(')');
                    }
                }
                if (nest[p].mode_line < 0)
                    print_cstr(" (\\output routine)");
                if (p == 0) {
                    if (PAGE_HEAD != page_tail) {
                        print_nl_cstr("### current page:");
                        if (output_active)
                            print_cstr(" (held over for next output)");
                        show_box(mem[PAGE_HEAD].b32.s1);
                        if (page_contents > EMPTY) {
                            print_nl_cstr("total height ");
                            print_totals();
                            print_nl_cstr(" goal height ");
                            print_scaled(page_so_far[0]);
                            r = mem[PAGE_INS_HEAD].b32.s1;
                            while (r != PAGE_INS_HEAD) {

                                print_ln();
                                print_esc_cstr("insert");
                                t = mem[r].b16.s0;
                                print_int(t);
                                print_cstr(" adds ");
                                if (COUNT_REG(t) == 1000)
                                    t = mem[r + 3].b32.s1;
                                else
                                    t = x_over_n(mem[r + 3].b32.s1, 1000) * COUNT_REG(t);
                                print_scaled(t);
                                if (mem[r].b16.s1 == SPLIT_UP) {
                                    q = PAGE_HEAD;
                                    t = 0;
                                    do {
                                        q = LLIST_link(q);
                                        if ((NODE_type(q) == INS_NODE) && (mem[q].b16.s0 == mem[r].b16.s0))
                                            t++;
                                    } while (!(q == mem[r + 1].b32.s0));
                                    print_cstr(", #");
                                    print_int(t);
                                    print_cstr(" might split");
                                }
                                r = LLIST_link(r);
                            }
                        }
                    }
                    if (mem[CONTRIB_HEAD].b32.s1 != TEX_NULL)
                        print_nl_cstr("### recent contributions:");
                }
                show_box(mem[nest[p].head].b32.s1);
                switch (abs(m) / ((MAX_COMMAND + 1))) {
                case 0:
                    {
                        print_nl_cstr("prevdepth ");
                        if (a.b32.s1 <= IGNORE_DEPTH)
                            print_cstr("ignored");
                        else
                            print_scaled(a.b32.s1);
                        if (nest[p].prev_graf != 0) {
                            print_cstr(", prevgraf ");
                            print_int(nest[p].prev_graf);
                            if (nest[p].prev_graf != 1)
                                print_cstr(" lines");
                            else
                                print_cstr(" line");
                        }
                    }
                    break;
                case 1:
                    {
                        print_nl_cstr("spacefactor ");
                        print_int(a.b32.s0);
                        if (m > 0) {

                            if (a.b32.s1 > 0) {
                                print_cstr(", current language ");
                                print_int(a.b32.s1);
                            }
                        }
                    }
                    break;
                case 2:
                    if (a.b32.s1 != TEX_NULL) {
                        print_cstr("this will begin denominator of:");
                        show_box(a.b32.s1);
                    }
                    break;
                }
            }
            while (p-- > for_end);
    }
}

/* print_param translated to Rust (src/output.rs). */

/* begin_diagnostic translated to Rust (src/output.rs). */
/* end_diagnostic translated to Rust (src/output.rs). */

/* print_length_param translated to Rust (src/output.rs). */


void
print_cmd_chr(uint16_t cmd, int32_t chr_code)
{
    int32_t n;
    str_number font_name_str;
    UTF16_code quote_char;

    switch (cmd) {
    case LEFT_BRACE:
        print_cstr("begin-group character ");
        if (chr_code < 65536L)
            print(chr_code);
        else
            print_char(chr_code);
        break;

    case RIGHT_BRACE:
        print_cstr("end-group character ");
        if (chr_code < 65536L)
            print(chr_code);
        else
            print_char(chr_code);
        break;

    case MATH_SHIFT:
        print_cstr("math shift character ");
        if (chr_code < 65536L)
            print(chr_code);
        else
            print_char(chr_code);
        break;

    case MAC_PARAM:
        print_cstr("macro parameter character ");
        if (chr_code < 65536L)
            print(chr_code);
        else
            print_char(chr_code);
        break;

    case SUP_MARK:
        print_cstr("superscript character ");
        if (chr_code < 65536L)
            print(chr_code);
        else
            print_char(chr_code);
        break;

    case SUB_MARK:
        print_cstr("subscript character ");
        if (chr_code < 65536L)
            print(chr_code);
        else
            print_char(chr_code);
        break;

    case ENDV:
        print_cstr("end of alignment template");
        break;

    case SPACER:
        print_cstr("blank space ");
        if (chr_code < 65536L)
            print(chr_code);
        else
            print_char(chr_code);
        break;

    case LETTER:
        print_cstr("the letter ");
        if (chr_code < 65536L)
            print(chr_code);
        else
            print_char(chr_code);
        break;

    case OTHER_CHAR:
        print_cstr("the character ");
        if (chr_code < 65536L)
            print(chr_code);
        else
            print_char(chr_code);
        break;

    case ASSIGN_GLUE:
    case ASSIGN_MU_GLUE:
        if (chr_code < SKIP_BASE) {
            print_skip_param(chr_code - GLUE_BASE);
        } else if (chr_code < MU_SKIP_BASE) {
            print_esc_cstr("skip");
            print_int(chr_code - SKIP_BASE);
        } else {
            print_esc_cstr("muskip");
            print_int(chr_code - MU_SKIP_BASE);
        }
        break;

    case ASSIGN_TOKS:
        if (chr_code >= TOKS_BASE) {
            print_esc_cstr("toks");
            print_int(chr_code - TOKS_BASE);
        } else {
            switch (chr_code) {
            case LOCAL_BASE + LOCAL__output_routine:
                print_esc_cstr("output");
                break;
            case LOCAL_BASE + LOCAL__every_par:
                print_esc_cstr("everypar");
                break;
            case LOCAL_BASE + LOCAL__every_math:
                print_esc_cstr("everymath");
                break;
            case LOCAL_BASE + LOCAL__every_display:
                print_esc_cstr("everydisplay");
                break;
            case LOCAL_BASE + LOCAL__every_hbox:
                print_esc_cstr("everyhbox");
                break;
            case LOCAL_BASE + LOCAL__every_vbox:
                print_esc_cstr("everyvbox");
                break;
            case LOCAL_BASE + LOCAL__every_job:
                print_esc_cstr("everyjob");
                break;
            case LOCAL_BASE + LOCAL__every_cr:
                print_esc_cstr("everycr");
                break;
            case LOCAL_BASE + LOCAL__every_eof:
                print_esc_cstr("everyeof");
                break;
            case LOCAL_BASE + LOCAL__xetex_inter_char_toks:
                print_esc_cstr("XeTeXinterchartoks");
                break;
            case LOCAL_BASE + LOCAL__tectonic_coda_tokens:
                print_esc_cstr("TectonicCodaTokens");
                break;
            default:
                print_esc_cstr("errhelp");
                break;
            }
        }
        break;

    case ASSIGN_INT:
        if (chr_code < COUNT_BASE) {
            print_param(chr_code - INT_BASE);
        } else {
            print_esc_cstr("count");
            print_int(chr_code - COUNT_BASE);
        }
        break;

    case ASSIGN_DIMEN:
        if (chr_code < SCALED_BASE) {
            print_length_param(chr_code - DIMEN_BASE);
        } else {
            print_esc_cstr("dimen");
            print_int(chr_code - SCALED_BASE);
        }
        break;

    case ACCENT:
        print_esc_cstr("accent");
        break;

    case ADVANCE:
        print_esc_cstr("advance");
        break;

    case AFTER_ASSIGNMENT:
        print_esc_cstr("afterassignment");
        break;

    case AFTER_GROUP:
        print_esc_cstr("aftergroup");
        break;

    case ASSIGN_FONT_DIMEN:
        print_esc_cstr("fontdimen");
        break;

    case BEGIN_GROUP:
        print_esc_cstr("begingroup");
        break;

    case BREAK_PENALTY:
        print_esc_cstr("penalty");
        break;

    case CHAR_NUM:
        print_esc_cstr("char");
        break;

    case CS_NAME:
        print_esc_cstr("csname");
        break;

    case DEF_FONT:
        print_esc_cstr("font");
        break;

    case DELIM_NUM:
        if (chr_code == 1)
            print_esc_cstr("Udelimiter");
        else
            print_esc_cstr("delimiter");
        break;

    case DIVIDE:
        print_esc_cstr("divide");
        break;

    case END_CS_NAME:
        print_esc_cstr("endcsname");
        break;

    case END_GROUP:
        print_esc_cstr("endgroup");
        break;

    case EX_SPACE:
        print_esc(' ' );
        break;

    case EXPAND_AFTER:
        if (chr_code == 0)
            print_esc_cstr("expandafter");
        else
            print_esc_cstr("unless");
        break;

    case HALIGN:
        print_esc_cstr("halign");
        break;

    case HRULE:
        print_esc_cstr("hrule");
        break;

    case IGNORE_SPACES:
        if (chr_code == 0)
            print_esc_cstr("ignorespaces");
        else
            print_esc_cstr("primitive");
        break;

    case INSERT:
        print_esc_cstr("insert");
        break;

    case ITAL_CORR:
        print_esc('/' );
        break;

    case MARK:
        print_esc_cstr("mark");
        if (chr_code > 0)
            print_char('s');
        break;

    case MATH_ACCENT:
        if (chr_code == 1)
            print_esc_cstr("Umathaccent");
        else
            print_esc_cstr("mathaccent");
        break;

    case MATH_CHAR_NUM:
        if (chr_code == 2)
            print_esc_cstr("Umathchar");
        else if (chr_code == 1)
            print_esc_cstr("Umathcharnum");
        else
            print_esc_cstr("mathchar");
        break;

    case MATH_CHOICE:
        print_esc_cstr("mathchoice");
        break;

    case MULTIPLY:
        print_esc_cstr("multiply");
        break;

    case NO_ALIGN:
        print_esc_cstr("noalign");
        break;

    case NO_BOUNDARY:
        print_esc_cstr("noboundary");
        break;

    case NO_EXPAND:
        if (chr_code == 0)
            print_esc_cstr("noexpand");
        else
            print_esc_cstr("primitive");
        break;

    case NON_SCRIPT:
        print_esc_cstr("nonscript");
        break;

    case OMIT:
        print_esc_cstr("omit");
        break;

    case RADICAL:
        if (chr_code == 1)
            print_esc_cstr("Uradical");
        else
            print_esc_cstr("radical");
        break;

    case READ_TO_CS:
        if (chr_code == 0)
            print_esc_cstr("read");
        else
            print_esc_cstr("readline");
        break;

    case RELAX:
        print_esc_cstr("relax");
        break;

    case SET_BOX:
        print_esc_cstr("setbox");
        break;

    case SET_PREV_GRAF:
        print_esc_cstr("prevgraf");
        break;

    case SET_SHAPE:
        switch (chr_code) {
        case LOCAL_BASE + LOCAL__par_shape:
            print_esc_cstr("parshape");
            break;
        case ETEX_PEN_BASE + ETEX_PENALTIES_PAR__inter_line_penalties:
            print_esc_cstr("interlinepenalties");
            break;
        case ETEX_PEN_BASE + ETEX_PENALTIES_PAR__club_penalties:
            print_esc_cstr("clubpenalties");
            break;
        case ETEX_PEN_BASE + ETEX_PENALTIES_PAR__widow_penalties:
            print_esc_cstr("widowpenalties");
            break;
        case ETEX_PEN_BASE + ETEX_PENALTIES_PAR__display_widow_penalties:
            print_esc_cstr("displaywidowpenalties");
            break;
        }
        break;

    case THE:
        if (chr_code == 0)
            print_esc_cstr("the");
        else if (chr_code == 1)
            print_esc_cstr("unexpanded");
        else
            print_esc_cstr("detokenize");
        break;

    case TOKS_REGISTER:
        print_esc_cstr("toks");
        if (chr_code != 0)
            print_sa_num(chr_code);
        break;

    case VADJUST:
        print_esc_cstr("vadjust");
        break;

    case VALIGN:
        if (chr_code == 0) {
            print_esc_cstr("valign");
        } else {
            switch (chr_code) {
            case BEGIN_L_CODE:
                print_esc_cstr("beginL");
                break;
            case END_L_CODE:
                print_esc_cstr("endL");
                break;
            case BEGIN_R_CODE:
                print_esc_cstr("beginR");
                break;
            default:
                print_esc_cstr("endR");
                break;
            }
        }
        break;

    case VCENTER:
        print_esc_cstr("vcenter");
        break;

    case VRULE:
        print_esc_cstr("vrule");
        break;

    case PAR_END:
        print_esc_cstr("par");
        break;

    case INPUT:
        if (chr_code == 0)
            print_esc_cstr("input");
        else if (chr_code == 2)
            print_esc_cstr("scantokens");
        else
            print_esc_cstr("endinput");
        break;

    case TOP_BOT_MARK:
        switch (chr_code % MARKS_CODE) {
        case FIRST_MARK_CODE:
            print_esc_cstr("firstmark");
            break;
        case BOT_MARK_CODE:
            print_esc_cstr("botmark");
            break;
        case SPLIT_FIRST_MARK_CODE:
            print_esc_cstr("splitfirstmark");
            break;
        case SPLIT_BOT_MARK_CODE:
            print_esc_cstr("splitbotmark");
            break;
        default:
            print_esc_cstr("topmark");
            break;
        }
        if (chr_code >= MARKS_CODE)
            print_char('s');
        break;

    case REGISTER:
        if (chr_code < 0 || chr_code > 19 /*lo_mem_stat_max*/) {
            cmd = (mem[chr_code].b16.s1 / 64);
        } else {
            cmd = chr_code;
            chr_code = TEX_NULL;
        }

        if (cmd == INT_VAL)
            print_esc_cstr("count");
        else if (cmd == DIMEN_VAL)
            print_esc_cstr("dimen");
        else if (cmd == GLUE_VAL)
            print_esc_cstr("skip");
        else
            print_esc_cstr("muskip");

        if (chr_code != TEX_NULL)
            print_sa_num(chr_code);
        break;

    case SET_AUX:
        if (chr_code == VMODE)
            print_esc_cstr("prevdepth");
        else
            print_esc_cstr("spacefactor");
        break;

    case SET_PAGE_INT:
        if (chr_code == 0)
            print_esc_cstr("deadcycles");
        else if (chr_code == 2)
            print_esc_cstr("interactionmode");
        else
            print_esc_cstr("insertpenalties");
        break;

    case SET_BOX_DIMEN:
        if (chr_code == WIDTH_OFFSET)
            print_esc_cstr("wd");
        else if (chr_code == HEIGHT_OFFSET)
            print_esc_cstr("ht");
        else
            print_esc_cstr("dp");
        break;

    case LAST_ITEM:
        switch (chr_code) {
        case INT_VAL:
            print_esc_cstr("lastpenalty");
            break;
        case DIMEN_VAL:
            print_esc_cstr("lastkern");
            break;
        case GLUE_VAL:
            print_esc_cstr("lastskip");
            break;
        case INPUT_LINE_NO_CODE:
            print_esc_cstr("inputlineno");
            break;
        case LAST_NODE_TYPE_CODE:
            print_esc_cstr("lastnodetype");
            break;
        case ETEX_VERSION_CODE:
            print_esc_cstr("eTeXversion");
            break;
        case XETEX_VERSION_CODE:
            print_esc_cstr("XeTeXversion");
            break;
        case XETEX_COUNT_GLYPHS_CODE:
            print_esc_cstr("XeTeXcountglyphs");
            break;
        case XETEX_COUNT_VARIATIONS_CODE:
            print_esc_cstr("XeTeXcountvariations");
            break;
        case XETEX_VARIATION_CODE:
            print_esc_cstr("XeTeXvariation");
            break;
        case XETEX_FIND_VARIATION_BY_NAME_CODE:
            print_esc_cstr("XeTeXfindvariationbyname");
            break;
        case XETEX_VARIATION_MIN_CODE:
            print_esc_cstr("XeTeXvariationmin");
            break;
        case XETEX_VARIATION_MAX_CODE:
            print_esc_cstr("XeTeXvariationmax");
            break;
        case XETEX_VARIATION_DEFAULT_CODE:
            print_esc_cstr("XeTeXvariationdefault");
            break;
        case XETEX_COUNT_FEATURES_CODE:
            print_esc_cstr("XeTeXcountfeatures");
            break;
        case XETEX_FEATURE_CODE_CODE:
            print_esc_cstr("XeTeXfeaturecode");
            break;
        case XETEX_FIND_FEATURE_BY_NAME_CODE:
            print_esc_cstr("XeTeXfindfeaturebyname");
            break;
        case XETEX_IS_EXCLUSIVE_FEATURE_CODE:
            print_esc_cstr("XeTeXisexclusivefeature");
            break;
        case XETEX_COUNT_SELECTORS_CODE:
            print_esc_cstr("XeTeXcountselectors");
            break;
        case XETEX_SELECTOR_CODE_CODE:
            print_esc_cstr("XeTeXselectorcode");
            break;
        case XETEX_FIND_SELECTOR_BY_NAME_CODE:
            print_esc_cstr("XeTeXfindselectorbyname");
            break;
        case XETEX_IS_DEFAULT_SELECTOR_CODE:
            print_esc_cstr("XeTeXisdefaultselector");
            break;
        case XETEX_OT_COUNT_SCRIPTS_CODE:
            print_esc_cstr("XeTeXOTcountscripts");
            break;
        case XETEX_OT_COUNT_LANGUAGES_CODE:
            print_esc_cstr("XeTeXOTcountlanguages");
            break;
        case XETEX_OT_COUNT_FEATURES_CODE:
            print_esc_cstr("XeTeXOTcountfeatures");
            break;
        case XETEX_OT_SCRIPT_CODE:
            print_esc_cstr("XeTeXOTscripttag");
            break;
        case XETEX_OT_LANGUAGE_CODE:
            print_esc_cstr("XeTeXOTlanguagetag");
            break;
        case XETEX_OT_FEATURE_CODE:
            print_esc_cstr("XeTeXOTfeaturetag");
            break;
        case XETEX_MAP_CHAR_TO_GLYPH_CODE:
            print_esc_cstr("XeTeXcharglyph");
            break;
        case XETEX_GLYPH_INDEX_CODE:
            print_esc_cstr("XeTeXglyphindex");
            break;
        case XETEX_GLYPH_BOUNDS_CODE:
            print_esc_cstr("XeTeXglyphbounds");
            break;
        case XETEX_FONT_TYPE_CODE:
            print_esc_cstr("XeTeXfonttype");
            break;
        case XETEX_FIRST_CHAR_CODE:
            print_esc_cstr("XeTeXfirstfontchar");
            break;
        case XETEX_LAST_CHAR_CODE:
            print_esc_cstr("XeTeXlastfontchar");
            break;
        case XETEX_PDF_PAGE_COUNT_CODE:
            print_esc_cstr("XeTeXpdfpagecount");
            break;
        case CURRENT_GROUP_LEVEL_CODE:
            print_esc_cstr("currentgrouplevel");
            break;
        case CURRENT_GROUP_TYPE_CODE:
            print_esc_cstr("currentgrouptype");
            break;
        case CURRENT_IF_LEVEL_CODE:
            print_esc_cstr("currentiflevel");
            break;
        case CURRENT_IF_TYPE_CODE:
            print_esc_cstr("currentiftype");
            break;
        case CURRENT_IF_BRANCH_CODE:
            print_esc_cstr("currentifbranch");
            break;
        case FONT_CHAR_WD_CODE:
            print_esc_cstr("fontcharwd");
            break;
        case FONT_CHAR_HT_CODE:
            print_esc_cstr("fontcharht");
            break;
        case FONT_CHAR_DP_CODE:
            print_esc_cstr("fontchardp");
            break;
        case FONT_CHAR_IC_CODE:
            print_esc_cstr("fontcharic");
            break;
        case PAR_SHAPE_LENGTH_CODE:
            print_esc_cstr("parshapelength");
            break;
        case PAR_SHAPE_INDENT_CODE:
            print_esc_cstr("parshapeindent");
            break;
        case PAR_SHAPE_DIMEN_CODE:
            print_esc_cstr("parshapedimen");
            break;
        case (ETEX_EXPR - INT_VAL + INT_VAL):
            print_esc_cstr("numexpr");
            break;
        case (ETEX_EXPR - INT_VAL + DIMEN_VAL):
            print_esc_cstr("dimexpr");
            break;
        case (ETEX_EXPR - INT_VAL + GLUE_VAL):
            print_esc_cstr("glueexpr");
            break;
        case (ETEX_EXPR - INT_VAL + MU_VAL):
            print_esc_cstr("muexpr");
            break;
        case GLUE_STRETCH_ORDER_CODE:
            print_esc_cstr("gluestretchorder");
            break;
        case GLUE_SHRINK_ORDER_CODE:
            print_esc_cstr("glueshrinkorder");
            break;
        case GLUE_STRETCH_CODE:
            print_esc_cstr("gluestretch");
            break;
        case GLUE_SHRINK_CODE:
            print_esc_cstr("glueshrink");
            break;
        case MU_TO_GLUE_CODE:
            print_esc_cstr("mutoglue");
            break;
        case GLUE_TO_MU_CODE:
            print_esc_cstr("gluetomu");
            break;
        case PDF_LAST_X_POS_CODE:
            print_esc_cstr("pdflastxpos");
            break;
        case PDF_LAST_Y_POS_CODE:
            print_esc_cstr("pdflastypos");
            break;
        case ELAPSED_TIME_CODE:
            print_esc_cstr("elapsedtime");
            break;
        case PDF_SHELL_ESCAPE_CODE:
            print_esc_cstr("shellescape");
            break;
        case RANDOM_SEED_CODE:
            print_esc_cstr("randomseed");
            break;
        default:
            print_esc_cstr("badness");
            break;
        }
        break;

    case CONVERT:
        switch (chr_code) {
        case NUMBER_CODE:
            print_esc_cstr("number");
            break;
        case ROMAN_NUMERAL_CODE:
            print_esc_cstr("romannumeral");
            break;
        case STRING_CODE:
            print_esc_cstr("string");
            break;
        case MEANING_CODE:
            print_esc_cstr("meaning");
            break;
        case FONT_NAME_CODE:
            print_esc_cstr("fontname");
            break;
        case ETEX_REVISION_CODE:
            print_esc_cstr("eTeXrevision");
            break;
        case EXPANDED_CODE:
            print_esc_cstr("expanded");
            break;
        case LEFT_MARGIN_KERN_CODE:
            print_esc_cstr("leftmarginkern");
            break;
        case RIGHT_MARGIN_KERN_CODE:
            print_esc_cstr("rightmarginkern");
            break;
        case PDF_CREATION_DATE_CODE:
            print_esc_cstr("creationdate");
            break;
        case PDF_FILE_MOD_DATE_CODE:
            print_esc_cstr("filemoddate");
            break;
        case PDF_FILE_SIZE_CODE:
            print_esc_cstr("filesize");
            break;
        case PDF_MDFIVE_SUM_CODE:
            print_esc_cstr("mdfivesum");
            break;
        case PDF_FILE_DUMP_CODE:
            print_esc_cstr("filedump");
            break;
        case PDF_STRCMP_CODE:
            print_esc_cstr("strcmp");
            break;
        case UNIFORM_DEVIATE_CODE:
            print_esc_cstr("uniformdeviate");
            break;
        case NORMAL_DEVIATE_CODE:
            print_esc_cstr("normaldeviate");
            break;
        case XETEX_REVISION_CODE:
            print_esc_cstr("XeTeXrevision");
            break;
        case XETEX_VARIATION_NAME_CODE:
            print_esc_cstr("XeTeXvariationname");
            break;
        case XETEX_FEATURE_NAME_CODE:
            print_esc_cstr("XeTeXfeaturename");
            break;
        case XETEX_SELECTOR_NAME_CODE:
            print_esc_cstr("XeTeXselectorname");
            break;
        case XETEX_GLYPH_NAME_CODE:
            print_esc_cstr("XeTeXglyphname");
            break;
        case XETEX_UCHAR_CODE:
            print_esc_cstr("Uchar");
            break;
        case XETEX_UCHARCAT_CODE:
            print_esc_cstr("Ucharcat");
            break;
        default:
            print_esc_cstr("jobname");
            break;
        }
        break;

    case IF_TEST:
        if (chr_code >= UNLESS_CODE)
            print_esc_cstr("unless");

        switch (chr_code % UNLESS_CODE) {
        case IF_CAT_CODE:
            print_esc_cstr("ifcat");
            break;
        case IF_INT_CODE:
            print_esc_cstr("ifnum");
            break;
        case IF_DIM_CODE:
            print_esc_cstr("ifdim");
            break;
        case IF_ODD_CODE:
            print_esc_cstr("ifodd");
            break;
        case IF_VMODE_CODE:
            print_esc_cstr("ifvmode");
            break;
        case IF_HMODE_CODE:
            print_esc_cstr("ifhmode");
            break;
        case IF_MMODE_CODE:
            print_esc_cstr("ifmmode");
            break;
        case IF_INNER_CODE:
            print_esc_cstr("ifinner");
            break;
        case IF_VOID_CODE:
            print_esc_cstr("ifvoid");
            break;
        case IF_HBOX_CODE:
            print_esc_cstr("ifhbox");
            break;
        case IF_VBOX_CODE:
            print_esc_cstr("ifvbox");
            break;
        case IFX_CODE:
            print_esc_cstr("ifx");
            break;
        case IF_EOF_CODE:
            print_esc_cstr("ifeof");
            break;
        case IF_TRUE_CODE:
            print_esc_cstr("iftrue");
            break;
        case IF_FALSE_CODE:
            print_esc_cstr("iffalse");
            break;
        case IF_CASE_CODE:
            print_esc_cstr("ifcase");
            break;
        case IF_PRIMITIVE_CODE:
            print_esc_cstr("ifprimitive");
            break;
        case IF_DEF_CODE:
            print_esc_cstr("ifdefined");
            break;
        case IF_CS_CODE:
            print_esc_cstr("ifcsname");
            break;
        case IF_FONT_CHAR_CODE:
            print_esc_cstr("iffontchar");
            break;
        case IF_IN_CSNAME_CODE:
            print_esc_cstr("ifincsname");
            break;
        default:
            print_esc_cstr("if");
            break;
        }
        break;

    case FI_OR_ELSE:
        if (chr_code == FI_CODE)
            print_esc_cstr("fi");
        else if (chr_code == OR_CODE)
            print_esc_cstr("or");
        else
            print_esc_cstr("else");
        break;

    case TAB_MARK:
        if (chr_code == SPAN_CODE) {
            print_esc_cstr("span");
        } else {
            print_cstr("alignment tab character ");
            if (chr_code < 65536L)
                print(chr_code);
            else
                print_char(chr_code);
        }
        break;

    case CAR_RET:
        if (chr_code == CR_CODE)
            print_esc_cstr("cr");
        else
            print_esc_cstr("crcr");
        break;

    case SET_PAGE_DIMEN:
        switch (chr_code) {
        case 0: /* genuine literal in WEB */
            print_esc_cstr("pagegoal");
            break;
        case 1: /* genuine literal in WEB */
            print_esc_cstr("pagetotal");
            break;
        case 2: /* genuine literal in WEB */
            print_esc_cstr("pagestretch");
            break;
        case 3: /* genuine literal in WEB */
            print_esc_cstr("pagefilstretch");
            break;
        case 4: /* genuine literal in WEB */
            print_esc_cstr("pagefillstretch");
            break;
        case 5: /* genuine literal in WEB */
            print_esc_cstr("pagefilllstretch");
            break;
        case 6: /* genuine literal in WEB */
            print_esc_cstr("pageshrink");
            break;
        default:
            print_esc_cstr("pagedepth");
            break;
        }
        break;

    case STOP:
        if (chr_code == 1)
            print_esc_cstr("dump");
        else
            print_esc_cstr("end");
        break;

    case HSKIP:
        switch (chr_code) {
        case SKIP_CODE:
            print_esc_cstr("hskip");
            break;
        case FIL_CODE:
            print_esc_cstr("hfil");
            break;
        case FILL_CODE:
            print_esc_cstr("hfill");
            break;
        case SS_CODE:
            print_esc_cstr("hss");
            break;
        default:
            print_esc_cstr("hfilneg");
            break;
        }
        break;

    case VSKIP:
        switch (chr_code) {
        case SKIP_CODE:
            print_esc_cstr("vskip");
            break;
        case FIL_CODE:
            print_esc_cstr("vfil");
            break;
        case FILL_CODE:
            print_esc_cstr("vfill");
            break;
        case SS_CODE:
            print_esc_cstr("vss");
            break;
        default:
            print_esc_cstr("vfilneg");
            break;
        }
        break;

    case MSKIP:
        print_esc_cstr("mskip");
        break;

    case KERN:
        print_esc_cstr("kern");
        break;

    case MKERN:
        print_esc_cstr("mkern");
        break;

    case HMOVE:
        if (chr_code == 1)
            print_esc_cstr("moveleft");
        else
            print_esc_cstr("moveright");
        break;

    case VMOVE:
        if (chr_code == 1)
            print_esc_cstr("raise");
        else
            print_esc_cstr("lower");
        break;

    case MAKE_BOX:
        switch (chr_code) {
        case BOX_CODE:
            print_esc_cstr("box");
            break;
        case COPY_CODE:
            print_esc_cstr("copy");
            break;
        case LAST_BOX_CODE:
            print_esc_cstr("lastbox");
            break;
        case VSPLIT_CODE:
            print_esc_cstr("vsplit");
            break;
        case VTOP_CODE:
            print_esc_cstr("vtop");
            break;
        case (VTOP_CODE + VMODE):
            print_esc_cstr("vbox");
            break;
        default:
            print_esc_cstr("hbox");
            break;
        }
        break;

    case LEADER_SHIP:
        if (chr_code == A_LEADERS)
            print_esc_cstr("leaders");
        else if (chr_code == C_LEADERS)
            print_esc_cstr("cleaders");
        else if (chr_code == X_LEADERS)
            print_esc_cstr("xleaders");
        else
            print_esc_cstr("shipout");
        break;

    case START_PAR:
        if (chr_code == 0)
            print_esc_cstr("noindent");
        else
            print_esc_cstr("indent");
        break;

    case REMOVE_ITEM:
        if (chr_code == GLUE_NODE)
            print_esc_cstr("unskip");
        else if (chr_code == KERN_NODE)
            print_esc_cstr("unkern");
        else
            print_esc_cstr("unpenalty");
        break;

    case UN_HBOX:
        if (chr_code == COPY_CODE)
            print_esc_cstr("unhcopy");
        else
            print_esc_cstr("unhbox");
        break;

    case UN_VBOX:
        if (chr_code == COPY_CODE)
            print_esc_cstr("unvcopy");
        else if (chr_code == LAST_BOX_CODE)
            print_esc_cstr("pagediscards");
        else if (chr_code == VSPLIT_CODE)
            print_esc_cstr("splitdiscards");
        else
            print_esc_cstr("unvbox");
        break;

    case DISCRETIONARY:
        if (chr_code == 1)
            print_esc('-' );
        else
            print_esc_cstr("discretionary");
        break;

    case EQ_NO:
        if (chr_code == 1)
            print_esc_cstr("leqno");
        else
            print_esc_cstr("eqno");
        break;

    case MATH_COMP:
        switch (chr_code) {
        case ORD_NOAD:
            print_esc_cstr("mathord");
            break;
        case OP_NOAD:
            print_esc_cstr("mathop");
            break;
        case BIN_NOAD:
            print_esc_cstr("mathbin");
            break;
        case REL_NOAD:
            print_esc_cstr("mathrel");
            break;
        case OPEN_NOAD:
            print_esc_cstr("mathopen");
            break;
        case CLOSE_NOAD:
            print_esc_cstr("mathclose");
            break;
        case PUNCT_NOAD:
            print_esc_cstr("mathpunct");
            break;
        case INNER_NOAD:
            print_esc_cstr("mathinner");
            break;
        case UNDER_NOAD:
            print_esc_cstr("underline");
            break;
        default:
            print_esc_cstr("overline");
            break;
        }
        break;

    case LIMIT_SWITCH:
        if (chr_code == LIMITS)
            print_esc_cstr("limits");
        else if (chr_code == NO_LIMITS)
            print_esc_cstr("nolimits");
        else
            print_esc_cstr("displaylimits");
        break;

    case MATH_STYLE:
        print_style(chr_code);
        break;

    case ABOVE:
        switch (chr_code) {
        case OVER_CODE:
            print_esc_cstr("over");
            break;
        case ATOP_CODE:
            print_esc_cstr("atop");
            break;
        case DELIMITED_CODE + ABOVE_CODE:
            print_esc_cstr("abovewithdelims");
            break;
        case DELIMITED_CODE + OVER_CODE:
            print_esc_cstr("overwithdelims");
            break;
        case DELIMITED_CODE + ATOP_CODE:
            print_esc_cstr("atopwithdelims");
            break;
        default:
            print_esc_cstr("above");
            break;
        }
        break;

    case LEFT_RIGHT:
        if (chr_code == LEFT_NOAD)
            print_esc_cstr("left");
        else if (chr_code == MIDDLE_NOAD)
            print_esc_cstr("middle");
        else
            print_esc_cstr("right");
        break;

    case PREFIX:
        if (chr_code == 1)
            print_esc_cstr("long");
        else if (chr_code == 2)
            print_esc_cstr("outer");
        else if (chr_code == 8)
            print_esc_cstr("protected");
        else
            print_esc_cstr("global");
        break;

    case DEF:
        if (chr_code == 0)
            print_esc_cstr("def");
        else if (chr_code == 1)
            print_esc_cstr("gdef");
        else if (chr_code == 2)
            print_esc_cstr("edef");
        else
            print_esc_cstr("xdef");
        break;

    case LET:
        if (chr_code != NORMAL)
            print_esc_cstr("futurelet");
        else
            print_esc_cstr("let");
        break;

    case SHORTHAND_DEF:
        switch (chr_code) {
        case CHAR_DEF_CODE:
            print_esc_cstr("chardef");
            break;
        case MATH_CHAR_DEF_CODE:
            print_esc_cstr("mathchardef");
            break;
        case XETEX_MATH_CHAR_DEF_CODE:
            print_esc_cstr("Umathchardef");
            break;
        case XETEX_MATH_CHAR_NUM_DEF_CODE:
            print_esc_cstr("Umathcharnumdef");
            break;
        case COUNT_DEF_CODE:
            print_esc_cstr("countdef");
            break;
        case DIMEN_DEF_CODE:
            print_esc_cstr("dimendef");
            break;
        case SKIP_DEF_CODE:
            print_esc_cstr("skipdef");
            break;
        case MU_SKIP_DEF_CODE:
            print_esc_cstr("muskipdef");
            break;
        case CHAR_SUB_DEF_CODE:
            print_esc_cstr("charsubdef");
            break;
        default:
            print_esc_cstr("toksdef");
            break;
        }
        break;

    case CHAR_GIVEN:
        print_esc_cstr("char");
        print_hex(chr_code);
        break;

    case MATH_GIVEN:
        print_esc_cstr("mathchar");
        print_hex(chr_code);
        break;

    case XETEX_MATH_GIVEN:
        print_esc_cstr("Umathchar");
        print_hex(math_class(chr_code));
        print_hex(math_fam(chr_code));
        print_hex(math_char(chr_code));
        break;

    case DEF_CODE:
        if (chr_code == CAT_CODE_BASE)
            print_esc_cstr("catcode");
        else if (chr_code == MATH_CODE_BASE)
            print_esc_cstr("mathcode");
        else if (chr_code == LC_CODE_BASE)
            print_esc_cstr("lccode");
        else if (chr_code == UC_CODE_BASE)
            print_esc_cstr("uccode");
        else if (chr_code == SF_CODE_BASE)
            print_esc_cstr("sfcode");
        else
            print_esc_cstr("delcode");
        break;

    case XETEX_DEF_CODE:
        if (chr_code == SF_CODE_BASE)
            print_esc_cstr("XeTeXcharclass");
        else if (chr_code == MATH_CODE_BASE)
            print_esc_cstr("Umathcodenum");
        else if (chr_code == (MATH_CODE_BASE + 1))
            print_esc_cstr("Umathcode");
        else if (chr_code == DEL_CODE_BASE)
            print_esc_cstr("Udelcodenum");
        else
            print_esc_cstr("Udelcode");
        break;

    case DEF_FAMILY:
        print_size(chr_code - MATH_FONT_BASE);
        break;

    case HYPH_DATA:
        if (chr_code == 1)
            print_esc_cstr("patterns");
        else
            print_esc_cstr("hyphenation");
        break;

    case ASSIGN_FONT_INT:
        switch (chr_code) {
        case 0:
            print_esc_cstr("hyphenchar");
            break;
        case 1:
            print_esc_cstr("skewchar");
            break;
        case LP_CODE_BASE:
            print_esc_cstr("lpcode");
            break;
        case RP_CODE_BASE:
            print_esc_cstr("rpcode");
            break;
        }
        break;

    case SET_FONT:
        print_cstr("select font ");

        font_name_str = font_name[chr_code];
        if (font_area[chr_code] == AAT_FONT_FLAG || font_area[chr_code] == OTGR_FONT_FLAG) {
            int32_t for_end = length(font_name_str) - 1;
            quote_char = '"' ;

            for (n = 0; n <= for_end; n++) {
                if (str_pool[str_start[(font_name_str) - 65536L] + n] == '"' )
                    quote_char = '\'' ;
            }

            print_char(quote_char);
            print(font_name_str);
            print_char(quote_char);
        } else {
            print(font_name_str);
        }

        if (font_size[chr_code] != font_dsize[chr_code]) {
            print_cstr(" at ");
            print_scaled(font_size[chr_code]);
            print_cstr("pt");
        }
        break;

    case SET_INTERACTION:
        switch (chr_code) {
        case BATCH_MODE:
            print_esc_cstr("batchmode");
            break;
        case NONSTOP_MODE:
            print_esc_cstr("nonstopmode");
            break;
        case SCROLL_MODE:
            print_esc_cstr("scrollmode");
            break;
        default:
            print_esc_cstr("errorstopmode");
            break;
        }
        break;

    case IN_STREAM:
        if (chr_code == 0)
            print_esc_cstr("closein");
        else
            print_esc_cstr("openin");
        break;

    case MESSAGE:
        if (chr_code == 0)
            print_esc_cstr("message");
        else
            print_esc_cstr("errmessage");
        break;

    case CASE_SHIFT:
        if (chr_code == LC_CODE_BASE)
            print_esc_cstr("lowercase");
        else
            print_esc_cstr("uppercase");
        break;

    case XRAY:
        switch (chr_code) {
        case SHOW_BOX_CODE:
            print_esc_cstr("showbox");
            break;
        case SHOW_THE_CODE:
            print_esc_cstr("showthe");
            break;
        case SHOW_LISTS:
            print_esc_cstr("showlists");
            break;
        case SHOW_GROUPS:
            print_esc_cstr("showgroups");
            break;
        case SHOW_TOKENS:
            print_esc_cstr("showtokens");
            break;
        case SHOW_IFS:
            print_esc_cstr("showifs");
            break;
        default:
            print_esc_cstr("show");
            break;
        }
        break;

    case UNDEFINED_CS:
        print_cstr("undefined");
        break;

    case CALL:
    case LONG_CALL:
    case OUTER_CALL:
    case LONG_OUTER_CALL:
        n = cmd - CALL;
        if (mem[mem[chr_code].b32.s1].b32.s0 == PROTECTED_TOKEN)
            n = n + 4;
        if (odd(n / 4))
            print_esc_cstr("protected");
        if (odd(n))
            print_esc_cstr("long");
        if (odd(n / 2))
            print_esc_cstr("outer");
        if (n > 0)
            print_char(' ');
        print_cstr("macro");
        break;

    case END_TEMPLATE:
        print_esc_cstr("outer endtemplate");
        break;

    case EXTENSION:
        switch (chr_code) {
        case OPEN_NODE:
            print_esc_cstr("openout");
            break;
        case WRITE_NODE:
            print_esc_cstr("write");
            break;
        case CLOSE_NODE:
            print_esc_cstr("closeout");
            break;
        case SPECIAL_NODE:
            print_esc_cstr("special");
            break;
        case IMMEDIATE_CODE:
            print_esc_cstr("immediate");
            break;
        case SET_LANGUAGE_CODE:
            print_esc_cstr("setlanguage");
            break;
        case PDF_SAVE_POS_NODE:
            print_esc_cstr("pdfsavepos");
            break;
        case RESET_TIMER_CODE:
            print_esc_cstr("resettimer");
            break;
        case SET_RANDOM_SEED_CODE:
            print_esc_cstr("setrandomseed");
            break;
        case PIC_FILE_CODE:
            print_esc_cstr("XeTeXpicfile");
            break;
        case PDF_FILE_CODE:
            print_esc_cstr("XeTeXpdffile");
            break;
        case GLYPH_CODE:
            print_esc_cstr("XeTeXglyph");
            break;
        case XETEX_LINEBREAK_LOCALE_EXTENSION_CODE:
            print_esc_cstr("XeTeXlinebreaklocale");
            break;
        case XETEX_INPUT_ENCODING_EXTENSION_CODE:
            print_esc_cstr("XeTeXinputencoding");
            break;
        case XETEX_DEFAULT_ENCODING_EXTENSION_CODE:
            print_esc_cstr("XeTeXdefaultencoding");
            break;
        default:
            print_cstr("[unknown extension!]");
            break;
        }
        break;

    default:
        print_cstr("[unknown command code!]");
        break;
    }
}


/* not_aat_font_error, not_aat_gr_font_error, not_ot_font_error,
   not_native_font_error translated to Rust (src/errors.rs). */

/*:1434*/


/* id_lookup translated to Rust (src/hash.rs). */

/* prim_lookup translated to Rust (src/hash.rs). */

/*:276*//*280: *//*296: */

/* print_group translated to Rust (src/output.rs). */

/*:1448*//*1449: */

/* pseudo_input: translated to Rust in src/input.rs */

/* pseudo_close: translated to Rust in src/input.rs */

/* group_warning translated to Rust (src/output.rs). */
/* if_warning translated to Rust (src/output.rs). */
/* file_warning translated to Rust (src/output.rs). */


/* delete_sa_ref translated to Rust (src/save.rs). */
/* sa_save translated to Rust (src/save.rs). */
/* sa_destroy translated to Rust (src/save.rs). */
/* sa_def translated to Rust (src/save.rs). */
/* sa_w_def translated to Rust (src/save.rs). */
/* gsa_def translated to Rust (src/save.rs). */
/* gsa_w_def translated to Rust (src/save.rs). */
/* sa_restore translated to Rust (src/save.rs). */

/* new_save_level translated to Rust (src/save.rs). */

/* eq_destroy translated to Rust (src/save.rs). */

/* eq_save translated to Rust (src/save.rs). */

/* eq_define translated to Rust (src/save.rs). */

/* eq_word_define translated to Rust (src/save.rs). */

/* geq_define translated to Rust (src/save.rs). */

/* geq_word_define translated to Rust (src/save.rs). */

/* save_for_after translated to Rust (src/save.rs). */

/* unsave translated to Rust (src/save.rs). */

/* prepare_mag translated to Rust (src/save.rs). */

/* token_show translated to Rust (src/output.rs). */

/* print_meaning translated to Rust (src/output.rs). */


/* show_cur_cmd_chr translated to Rust (src/output.rs). */

void show_context(void)
{
    unsigned char /*max_selector */ old_setting;
    int32_t nn;
    bool bottom_line;
    int32_t i;
    int32_t j;
    int32_t l;
    int32_t m;
    int32_t n;
    int32_t p;
    int32_t q;

    base_ptr = input_ptr;
    input_stack[base_ptr] = cur_input;
    nn = -1;
    bottom_line = false;

    while (true) {

        cur_input = input_stack[base_ptr];
        if ((cur_input.state != TOKEN_LIST)) {

            if ((cur_input.name > 19) || (base_ptr == 0))
                bottom_line = true;
        }
        if ((base_ptr == input_ptr) || bottom_line || (nn < INTPAR(error_context_lines))) {   /*324: */
            if ((base_ptr == input_ptr) || (cur_input.state != TOKEN_LIST)
                || (cur_input.index != BACKED_UP) || (cur_input.loc != TEX_NULL)) {
                tally = 0;
                old_setting = selector;
                if (cur_input.state != TOKEN_LIST) {
                    if (cur_input.name <= 17) {

                        if (cur_input.name == 0) {

                            if (base_ptr == 0)
                                print_nl_cstr("<*>");
                            else
                                print_nl_cstr("<insert> ");
                        } else {

                            print_nl_cstr("<read ");
                            if (cur_input.name == 17)
                                print_char('*');
                            else
                                print_int(cur_input.name - 1);
                            print_char('>');
                        }
                    } else {

                        print_nl_cstr("l.");
                        if (cur_input.index == in_open)
                            print_int(line);
                        else
                            print_int(line_stack[cur_input.index + 1]);
                    }
                    print_char(' ');
                    {
                        l = tally;
                        tally = 0;
                        selector = SELECTOR_PSEUDO;
                        trick_count = 1000000L;
                    }
                    if (buffer[cur_input.limit] == INTPAR(end_line_char))
                        j = cur_input.limit;
                    else
                        j = cur_input.limit + 1;
                    if (j > 0) {
                        register int32_t for_end;
                        i = cur_input.start;
                        for_end = j - 1;
                        if (i <= for_end)
                            do {
                                if (i == cur_input.loc) {
                                    first_count = tally;
                                    trick_count = tally + 1 + error_line - half_error_line;
                                    if (trick_count < error_line)
                                        trick_count = error_line;
                                }
                                print_char(buffer[i]);
                            }
                            while (i++ < for_end);
                    }
                } else {

                    switch (cur_input.index) {
                    case PARAMETER:
                        print_nl_cstr("<argument> ");
                        break;
                    case U_TEMPLATE:
                    case V_TEMPLATE:
                        print_nl_cstr("<template> ");
                        break;
                    case BACKED_UP:
                    case BACKED_UP_CHAR:
                        if (cur_input.loc == TEX_NULL)
                            print_nl_cstr("<recently read> ");
                        else
                            print_nl_cstr("<to be read again> ");
                        break;
                    case INSERTED:
                        print_nl_cstr("<inserted text> ");
                        break;
                    case MACRO:
                        print_ln();
                        print_cs(cur_input.name);
                        break;
                    case OUTPUT_TEXT:
                        print_nl_cstr("<output> ");
                        break;
                    case EVERY_PAR_TEXT:
                        print_nl_cstr("<everypar> ");
                        break;
                    case EVERY_MATH_TEXT:
                        print_nl_cstr("<everymath> ");
                        break;
                    case EVERY_DISPLAY_TEXT:
                        print_nl_cstr("<everydisplay> ");
                        break;
                    case EVERY_HBOX_TEXT:
                        print_nl_cstr("<everyhbox> ");
                        break;
                    case EVERY_VBOX_TEXT:
                        print_nl_cstr("<everyvbox> ");
                        break;
                    case EVERY_JOB_TEXT:
                        print_nl_cstr("<everyjob> ");
                        break;
                    case EVERY_CR_TEXT:
                        print_nl_cstr("<everycr> ");
                        break;
                    case MARK_TEXT:
                        print_nl_cstr("<mark> ");
                        break;
                    case EVERY_EOF_TEXT:
                        print_nl_cstr("<everyeof> ");
                        break;
                    case INTER_CHAR_TEXT:
                        print_nl_cstr("<XeTeXinterchartoks> ");
                        break;
                    case WRITE_TEXT:
                        print_nl_cstr("<write> ");
                        break;
                    case TECTONIC_CODA_TEXT:
                        print_nl_cstr("<TectonicCodaTokens> ");
                        break;
                    default:
                        print_nl('?' );
                        break;
                    }
                    {
                        l = tally;
                        tally = 0;
                        selector = SELECTOR_PSEUDO;
                        trick_count = 1000000L;
                    }
                    if (cur_input.index < MACRO)
                        show_token_list(cur_input.start, cur_input.loc, 100000L);
                    else
                        show_token_list(mem[cur_input.start].b32.s1, cur_input.loc, 100000L);
                }
                selector = old_setting;
                if (trick_count == 1000000L) {
                    first_count = tally;
                    trick_count = tally + 1 + error_line - half_error_line;
                    if (trick_count < error_line)
                        trick_count = error_line;
                }
                if (tally < trick_count)
                    m = tally - first_count;
                else
                    m = trick_count - first_count;
                if (l + first_count <= half_error_line) {
                    p = 0;
                    n = l + first_count;
                } else {

                    print_cstr("...");
                    p = l + first_count - half_error_line + 3;
                    n = half_error_line;
                }
                {
                    register int32_t for_end;
                    q = p;
                    for_end = first_count - 1;
                    if (q <= for_end)
                        do
                            print_char(trick_buf[q % error_line]);
                        while (q++ < for_end);
                }
                print_ln();
                {
                    register int32_t for_end;
                    q = 1;
                    for_end = n;
                    if (q <= for_end)
                        do
                            print_raw_char(' ', true);
                        while (q++ < for_end);
                }
                if (m + n <= error_line)
                    p = first_count + m;
                else
                    p = first_count + (error_line - n - 3);
                {
                    register int32_t for_end;
                    q = first_count;
                    for_end = p - 1;
                    if (q <= for_end)
                        do
                            print_char(trick_buf[q % error_line]);
                        while (q++ < for_end);
                }
                if (m + n > error_line)
                    print_cstr("...");
                nn++;
            }
        } else if (nn == INTPAR(error_context_lines)) {
            print_nl_cstr("...");
            nn++;
        }
        if (bottom_line)
            goto done;
        base_ptr--;
    }
done:
    cur_input = input_stack[input_ptr];
}


/* begin_token_list: translated to Rust in src/input.rs */
#if 0
void
begin_token_list(int32_t p, uint16_t t)
{

    if (input_ptr > max_in_stack) {
        max_in_stack = input_ptr;
        if (input_ptr == stack_size)
            overflow("input stack size", stack_size);
    }

    input_stack[input_ptr] = cur_input;
    input_ptr++;

    cur_input.state = TOKEN_LIST;
    cur_input.start = p;
    cur_input.index = t;

    if (t >= MACRO) {
        mem[p].b32.s0++;

        if (t == MACRO) {
            cur_input.limit = param_ptr;
        } else {
            cur_input.loc = mem[p].b32.s1;

            if (INTPAR(tracing_macros) > 1) {
                begin_diagnostic();
                diagnostic_begin_capture_warning_here();
                print_nl_cstr("");
                switch (t) {
                case MARK_TEXT:
                    print_esc_cstr("mark");
                    break;
                case WRITE_TEXT:
                    print_esc_cstr("write");
                    break;
                default:
                    print_cmd_chr(ASSIGN_TOKS, t + LOCAL_BASE + LOCAL__output_routine - OUTPUT_TEXT);
                    break;
                }
                print_cstr("->");
                token_show(p);
                capture_to_diagnostic(NULL);
                end_diagnostic(false);
            }
        }
    } else {
        cur_input.loc = p;
    }
}
#endif


/* end_token_list: translated to Rust in src/input.rs */
/* back_input: translated to Rust in src/input.rs */
/* back_error: translated to Rust in src/input.rs */
/* ins_error: translated to Rust in src/input.rs */

/* begin_file_reading: translated to Rust in src/input.rs */
/* end_file_reading: translated to Rust in src/input.rs */


/* check_outer_validity: translated to Rust in src/scan.rs */
#if 0
void
check_outer_validity(void)
{
    int32_t p;
    int32_t q;

    if (scanner_status != NORMAL) {
        deletions_allowed = false;

        if (cur_cs != 0) {
            if (cur_input.state == TOKEN_LIST || cur_input.name < 1 || cur_input.name > 17) {
                p = get_avail();
                mem[p].b32.s0 = CS_TOKEN_FLAG + cur_cs;
                begin_token_list(p, BACKED_UP);
            }

            cur_cmd = SPACER;
            cur_chr = ' ' ;
        }

        if (scanner_status > SKIPPING) { /*350:*/
            runaway();

            if (cur_cs == 0) {
                error_here_with_diagnostic("File ended");
            } else {
                cur_cs = 0;
                error_here_with_diagnostic("Forbidden control sequence found");
            }

            p = get_avail();

            switch (scanner_status) {
            case DEFINING:
                print_cstr(" while scanning definition");
                mem[p].b32.s0 = (RIGHT_BRACE_TOKEN + '}' );
                break;
            case MATCHING:
                print_cstr(" while scanning use");
                mem[p].b32.s0 = par_token;
                long_state = OUTER_CALL;
                break;

            case ALIGNING:
                print_cstr(" while scanning preamble");
                mem[p].b32.s0 = (RIGHT_BRACE_TOKEN + '}' );
                q = p;
                p = get_avail();
                mem[p].b32.s1 = q;
                mem[p].b32.s0 = CS_TOKEN_FLAG + FROZEN_CR;
                align_state = -1000000L;
                break;

            case ABSORBING:
                print_cstr(" while scanning text");
                mem[p].b32.s0 = (RIGHT_BRACE_TOKEN + '}' );
                break;
            }

            begin_token_list(p, INSERTED);
            print_cstr(" of ");
            sprint_cs(warning_index);
            capture_to_diagnostic(NULL);
            help_ptr = 4;
            help_line[3] = "I suspect you have forgotten a `}', causing me";
            help_line[2] = "to read past where you wanted me to stop.";
            help_line[1] = "I'll try to recover; but if the error is serious,";
            help_line[0] = "you'd better type `E' or `X' now and fix your file.";
            error();
        } else {
            error_here_with_diagnostic("Incomplete ");
            print_cmd_chr(IF_TEST, cur_if);
            print_cstr("; all text was ignored after line ");
            print_int(skip_line);
            capture_to_diagnostic(NULL);
            help_ptr = 3;
            help_line[2] = "A forbidden control sequence occurred in skipped text.";
            help_line[1] = "This kind of error happens when you say `\\if...' and forget";
            help_line[0] = "the matching `\\fi'. I've inserted a `\\fi'; this might work.";

            if (cur_cs != 0)
                cur_cs = 0;
            else
                help_line[2] = "The file ended while I was skipping conditional text.";

            cur_tok = CS_TOKEN_FLAG + FROZEN_FI;
            ins_error();
        }

        deletions_allowed = true;
    }
}
#endif


/* These macros are kinda scary, but convenient */
#define ANY_STATE_PLUS(c) case (MID_LINE + c): case (SKIP_BLANKS + c): case (NEW_LINE + c)
#define ADD_DELIMS_TO(s) \
    case (s + MATH_SHIFT): case (s + TAB_MARK): case (s + MAC_PARAM): \
    case (s + SUB_MARK): case (s + LETTER): case (s + OTHER_CHAR)


void
get_next(void)
{
    int32_t k;
    int32_t t;
    unsigned char /*max_char_code */ cat;
    UnicodeScalar c;
    UTF16_code lower;
    small_number d;
    small_number sup_count;

restart:
    cur_cs = 0;

    if (cur_input.state != TOKEN_LIST) { /*355:*/
    texswitch:
        if (cur_input.loc <= cur_input.limit) {
            cur_chr = buffer[cur_input.loc];
            cur_input.loc++;

            if (cur_chr >= 0xD800 && cur_chr < 0xDC00 && cur_input.loc <= cur_input.limit &&
                buffer[cur_input.loc] >= 0xDC00 && buffer[cur_input.loc] < 0xE000) {
                lower = buffer[cur_input.loc] - 0xDC00;
                cur_input.loc++;
                cur_chr = 65536L + (cur_chr - 0xD800) * 1024 + lower;
            }

        reswitch:
            cur_cmd = CAT_CODE(cur_chr);

            switch (cur_input.state + cur_cmd) { /*357:*/
            ANY_STATE_PLUS(IGNORE):
            case SKIP_BLANKS + SPACER:
            case NEW_LINE + SPACER:
                goto texswitch;
                break;

            ANY_STATE_PLUS(ESCAPE):
                if (cur_input.loc > cur_input.limit) {
                    cur_cs = NULL_CS;
                } else {
                start_cs:
                    k = cur_input.loc;
                    cur_chr = buffer[k];
                    cat = CAT_CODE(cur_chr);
                    k++;

                    if (cat == LETTER)
                        cur_input.state = SKIP_BLANKS;
                    else if (cat == SPACER)
                        cur_input.state = SKIP_BLANKS;
                    else
                        cur_input.state = MID_LINE;

                    if (cat == LETTER && k <= cur_input.limit) { /*368:*/
                        do {
                            cur_chr = buffer[k];
                            cat = CAT_CODE(cur_chr);
                            k++;
                        } while (cat == LETTER && k <= cur_input.limit);

                        if (cat == SUP_MARK && buffer[k] == cur_chr && k < cur_input.limit) {
                            /* Special characters: either ^^X, or up to six
                             * ^'s followed by one hex character for each
                             * ^. */

                            int32_t sup_count_save;

                            /* How many ^'s are there? */

                            sup_count = 2;

                            while (sup_count < 6 && k + 2 * sup_count - 2 <= cur_input.limit &&
                                   buffer[k + sup_count - 1] == cur_chr)
                                sup_count++;

                            /* If they are followed by a sufficient number of
                             * hex characters, treat it as an extended ^^^
                             * sequence. If not, treat it as original-style
                             * ^^X. */

                            sup_count_save = sup_count;

                            for (d = 1; d <= sup_count_save; d++) {
                                if (!IS_LC_HEX(buffer[k + sup_count - 2 + d])) {
                                    /* Non-hex: do it old style */
                                    c = buffer[k + 1];

                                    if (c < 128) {
                                        if (c < 64)
                                            buffer[k - 1] = c + 64;
                                        else
                                            buffer[k - 1] = c - 64;
                                        d = 2;
                                        cur_input.limit = cur_input.limit - d;
                                        while (k <= cur_input.limit) {
                                            buffer[k] = buffer[k + d];
                                            k++;
                                        }
                                        goto start_cs;
                                    } else {
                                        sup_count = 0;
                                    }
                                }
                            }

                            if (sup_count > 0) {
                                cur_chr = 0;

                                for (d = 1; d <= sup_count; d++) {
                                    c = buffer[k + sup_count - 2 + d];
                                    if (c <= '9' )
                                        cur_chr = 16 * cur_chr + c - '0';
                                    else
                                        cur_chr = 16 * cur_chr + c - 'a' + 10;
                                }

                                if (cur_chr > BIGGEST_USV) {
                                    cur_chr = buffer[k];
                                } else {
                                    buffer[k - 1] = cur_chr;
                                    d = 2 * sup_count - 1;
                                    cur_input.limit = cur_input.limit - d;

                                    while (k <= cur_input.limit) {
                                        buffer[k] = buffer[k + d];
                                        k++;
                                    }
                                    goto start_cs;
                                }
                            }
                        }

                        if (cat != LETTER)
                            k--;

                        if (k > cur_input.loc + 1) {
                            cur_cs = id_lookup(cur_input.loc, k - cur_input.loc);
                            cur_input.loc = k;
                            goto found;
                        }
                    } else { /*367:*/
                        if (cat == SUP_MARK && buffer[k] == cur_chr && k < cur_input.limit) {
                            int32_t sup_count_save;

                            sup_count = 2;

                            while (sup_count < 6 && k + 2 * sup_count - 2 <= cur_input.limit &&
                                   buffer[k + sup_count - 1] == cur_chr)
                                sup_count++;

                            sup_count_save = sup_count;

                            for (d = 1; d <= sup_count_save; d++) {
                                if (!IS_LC_HEX(buffer[k + sup_count - 2 + d])) {
                                    c = buffer[k + 1];
                                    if (c < 128) {
                                        if (c < 64)
                                            buffer[k - 1] = c + 64;
                                        else
                                            buffer[k - 1] = c - 64;
                                        d = 2;
                                        cur_input.limit = cur_input.limit - d;
                                        while (k <= cur_input.limit) {
                                            buffer[k] = buffer[k + d];
                                            k++;
                                        }
                                        goto start_cs;
                                    } else {
                                        sup_count = 0;
                                    }
                                }
                            }

                            if (sup_count > 0) {
                                cur_chr = 0;

                                for (d = 1; d <= sup_count; d++) {
                                    c = buffer[k + sup_count - 2 + d];
                                    if (c <= '9' )
                                        cur_chr = 16 * cur_chr + c - '0';
                                    else
                                        cur_chr = 16 * cur_chr + c - 'a' + 10;
                                }

                                if (cur_chr > BIGGEST_USV) {
                                    cur_chr = buffer[k];
                                } else {
                                    buffer[k - 1] = cur_chr;
                                    d = 2 * sup_count - 1;
                                    cur_input.limit = cur_input.limit - d;
                                    while (k <= cur_input.limit) {
                                        buffer[k] = buffer[k + d];
                                        k++;
                                    }
                                    goto start_cs;
                                }
                            }
                        }
                    }

                    if (buffer[cur_input.loc] > 65535L) {
                        cur_cs = id_lookup(cur_input.loc, 1);
                        cur_input.loc++;
                        goto found;
                    }

                    cur_cs = SINGLE_BASE + buffer[cur_input.loc];
                    cur_input.loc++;
                }

            found:
                cur_cmd = eqtb[cur_cs].b16.s1;
                cur_chr = eqtb[cur_cs].b32.s1;
                if (cur_cmd >= OUTER_CALL)
                    check_outer_validity();
                break;

            ANY_STATE_PLUS(ACTIVE_CHAR):
                cur_cs = cur_chr + 1;
                cur_cmd = eqtb[cur_cs].b16.s1;
                cur_chr = eqtb[cur_cs].b32.s1;
                cur_input.state = MID_LINE;
                if (cur_cmd >= OUTER_CALL)
                    check_outer_validity();
                break;

            ANY_STATE_PLUS(SUP_MARK):
                if (cur_chr == buffer[cur_input.loc]) {
                    if (cur_input.loc < cur_input.limit) {
                        sup_count = 2;

                        while (sup_count < 6 && cur_input.loc + 2 * sup_count - 2 <= cur_input.limit &&
                               cur_chr == buffer[cur_input.loc + sup_count - 1])
                            sup_count++;

                        for (d = 1; d <= sup_count; d++) {
                            if (!IS_LC_HEX(buffer[cur_input.loc + sup_count - 2 + d])) {
                                c = buffer[cur_input.loc + 1];
                                if (c < 128) {
                                    cur_input.loc = cur_input.loc + 2;
                                    if (c < 64)
                                        cur_chr = c + 64;
                                    else
                                        cur_chr = c - 64;
                                    goto reswitch;
                                }
                                goto not_exp;
                            }
                        }

                        cur_chr = 0;

                        for (d = 1; d <= sup_count; d++) {
                            c = buffer[cur_input.loc + sup_count - 2 + d];
                            if (c <= '9' )
                                cur_chr = 16 * cur_chr + c - '0';
                            else
                                cur_chr = 16 * cur_chr + c - 'a' + 10;
                        }

                        if (cur_chr > BIGGEST_USV) {
                            cur_chr = buffer[cur_input.loc];
                            goto not_exp;
                        }

                        cur_input.loc = cur_input.loc + 2 * sup_count - 1;
                        goto reswitch;
                    }
                }

            not_exp:
                cur_input.state = MID_LINE;
                break;

            ANY_STATE_PLUS(INVALID_CHAR):
                error_here_with_diagnostic("Text line contains an invalid character");
                capture_to_diagnostic(NULL);

                help_ptr = 2;
                help_line[1] = "A funny symbol that I can't read has just been input.";
                help_line[0] = "Continue, and I'll forget that it ever happened.";
                deletions_allowed = false;
                error();
                deletions_allowed = true;
                goto restart;
                break;

            case MID_LINE + SPACER:
                cur_input.state = SKIP_BLANKS;
                cur_chr = ' ' ;
                break;

            case MID_LINE + CAR_RET:
                cur_input.loc = cur_input.limit + 1;
                cur_cmd = SPACER;
                cur_chr = ' ' ;
                break;

            ANY_STATE_PLUS(COMMENT):
            case SKIP_BLANKS + CAR_RET:
                cur_input.loc = cur_input.limit + 1;
                goto texswitch;
                break;

            case NEW_LINE + CAR_RET:
                cur_input.loc = cur_input.limit + 1;
                cur_cs = par_loc;
                cur_cmd = eqtb[cur_cs].b16.s1;
                cur_chr = eqtb[cur_cs].b32.s1;
                if (cur_cmd >= OUTER_CALL)
                    check_outer_validity();
                break;

            case MID_LINE + LEFT_BRACE:
                align_state++;
                break;

            case SKIP_BLANKS + LEFT_BRACE:
            case NEW_LINE + LEFT_BRACE:
                cur_input.state = MID_LINE;
                align_state++;
                break;

            case MID_LINE + RIGHT_BRACE:
                align_state--;
                break;

            case SKIP_BLANKS + RIGHT_BRACE:
            case NEW_LINE + RIGHT_BRACE:
                cur_input.state = MID_LINE;
                align_state--;
                break;

            ADD_DELIMS_TO(SKIP_BLANKS):
            ADD_DELIMS_TO(NEW_LINE):
                cur_input.state = MID_LINE;
                break;

            default:
                break;
            }
        } else {
            cur_input.state = NEW_LINE;

            if (cur_input.name > 17) { /*374:*/
                line++;
                first = cur_input.start;

                if (!force_eof) {
                    if (cur_input.name <= 19) {
                        if (pseudo_input()) {
                            cur_input.limit = last;
                        } else if (LOCAL(every_eof) != TEX_NULL && !eof_seen[cur_input.index]) {
                            cur_input.limit = first - 1;
                            eof_seen[cur_input.index] = true;
                            begin_token_list(LOCAL(every_eof), EVERY_EOF_TEXT);
                            goto restart;
                        } else {
                            force_eof = true;
                        }
                    } else {
                        if (input_line(input_file[cur_input.index])) {
                            cur_input.limit = last;
                        } else if (LOCAL(every_eof) != TEX_NULL && !eof_seen[cur_input.index]) {
                            cur_input.limit = first - 1;
                            eof_seen[cur_input.index] = true;
                            begin_token_list(LOCAL(every_eof), EVERY_EOF_TEXT);
                            goto restart;
                        } else {
                            force_eof = true;
                        }
                    }
                }

                if (force_eof) {
                    if (INTPAR(tracing_nesting) > 0) {
                        if (grp_stack[in_open] != cur_boundary || if_stack[in_open] != cond_ptr)
                            file_warning();
                    }

                    if (cur_input.name >= 19) {
                        print_char(')');
                        open_parens--;
                        ttstub_output_flush(rust_stdout);
                    }

                    force_eof = false;
                    end_file_reading();
                    check_outer_validity();
                    goto restart;
                }

                if (INTPAR(end_line_char) < 0 || INTPAR(end_line_char) > 255)
                    cur_input.limit--;
                else
                    buffer[cur_input.limit] = INTPAR(end_line_char);

                first = cur_input.limit + 1;
                cur_input.loc = cur_input.start;
            } else {
                if (cur_input.name != 0) {
                    cur_cmd = 0;
                    cur_chr = 0;
                    return;
                }

                if (input_ptr > 0) {
                    end_file_reading();
                    goto restart;
                }

                /* Tectonic extension: we add a \TectonicCodaTokens toklist
                 * that gets inserted at the very very end of processing if no
                 * \end or \dump has been seen. We just use a global state
                 * variable to make sure it only gets inserted once. */

                if (!used_tectonic_coda_tokens && LOCAL(tectonic_coda_tokens) != TEX_NULL) {
                    used_tectonic_coda_tokens = true;
                    begin_token_list(LOCAL(tectonic_coda_tokens), TECTONIC_CODA_TEXT);
                    goto restart;
                }

                if (selector < SELECTOR_LOG_ONLY)
                    open_log_file();

                fatal_error("*** (job aborted, no legal \\end found)");
            }
            goto texswitch;
        }
    } else if (cur_input.loc != TEX_NULL) { /* if we're inputting from a non-null token list: */
        t = mem[cur_input.loc].b32.s0;
        cur_input.loc = LLIST_link(cur_input.loc);

        if (t >= CS_TOKEN_FLAG) {
            cur_cs = t - CS_TOKEN_FLAG;
            cur_cmd = eqtb[cur_cs].b16.s1;
            cur_chr = eqtb[cur_cs].b32.s1;

            if (cur_cmd >= OUTER_CALL) {
                if (cur_cmd == DONT_EXPAND) { /*370:*/
                    cur_cs = mem[cur_input.loc].b32.s0 - CS_TOKEN_FLAG;
                    cur_input.loc = TEX_NULL;
                    cur_cmd = eqtb[cur_cs].b16.s1;
                    cur_chr = eqtb[cur_cs].b32.s1;
                    if (cur_cmd > MAX_COMMAND) {
                        cur_cmd = RELAX;
                        cur_chr = NO_EXPAND_FLAG;
                    }
                } else {
                    check_outer_validity();
                }
            }
        } else {
            cur_cmd = t / MAX_CHAR_VAL;
            cur_chr = t % MAX_CHAR_VAL;

            switch (cur_cmd) {
            case LEFT_BRACE:
                align_state++;
                break;
            case RIGHT_BRACE:
                align_state--;
                break;
            case OUT_PARAM:
                begin_token_list(param_stack[cur_input.limit + cur_chr - 1], PARAMETER);
                goto restart;
                break;
            default:
                break;
            }
        }
    } else { /* token list but no tokens left */
        end_token_list();
        goto restart;
    }

    if (cur_cmd <= CAR_RET && cur_cmd >= TAB_MARK && align_state == 0) { /*818:*/
        if (scanner_status == ALIGNING || cur_align == TEX_NULL)
            fatal_error("(interwoven alignment preambles are not allowed)");

        cur_cmd = mem[cur_align + 5].b32.s0;
        mem[cur_align + 5].b32.s0 = cur_chr;
        if (cur_cmd == OMIT)
            begin_token_list(OMIT_TEMPLATE, V_TEMPLATE);
        else
            begin_token_list(mem[cur_align + 2].b32.s1, V_TEMPLATE);
        align_state = 1000000L;
        goto restart;
    }
}


/* get_token: translated to Rust in src/scan.rs */


void
macro_call(void)
{
    int32_t r;
    int32_t p = TEX_NULL;
    int32_t q;
    int32_t s;
    int32_t t;
    int32_t u, v;
    int32_t rbrace_ptr = TEX_NULL;
    small_number n;
    int32_t unbalance;
    int32_t m = 0;
    int32_t ref_count;
    small_number save_scanner_status;
    int32_t save_warning_index;
    UTF16_code match_chr;

    save_scanner_status = scanner_status;
    save_warning_index = warning_index;
    warning_index = cur_cs;
    ref_count = cur_chr;
    r = mem[ref_count].b32.s1;
    n = 0;

    if (INTPAR(tracing_macros) > 0) { /*419:*/
        begin_diagnostic();
        diagnostic_begin_capture_warning_here();

        if (INTPAR(tracing_stack_levels) > 0) {
            if (input_ptr < INTPAR(tracing_stack_levels)) {
                v = input_ptr;
                print_ln();
                print_char('~');

                while (v > 0) {
                    print_char('.');
                    v--;
                }

                print_cs(warning_index);
                token_show(ref_count);
            } else {
                print_char('~');
                print_char('~');
                print_cs(warning_index);
            }
        } else {
            print_ln();
            print_cs(warning_index);
            token_show(ref_count);
        }

        capture_to_diagnostic(NULL);
        end_diagnostic(false);
    }

    if (mem[r].b32.s0 == PROTECTED_TOKEN)
        r = LLIST_link(r);

    if (mem[r].b32.s0 != END_MATCH_TOKEN) { /*409:*/
        scanner_status = MATCHING;
        unbalance = 0;
        long_state = eqtb[cur_cs].b16.s1;

        if (long_state >= OUTER_CALL)
            long_state = long_state - 2;

        do {
            mem[TEMP_HEAD].b32.s1 = TEX_NULL;
            if (mem[r].b32.s0 >= END_MATCH_TOKEN || mem[r].b32.s0 < MATCH_TOKEN) {
                s = TEX_NULL;
            } else {
                match_chr = mem[r].b32.s0 - MATCH_TOKEN;
                s = mem[r].b32.s1;
                r = s;
                p = TEMP_HEAD;
                m = 0;
            }

        continue_:
            get_token();

            if (cur_tok == mem[r].b32.s0) { /*412:*/
                r = LLIST_link(r);
                if (mem[r].b32.s0 >= MATCH_TOKEN && mem[r].b32.s0 <= END_MATCH_TOKEN) {
                    if (cur_tok < LEFT_BRACE_LIMIT)
                        align_state--;
                    goto found;
                } else {
                    goto continue_;
                }
            }

            if (s != r) {
                if (s == TEX_NULL) { /*416:*/
                    error_here_with_diagnostic("Use of ");
                    sprint_cs(warning_index);
                    print_cstr(" doesn't match its definition");
                    capture_to_diagnostic(NULL);
                    help_ptr = 4;
                    help_line[3] = "If you say, e.g., `\\def\\a1{...}', then you must always";
                    help_line[2] = "put `1' after `\\a', since control sequence names are";
                    help_line[1] = "made up of letters only. The macro here has not been";
                    help_line[0] = "followed by the required stuff, so I'm ignoring it.";
                    error();
                    goto exit;
                } else {
                    t = s;

                    do {
                        q = get_avail();
                        mem[p].b32.s1 = q;
                        mem[q].b32.s0 = mem[t].b32.s0;
                        p = q;

                        m++;
                        u = mem[t].b32.s1;
                        v = s;

                        while (true) {
                            if (u == r) {
                                if (cur_tok != mem[v].b32.s0) {
                                    goto done;
                                } else {
                                    r = mem[v].b32.s1;
                                    goto continue_;
                                }
                            }

                            if (mem[u].b32.s0 != mem[v].b32.s0)
                                goto done;

                            u = LLIST_link(u);
                            v = LLIST_link(v);
                        }

                    done:
                        t = LLIST_link(t);
                    } while (t != r);

                    r = s;
                }
            }

            if (cur_tok == par_token) {
                if (long_state != LONG_CALL) { /*414:*/
                    if (long_state == CALL) {
                        runaway();
                        error_here_with_diagnostic("Paragraph ended before ");
                        sprint_cs(warning_index);
                        print_cstr(" was complete");
                        capture_to_diagnostic(NULL);
                        help_ptr = 3;
                        help_line[2] = "I suspect you've forgotten a `}', causing me to apply this";
                        help_line[1] = "control sequence to too much text. How can we recover?";
                        help_line[0] = "My plan is to forget the whole thing and hope for the best.";
                        back_error();
                    }

                    pstack[n] = mem[TEMP_HEAD].b32.s1;
                    align_state = align_state - unbalance;

                    for (m = 0; m <= n; m++)
                        flush_list(pstack[m]);

                    goto exit;
                }
            }

            if (cur_tok < RIGHT_BRACE_LIMIT) {
                if (cur_tok < LEFT_BRACE_LIMIT) { /*417:*/
                    unbalance = 1;

                    while (true) {
                        q = avail;
                        if (q == TEX_NULL) {
                            q = get_avail();
                        } else {
                            avail = mem[q].b32.s1;
                            mem[q].b32.s1 = TEX_NULL;
                        }

                        mem[p].b32.s1 = q;
                        mem[q].b32.s0 = cur_tok;
                        p = q;

                        get_token();

                        if (cur_tok == par_token) {
                            if (long_state != LONG_CALL) { /*414:*/
                                if (long_state == CALL) {
                                    runaway();
                                    error_here_with_diagnostic("Paragraph ended before ");
                                    sprint_cs(warning_index);
                                    print_cstr(" was complete");
                                    capture_to_diagnostic(NULL);
                                    help_ptr = 3;
                                    help_line[2] = "I suspect you've forgotten a `}', causing me to apply this";
                                    help_line[1] = "control sequence to too much text. How can we recover?";
                                    help_line[0] = "My plan is to forget the whole thing and hope for the best.";
                                    back_error();
                                }

                                pstack[n] = mem[TEMP_HEAD].b32.s1;
                                align_state = align_state - unbalance;

                                for (m = 0; m <= n; m++)
                                    flush_list(pstack[m]);

                                goto exit;
                            }
                        }

                        if (cur_tok < RIGHT_BRACE_LIMIT) {
                            if (cur_tok < LEFT_BRACE_LIMIT) {
                                unbalance++;
                            } else {
                                unbalance--;
                                if (unbalance == 0)
                                    goto done1;
                            }
                        }

                    }

                done1:
                    rbrace_ptr = p;

                    q = get_avail();
                    mem[p].b32.s1 = q;
                    mem[q].b32.s0 = cur_tok;
                    p = q;
                } else { /*413:*/
                    back_input();

                    error_here_with_diagnostic("Argument of ");
                    sprint_cs(warning_index);
                    print_cstr(" has an extra }");
                    capture_to_diagnostic(NULL);

                    help_ptr = 6;
                    help_line[5] = "I've run across a `}' that doesn't seem to match anything.";
                    help_line[4] = "For example, `\\def\\a#1{...}' and `\\a}' would produce";
                    help_line[3] = "this error. If you simply proceed now, the `\\par' that";
                    help_line[2] = "I've just inserted will cause me to report a runaway";
                    help_line[1] = "argument that might be the root of the problem. But if";
                    help_line[0] = "your `}' was spurious, just type `2' and it will go away.";
                    align_state++;
                    long_state = CALL;
                    cur_tok = par_token;
                    ins_error();
                    goto continue_;
                }
            } else { /*411:*/
                if (cur_tok == SPACE_TOKEN) {
                    if (mem[r].b32.s0 <= END_MATCH_TOKEN) {
                        if (mem[r].b32.s0 >= MATCH_TOKEN)
                            goto continue_;
                    }
                }

                q = get_avail();
                mem[p].b32.s1 = q;
                mem[q].b32.s0 = cur_tok;
                p = q;
            }

            m++;

            if (mem[r].b32.s0 > END_MATCH_TOKEN)
                goto continue_;
            if (mem[r].b32.s0 < MATCH_TOKEN)
                goto continue_;

        found:
            if (s != TEX_NULL) { /*418:*/
                if (m == 1 && mem[p].b32.s0 < RIGHT_BRACE_LIMIT) {
                    mem[rbrace_ptr].b32.s1 = TEX_NULL;
                    mem[p].b32.s1 = avail;
                    avail = p;
                    p = mem[TEMP_HEAD].b32.s1;
                    pstack[n] = mem[p].b32.s1;
                    mem[p].b32.s1 = avail;
                    avail = p;
                } else {
                    pstack[n] = mem[TEMP_HEAD].b32.s1;
                }

                n++;

                if (INTPAR(tracing_macros) > 0) {
                    if (INTPAR(tracing_stack_levels) == 0 || input_ptr < INTPAR(tracing_stack_levels)) {
                        begin_diagnostic();
                        diagnostic_begin_capture_warning_here();
                        print_nl(match_chr);
                        print_int(n);
                        print_cstr("<-");
                        show_token_list(pstack[n - 1], TEX_NULL, 1000);
                        capture_to_diagnostic(NULL);
                        end_diagnostic(false);
                    }
                }
            }
        } while (mem[r].b32.s0 != END_MATCH_TOKEN);
    }

    while (cur_input.state == TOKEN_LIST && cur_input.loc == TEX_NULL && cur_input.index != V_TEMPLATE)
        end_token_list();

    begin_token_list(ref_count, MACRO);
    cur_input.name = warning_index;
    cur_input.loc = mem[r].b32.s1;

    if (n > 0) {
        if (param_ptr + n > max_param_stack) {
            max_param_stack = param_ptr + n;
            if (max_param_stack > param_size)
                overflow("parameter stack size", param_size);
        }

        for (m = 0; m <= n - 1; m++)
            param_stack[param_ptr + m] = pstack[m];

        param_ptr += n;
    }

exit:
    scanner_status = save_scanner_status;
    warning_index = save_warning_index;
}


/* insert_relax: translated to Rust in src/input.rs */


/* new_index: translated to Rust in src/save.rs */
/* find_sa_element: translated to Rust in src/save.rs */


void
expand(void)
{
    int32_t t;
    bool b;
    int32_t p, q, r;
    int32_t j;
    int32_t cv_backup;
    small_number cvl_backup, radix_backup, co_backup;
    int32_t backup_backup;
    small_number save_scanner_status;

    expand_depth_count++;
    if (expand_depth_count >= expand_depth)
        overflow("expansion depth", expand_depth);

    cv_backup = cur_val;
    cvl_backup = cur_val_level;
    radix_backup = radix;
    co_backup = cur_order;
    backup_backup = mem[BACKUP_HEAD].b32.s1;

reswitch:
    if (cur_cmd < CALL) { /*384:*/
        if (INTPAR(tracing_commands) > 1)
            show_cur_cmd_chr();

        switch (cur_cmd) {
        case TOP_BOT_MARK:
            t = cur_chr % 5;

            if (cur_chr >= 5)
                scan_register_num();
            else
                cur_val = 0;

            if (cur_val == 0) {
                cur_ptr = cur_mark[t];
            } else { /*1612:*/
                find_sa_element(MARK_VAL, cur_val, false);
                if (cur_ptr != TEX_NULL) {
                    if (odd(t))
                        cur_ptr = mem[cur_ptr + (t / 2) + 1].b32.s1;
                    else
                        cur_ptr = mem[cur_ptr + (t / 2) + 1].b32.s0;
                }
            }

            if (cur_ptr != TEX_NULL)
                begin_token_list(cur_ptr, MARK_TEXT);
            break;

        case EXPAND_AFTER: /*385:*/
            if (cur_chr == 0) {
                get_token();
                t = cur_tok;
                get_token();
                if (cur_cmd > MAX_COMMAND)
                    expand();
                else
                    back_input();
                cur_tok = t;
                back_input();
            } else { /*1553: "\unless" implementation */
                get_token();

                if (cur_cmd == IF_TEST && cur_chr != IF_CASE_CODE) {
                    cur_chr = cur_chr + UNLESS_CODE;
                    goto reswitch;
                }

                error_here_with_diagnostic("You can't use `");
                print_esc_cstr("unless");
                print_cstr("' before `");
                print_cmd_chr(cur_cmd, cur_chr);
                print_char('\'');
                capture_to_diagnostic(NULL);
                help_ptr = 1;
                help_line[0] = "Continue, and I'll forget that it ever happened.";
                back_error();
            }
            break;

        case NO_EXPAND: /*386:*/
            if (cur_chr == 0) {
                save_scanner_status = scanner_status;
                scanner_status = NORMAL;
                get_token();
                scanner_status = save_scanner_status;
                t = cur_tok;
                back_input();
                if (t >= CS_TOKEN_FLAG) {
                    p = get_avail();
                    mem[p].b32.s0 = CS_TOKEN_FLAG + FROZEN_DONT_EXPAND;
                    mem[p].b32.s1 = cur_input.loc;
                    cur_input.start = p;
                    cur_input.loc = p;
                }
            } else { /*387: \primitive implementation */
                save_scanner_status = scanner_status;
                scanner_status = NORMAL;
                get_token();
                scanner_status = save_scanner_status;

                if (cur_cs < HASH_BASE)
                    cur_cs = prim_lookup(cur_cs - SINGLE_BASE);
                else
                    cur_cs = prim_lookup(hash[cur_cs].s1);

                if (cur_cs != UNDEFINED_PRIMITIVE) {
                    t = eqtb[PRIM_EQTB_BASE + cur_cs].b16.s1;
                    if (t > MAX_COMMAND) {
                        cur_cmd = t;
                        cur_chr = eqtb[PRIM_EQTB_BASE + cur_cs].b32.s1;
                        cur_tok = (cur_cmd * MAX_CHAR_VAL) + cur_chr;
                        cur_cs = 0;
                        goto reswitch;
                    } else {
                        back_input();
                        p = get_avail();
                        mem[p].b32.s0 = CS_TOKEN_FLAG + FROZEN_PRIMITIVE;
                        mem[p].b32.s1 = cur_input.loc;
                        cur_input.loc = p;
                        cur_input.start = p;
                    }
                }
            }
            break;

        case CS_NAME:
            r = get_avail();
            p = r;
            b = is_in_csname;
            is_in_csname = true;

            do {
                get_x_token();
                if (cur_cs == 0) {
                    q = get_avail();
                    mem[p].b32.s1 = q;
                    mem[q].b32.s0 = cur_tok;
                    p = q;
                }
            } while (cur_cs == 0);

            if (cur_cmd != END_CS_NAME) { /*391:*/
                error_here_with_diagnostic("Missing ");
                print_esc_cstr("endcsname");
                print_cstr(" inserted");
                capture_to_diagnostic(NULL);

                help_ptr = 2;
                help_line[1] = "The control sequence marked <to be read again> should";
                help_line[0] = "not appear between \\csname and \\endcsname.";
                back_error();
            }

            is_in_csname = b;
            j = first;
            p = mem[r].b32.s1;

            while (p != TEX_NULL) {
                if (j >= max_buf_stack) {
                    max_buf_stack = j + 1;
                    if (max_buf_stack == buf_size)
                        overflow("buffer size", buf_size);
                }
                buffer[j] = mem[p].b32.s0 % MAX_CHAR_VAL;
                j++;
                p = LLIST_link(p);
            }

            if (j > first + 1 || buffer[first] > 65535L) {
                no_new_control_sequence = false;
                cur_cs = id_lookup(first, j - first);
                no_new_control_sequence = true;
            } else if (j == first) {
                cur_cs = NULL_CS;
            } else {
                cur_cs = SINGLE_BASE + buffer[first]; /*:392*/
            }

            flush_list(r);

            if (eqtb[cur_cs].b16.s1 == UNDEFINED_CS)
                eq_define(cur_cs, RELAX, TOO_BIG_USV);

            cur_tok = cur_cs + CS_TOKEN_FLAG;
            back_input();
            break;

        case CONVERT:
            conv_toks();
            break;

        case THE:
            ins_the_toks();
            break;

        case IF_TEST:
            conditional();
            break;

        case FI_OR_ELSE:
            if (INTPAR(tracing_ifs) > 0) {
                if (INTPAR(tracing_commands) <= 1)
                    show_cur_cmd_chr();
            }

            if (cur_chr > if_limit) {
                if (if_limit == IF_CODE) {
                    insert_relax();
                } else {
                    error_here_with_diagnostic("Extra ");
                    print_cmd_chr(FI_OR_ELSE, cur_chr);
                    capture_to_diagnostic(NULL);

                    help_ptr = 1;
                    help_line[0] = "I'm ignoring this; it doesn't match any \\if.";
                    error();
                }
            } else {
                while (cur_chr != FI_CODE)
                    pass_text();

                if (if_stack[in_open] == cond_ptr)
                    if_warning();
                p = cond_ptr;
                if_line = mem[p + 1].b32.s1;
                cur_if = mem[p].b16.s0;
                if_limit = mem[p].b16.s1;
                cond_ptr = mem[p].b32.s1;
                free_node(p, IF_NODE_SIZE);
            }
            break;

        case INPUT:
            if (cur_chr == 1) /* \endinput */
                force_eof = true; /*1537:*/
            else if (cur_chr == 2) /* \scantokens */
                pseudo_start();
            else if (name_in_progress)
                insert_relax();
            else /* \input */
                start_input(NULL);
            break;

        default:
            error_here_with_diagnostic("Undefined control sequence");
            capture_to_diagnostic(NULL);

            help_ptr = 5;
            help_line[4] = "The control sequence at the end of the top line";
            help_line[3] = "of your error message was never \\def'ed. If you have";
            help_line[2] = "misspelled it (e.g., `\\hobx'), type `I' and the correct";
            help_line[1] = "spelling (e.g., `I\\hbox'). Otherwise just continue,";
            help_line[0] = "and I'll forget about whatever was undefined.";
            error();
            break;
        }
    } else if (cur_cmd < END_TEMPLATE) {
        macro_call();
    } else { /*393:*/
        cur_tok = CS_TOKEN_FLAG + FROZEN_ENDV;
        back_input();
    }

    cur_val = cv_backup;
    cur_val_level = cvl_backup;
    radix = radix_backup;
    cur_order = co_backup;
    mem[BACKUP_HEAD].b32.s1 = backup_backup;
    expand_depth_count--;
}


/* get_x_token: translated to Rust in src/scan.rs */

/* x_token: translated to Rust in src/scan.rs */


/* scan_left_brace: translated to Rust in src/scan.rs */


/* scan_optional_equals: translated to Rust in src/scan.rs */


/* scan_keyword: translated to Rust in src/scan.rs */

/* mu_error translated to Rust (src/scan.rs). */

void scan_glyph_number(internal_font_number f)
{
    if (scan_keyword("/")) {
        scan_and_pack_name();
        {
            cur_val = map_glyph_to_index(f);
            cur_val_level = INT_VAL;
        }
    } else if (scan_keyword("u")) {
        scan_char_num();
        {
            cur_val = map_char_to_glyph(f, cur_val);
            cur_val_level = INT_VAL;
        }
    } else
        scan_int();
}

/* scan_char_class translated to Rust (src/scan.rs). */
/* scan_char_class_not_ignored translated to Rust (src/scan.rs). */
/* scan_eight_bit_int translated to Rust (src/scan.rs). */
/* scan_usv_num translated to Rust (src/scan.rs). */
/* scan_char_num translated to Rust (src/scan.rs). */

/* scan_xetex_math_char_int translated to Rust (src/scan.rs). */

/* scan_math: translated to Rust in src/scan.rs */

/* set_math_char: translated to Rust in src/scan.rs */

/* scan_math_class_int translated to Rust (src/scan.rs). */
/* scan_math_fam_int translated to Rust (src/scan.rs). */
/* scan_four_bit_int translated to Rust (src/scan.rs). */
/* scan_fifteen_bit_int translated to Rust (src/scan.rs). */
/* scan_delimiter_int translated to Rust (src/scan.rs). */


/* scan_register_num translated to Rust (src/scan.rs). */
/* scan_four_bit_int_or_18 translated to Rust (src/scan.rs). */

/* get_x_or_protected: translated to Rust in src/scan.rs */


/* effective_char: translated to Rust in src/scan.rs */


/* scan_font_ident: translated to Rust in src/scan.rs */
#if 0
void scan_font_ident(void)
{
    internal_font_number f;
    int32_t m;

    do {
        get_x_token();
    } while (cur_cmd == SPACER);

    if (cur_cmd == DEF_FONT)
        f = eqtb[CUR_FONT_LOC].b32.s1;
    else if (cur_cmd == SET_FONT)
        f = cur_chr;
    else if (cur_cmd == DEF_FAMILY) {
        m = cur_chr;
        scan_math_fam_int();
        f = eqtb[m + cur_val].b32.s1;
    } else {
        error_here_with_diagnostic("Missing font identifier");
        capture_to_diagnostic(NULL);
        {
            help_ptr = 2;
            help_line[1] = "I was looking for a control sequence whose";
            help_line[0] = "current meaning has been defined by \\font.";
        }
        back_error();
        f = FONT_BASE;
    }
    cur_val = f;
}
#endif

/* find_font_dimen: translated to Rust in src/scan.rs */


/* scan_something_internal: translated to Rust in src/scan.rs */


/* scan_int: translated to Rust in src/scan.rs */


static scaled_t
round_decimals(small_number k)
{
    int32_t a = 0;

    while (k > 0) {
        k--;
        a = (a + dig[k] * 0x20000) / 10;
    }

    return (a + 1) / 2;
}


void
xetex_scan_dimen(bool mu, bool inf, bool shortcut, bool requires_units)
{
    bool negative;
    int32_t f;
    int32_t num, denom;
    small_number k, kk;
    int32_t p, q;
    scaled_t v;
    int32_t save_cur_val;

    f = 0;
    arith_error = false;
    cur_order = NORMAL;
    negative = false;

    if (!shortcut) {
        negative = false;

        do {
            do {
                get_x_token();
            } while (cur_cmd == SPACER);

            if (cur_tok == OTHER_TOKEN + '-' ) {
                negative = !negative;
                cur_tok = OTHER_TOKEN + '+';
            }
        } while (cur_tok == OTHER_TOKEN + '+');

        if (cur_cmd >= MIN_INTERNAL && cur_cmd <= MAX_INTERNAL) { /*468:*/
            if (mu) {
                scan_something_internal(MU_VAL, false);
                if (cur_val_level >= GLUE_VAL) {
                    v = mem[cur_val + 1].b32.s1;
                    delete_glue_ref(cur_val);
                    cur_val = v;
                }

                if (cur_val_level == MU_VAL)
                    goto attach_sign;
                if (cur_val_level != INT_VAL)
                    mu_error();
            } else {
                scan_something_internal(DIMEN_VAL, false);
                if (cur_val_level == DIMEN_VAL)
                    goto attach_sign;
            }
        } else {
            back_input();

            if (cur_tok == CONTINENTAL_POINT_TOKEN)
                cur_tok = POINT_TOKEN;

            if (cur_tok != POINT_TOKEN) {
                scan_int();
            } else {
                radix = 10;
                cur_val = 0;
            }

            if (cur_tok == CONTINENTAL_POINT_TOKEN)
                cur_tok = POINT_TOKEN;

            if (radix == 10 && cur_tok == POINT_TOKEN) { /*471:*/
                k = 0;
                p = TEX_NULL;
                get_token();

                while (true) {
                    get_x_token();
                    if (cur_tok > ZERO_TOKEN + 9 || cur_tok < ZERO_TOKEN)
                        goto done1;

                    if (k < 17) {
                        q = get_avail();
                        mem[q].b32.s1 = p;
                        mem[q].b32.s0 = cur_tok - ZERO_TOKEN;
                        p = q;
                        k++;
                    }
                }

            done1:
                for (kk = k; kk >= 1; kk--) {
                    dig[kk - 1] = mem[p].b32.s0;
                    q = p;
                    p = LLIST_link(p);
                    mem[q].b32.s1 = avail;
                    avail = q;
                }

                f = round_decimals(k);
                if (cur_cmd != SPACER)
                    back_input();
            }
        }
    }

    if (cur_val < 0) {
        negative = !negative;
        cur_val = -(int32_t) cur_val;
    }

    if (requires_units) {
        if (inf) { /*473:*/
            if (scan_keyword("fil")) {
                cur_order = FIL;

                while (scan_keyword("l")) {
                    if (cur_order == FILLL) {
                        error_here_with_diagnostic("Illegal unit of measure (replaced with filll)");
                        capture_to_diagnostic(NULL);

                        help_ptr = 1;
                        // "ddon't" looks like wordplay from Knuth inspired by "filll"
                        help_line[0] = "I dddon't go any higher than filll.";
                        error();
                    } else {
                        cur_order++;
                    }
                }

                goto attach_fraction;
            }
        }

        save_cur_val = cur_val;

        do {
            get_x_token();
        } while (cur_cmd == SPACER);

        if (cur_cmd < MIN_INTERNAL || cur_cmd > MAX_INTERNAL) {
            back_input();
        } else {
            if (mu) {
                scan_something_internal(MU_VAL, false);
                if (cur_val_level >= GLUE_VAL) {
                    v = mem[cur_val + 1].b32.s1;
                    delete_glue_ref(cur_val);
                    cur_val = v;
                }
                if (cur_val_level != MU_VAL)
                    mu_error();
            } else {
                scan_something_internal(DIMEN_VAL, false);
            }

            v = cur_val;
            goto found;
        }

        if (mu)
            goto not_found;

        if (scan_keyword("em"))
            v = font_info[QUAD_CODE + param_base[eqtb[CUR_FONT_LOC].b32.s1]].b32.s1;
        else if (scan_keyword("ex"))
            v = font_info[X_HEIGHT_CODE + param_base[eqtb[CUR_FONT_LOC].b32.s1]].b32.s1;
        else
            goto not_found;

        get_x_token();
        if (cur_cmd != SPACER)
            back_input();

    found:
        cur_val = mult_and_add(save_cur_val, v, xn_over_d(v, f, 65536L), MAX_HALFWORD);
        goto attach_sign;

    not_found:
        if (mu) { /*475:*/
            if (scan_keyword("mu")) {
                goto attach_fraction;
            } else {
                error_here_with_diagnostic("Illegal unit of measure (mu inserted)");
                capture_to_diagnostic(NULL);

                help_ptr = 4;
                help_line[3] = "The unit of measurement in math glue must be mu.";
                help_line[2] = "To recover gracefully from this error, it's best to";
                help_line[1] = "delete the erroneous units; e.g., type `2' to delete";
                help_line[0] = "two letters. (See Chapter 27 of The TeXbook.)";
                error();
                goto attach_fraction;
            }
        }

        if (scan_keyword("true")) { /*476:*/
            prepare_mag();
            if (INTPAR(mag) != 1000) {
                cur_val = xn_over_d(cur_val, 1000, INTPAR(mag));
                f = (1000 * f + 65536L * tex_remainder) / INTPAR(mag);
                cur_val = cur_val + (f / 65536L);
                f = f % 65536L;
            }
        }

        if (scan_keyword("pt"))
            goto attach_fraction;

        if (scan_keyword("in")) {
            num = 7227; /* magic ratio consant */
            denom = 100;
        } else if (scan_keyword("pc")) {
            num = 12;
            denom = 1;
        } else if (scan_keyword("cm")) {
            num = 7227; /* magic ratio consant */
            denom = 254; /* magic ratio consant */
        } else if (scan_keyword("mm")) {
            num = 7227; /* magic ratio consant */
            denom = 2540; /* magic ratio consant */
        } else if (scan_keyword("bp")) {
            num = 7227; /* magic ratio consant */
            denom = 7200; /* magic ratio consant */
        } else if (scan_keyword("dd")) {
            num = 1238; /* magic ratio consant */
            denom = 1157; /* magic ratio consant */
        } else if (scan_keyword("cc")) {
            num = 14856; /* magic ratio consant */
            denom = 1157; /* magic ratio consant */
        } else if (scan_keyword("sp")) {
            goto done;
        } else { /*478:*/
            error_here_with_diagnostic("Illegal unit of measure (pt inserted)");
            capture_to_diagnostic(NULL);

            help_ptr = 6;
            help_line[5] = "Dimensions can be in units of em, ex, in, pt, pc,";
            help_line[4] = "cm, mm, dd, cc, bp, or sp; but yours is a new one!";
            help_line[3] = "I'll assume that you meant to say pt, for printer's points.";
            help_line[2] = "To recover gracefully from this error, it's best to";
            help_line[1] = "delete the erroneous units; e.g., type `2' to delete";
            help_line[0] = "two letters. (See Chapter 27 of The TeXbook.)";
            error();
            goto done2;
        }

        cur_val = xn_over_d(cur_val, num, denom);
        f = (num * f + 65536L * tex_remainder) / denom;
        cur_val = cur_val + (f / 65536L);
        f = f % 65536L;

    done2:
        ;
    attach_fraction:

        if (cur_val >= 16384)
            arith_error = true;
        else
            cur_val = cur_val * 65536L + f;

    done:
        get_x_token();
        if (cur_cmd != SPACER)
            back_input();
    } else { /* if(requires_units) */
        if (cur_val >= 16384)
            arith_error = true;
        else
            cur_val = cur_val * 65536L + f;
    }

attach_sign:
    if (arith_error || abs(cur_val) >= 0x40000000) { /*479:*/
        error_here_with_diagnostic("Dimension too large");
        capture_to_diagnostic(NULL);

        help_ptr = 2;
        help_line[1] = "I can't work with sizes bigger than about 19 feet.";
        help_line[0] = "Continue and I'll use the largest value I can.";
        error();
        cur_val = MAX_HALFWORD;
        arith_error = false;
    }

    if (negative)
        cur_val = -(int32_t) cur_val;
}


/* scan_dimen: translated to Rust in src/scan.rs */
/* scan_decimal: translated to Rust in src/scan.rs */


/* scan_glue: translated to Rust in src/scan.rs */


/* add_or_sub: translated to Rust in src/scaled_math.rs */

/* quotient: translated to Rust in src/scaled_math.rs */

/* fract: translated to Rust in src/scaled_math.rs */

/* scan_expr: translated to Rust in src/scan.rs */
#if 0
void scan_expr_DELETED(void)
{
    bool a, b;
    small_number l;
    small_number r;
    small_number s;
    small_number o;
    int32_t e;
    int32_t t;
    int32_t f;
    int32_t n;
    int32_t p;
    int32_t q;
    l = cur_val_level;
    a = arith_error;
    b = false;
    p = TEX_NULL;

    expand_depth_count++;
    if (expand_depth_count >= expand_depth)
        overflow("expansion depth", expand_depth);

restart:
    r = EXPR_NONE;
    e = 0;
    s = EXPR_NONE;
    t = 0;
    n = 0;
continue_:
    if (s == EXPR_NONE)
        o = l;
    else
        o = INT_VAL;
    do {
        get_x_token();
    } while (cur_cmd == SPACER);
    if (cur_tok == (OTHER_TOKEN + 40)) {    /*1576: */
        q = get_node(EXPR_NODE_SIZE);
        mem[q].b32.s1 = p;
        mem[q].b16.s1 = l;
        mem[q].b16.s0 = 4 * s + r;
        mem[q + 1].b32.s1 = e;
        mem[q + 2].b32.s1 = t;
        mem[q + 3].b32.s1 = n;
        p = q;
        l = o;
        goto restart;
    }
    back_input();
    if (o == INT_VAL)
        scan_int();
    else if (o == DIMEN_VAL)
        scan_dimen(false, false, false);
    else if (o == GLUE_VAL)
        scan_normal_glue();
    else
        scan_mu_glue();
    f = /*:1573 */ cur_val;
found: /*1572:*//*424:*/
    do {
        get_x_token();
    } while (cur_cmd == SPACER);
    if (cur_tok == (OTHER_TOKEN + 43))
        o = EXPR_ADD;
    else if (cur_tok == (OTHER_TOKEN + 45))
        o = EXPR_SUB;
    else if (cur_tok == (OTHER_TOKEN + 42))
        o = EXPR_MULT;
    else if (cur_tok == (OTHER_TOKEN + 47))
        o = EXPR_DIV;
    else {

        o = EXPR_NONE;
        if (p == TEX_NULL) {
            if (cur_cmd != RELAX)
                back_input();
        } else if (cur_tok != (OTHER_TOKEN + 41)) {
            error_here_with_diagnostic("Missing ) inserted for expression");
            capture_to_diagnostic(NULL);

            {
                help_ptr = 1;
                help_line[0] = "I was expecting to see `+', `-', `*', `/', or `)'. Didn't.";
            }
            back_error();
        }
    }
    arith_error = b;
    if ((l == INT_VAL) || (s > EXPR_SUB)) {
        if ((f > TEX_INFINITY) || (f < -TEX_INFINITY)) {
            arith_error = true;
            f = 0;
        }
    } else if (l == DIMEN_VAL) {
        if (abs(f) > MAX_HALFWORD) {
            arith_error = true;
            f = 0;
        }
    } else {

        if ((abs(mem[f + 1].b32.s1) > MAX_HALFWORD) || (abs(mem[f + 2].b32.s1) > MAX_HALFWORD)
            || (abs(mem[f + 3].b32.s1) > MAX_HALFWORD)) {
            arith_error = true;
            delete_glue_ref(f);
            f = new_spec(0);
        }
    }
    switch (s) {                /*1579: */
    case 0:
        if ((l >= GLUE_VAL) && (o != EXPR_NONE)) {
            t = new_spec(f);
            delete_glue_ref(f);
            if (mem[t + 2].b32.s1 == 0)
                mem[t].b16.s1 = NORMAL;
            if (mem[t + 3].b32.s1 == 0)
                mem[t].b16.s0 = NORMAL;
        } else
            t = f;
        break;
    case 3:
        if (o == EXPR_DIV) {
            n = f;
            o = EXPR_SCALE;
        } else if (l == INT_VAL)
            t = mult_and_add(t, f, 0, TEX_INFINITY);
        else if (l == DIMEN_VAL)
            t = mult_and_add(t, f, 0, MAX_HALFWORD);
        else {

            mem[t + 1].b32.s1 = mult_and_add(mem[t + 1].b32.s1, f, 0, MAX_HALFWORD);
            mem[t + 2].b32.s1 = mult_and_add(mem[t + 2].b32.s1, f, 0, MAX_HALFWORD);
            mem[t + 3].b32.s1 = mult_and_add(mem[t + 3].b32.s1, f, 0, MAX_HALFWORD);
        }
        break;
    case 4:
        if (l < GLUE_VAL)
            t = quotient(t, f);
        else {

            mem[t + 1].b32.s1 = quotient(mem[t + 1].b32.s1, f);
            mem[t + 2].b32.s1 = quotient(mem[t + 2].b32.s1, f);
            mem[t + 3].b32.s1 = quotient(mem[t + 3].b32.s1, f);
        }
        break;
    case 5:
        if (l == INT_VAL)
            t = fract(t, n, f, TEX_INFINITY);
        else if (l == DIMEN_VAL)
            t = fract(t, n, f, MAX_HALFWORD);
        else {

            mem[t + 1].b32.s1 = fract(mem[t + 1].b32.s1, n, f, MAX_HALFWORD);
            mem[t + 2].b32.s1 = fract(mem[t + 2].b32.s1, n, f, MAX_HALFWORD);
            mem[t + 3].b32.s1 = fract(mem[t + 3].b32.s1, n, f, MAX_HALFWORD);
        }
        break;
    }
    if (o > EXPR_SUB)
        s = o;
    else {                      /*1580: */

        s = EXPR_NONE;
        if (r == EXPR_NONE)
            e = t;
        else if (l == INT_VAL)
            e = add_or_sub(e, t, TEX_INFINITY, r == EXPR_SUB);
        else if (l == DIMEN_VAL)
            e = add_or_sub(e, t, MAX_HALFWORD, r == EXPR_SUB);
        else {                  /*1582: */

            mem[e + 1].b32.s1 = add_or_sub(mem[e + 1].b32.s1, mem[t + 1].b32.s1, MAX_HALFWORD, r == EXPR_SUB);
            if (mem[e].b16.s1 == mem[t].b16.s1)
                mem[e + 2].b32.s1 = add_or_sub(mem[e + 2].b32.s1, mem[t + 2].b32.s1, MAX_HALFWORD, r == EXPR_SUB);
            else if ((mem[e].b16.s1 < mem[t].b16.s1) && (mem[t + 2].b32.s1 != 0)) {
                mem[e + 2].b32.s1 = mem[t + 2].b32.s1;
                mem[e].b16.s1 = mem[t].b16.s1;
            }
            if (mem[e].b16.s0 == mem[t].b16.s0)
                mem[e + 3].b32.s1 = add_or_sub(mem[e + 3].b32.s1, mem[t + 3].b32.s1, MAX_HALFWORD, r == EXPR_SUB);
            else if ((mem[e].b16.s0 < mem[t].b16.s0) && (mem[t + 3].b32.s1 != 0)) {
                mem[e + 3].b32.s1 = mem[t + 3].b32.s1;
                mem[e].b16.s0 = mem[t].b16.s0;
            }
            delete_glue_ref(t);
            if (mem[e + 2].b32.s1 == 0)
                mem[e].b16.s1 = NORMAL;
            if (mem[e + 3].b32.s1 == 0)
                mem[e].b16.s0 = NORMAL;
        }
        r = o;
    }
    b = arith_error;
    if (o != EXPR_NONE)
        goto continue_;
    if (p != TEX_NULL) {     /*1577: */
        f = e;
        q = p;
        e = mem[q + 1].b32.s1;
        t = mem[q + 2].b32.s1;
        n = mem[q + 3].b32.s1;
        s = mem[q].b16.s0 / 4;
        r = mem[q].b16.s0 % 4;
        l = mem[q].b16.s1;
        p = mem[q].b32.s1;
        free_node(q, EXPR_NODE_SIZE);
        goto found;
    }

    expand_depth_count--;

    if (b) {
        error_here_with_diagnostic("Arithmetic overflow");
        capture_to_diagnostic(NULL);

        {
            help_ptr = 2;
            help_line[1] = "I can't evaluate this expression,";
            help_line[0] = "since the result is out of range.";
        }
        error();
        if (l >= GLUE_VAL) {
            delete_glue_ref(e);
            e = 0;
            GLUE_SPEC_ref_count(e)++;
        } else
            e = 0;
    }
#endif /* scan_expr_DELETED */

/* scan_normal_glue: translated to Rust in src/scan.rs */
/* scan_mu_glue: translated to Rust in src/scan.rs */

/* scan_rule_spec: translated to Rust in src/scan.rs */

/* scan_general_text: translated to Rust in src/scan.rs */

/* pseudo_start: translated to Rust in src/scan.rs */
#if 0
void pseudo_start(void)
{
    unsigned char /*max_selector */ old_setting;
    str_number s;
    pool_pointer l, m;
    int32_t p, q, r;
    b16x4 w;
    int32_t nl, sz;

    scan_general_text();
    old_setting = selector;
    selector = SELECTOR_NEW_STRING ;
    token_show(TEMP_HEAD);
    selector = old_setting;
    flush_list(mem[TEMP_HEAD].b32.s1);
    {
        if (pool_ptr + 1 > pool_size)
            overflow("pool size", pool_size - init_pool_ptr);
    }
    s = make_string();
    str_pool[pool_ptr] = ' ' ;
    l = str_start[(s) - 65536L];
    nl = INTPAR(new_line_char);
    p = get_avail();
    q = p;
    while (l < pool_ptr) {

        m = l;
        while ((l < pool_ptr) && (str_pool[l] != nl))
            l++;
        sz = (l - m + 7) / 4;
        if (sz == 1)
            sz = 2;
        r = get_node(sz);
        mem[q].b32.s1 = r;
        q = r;
        mem[q].b32.s0 = sz;
        while (sz > 2) {

            sz--;
            r++;
            w.s3 = str_pool[m];
            w.s2 = str_pool[m + 1];
            w.s1 = str_pool[m + 2];
            w.s0 = str_pool[m + 3];
            mem[r].b16 = w;
            m = m + 4;
        }
        w.s3 = ' ' ;
        w.s2 = ' ' ;
        w.s1 = ' ' ;
        w.s0 = ' ' ;
        if (l > m) {
            w.s3 = str_pool[m];
            if (l > m + 1) {
                w.s2 = str_pool[m + 1];
                if (l > m + 2) {
                    w.s1 = str_pool[m + 2];
                    if (l > m + 3)
                        w.s0 = str_pool[m + 3];
                }
            }
        }
        mem[r + 1].b16 = w;
        if (str_pool[l] == nl)
            l++;
    }
    mem[p].b32.s0 = mem[p].b32.s1;
    mem[p].b32.s1 = pseudo_files;
    pseudo_files = /*:1542 */ p;
    {
        str_ptr--;
        pool_ptr = str_start[str_ptr - TOO_BIG_CHAR];
    }
    begin_file_reading();
    line = 0;
    cur_input.limit = cur_input.start;
    cur_input.loc = cur_input.limit + 1;
    if (INTPAR(tracing_scan_tokens) > 0) {
        if (term_offset > max_print_line - 3)
            print_ln();
        else if ((term_offset > 0) || (file_offset > 0))
            print_char(' ');
        cur_input.name = 19;
        print_cstr("( ");
        open_parens++;
        ttstub_output_flush (rust_stdout);
    } else {

        cur_input.name = 18;
        cur_input.synctex_tag = 0;
    }
}
#endif

/* str_toks_cat, str_toks: translated to Rust in src/scan.rs */
#if 0
/* "The token list created by str toks begins at `LLIST_link(TEMP_HEAD)` and
 * ends at the value `p` that is returned. (If `p = temp head`, the list is
 * empty.)" */
int32_t
str_toks_cat(pool_pointer b, small_number cat)
{
    int32_t p;
    int32_t q;
    int32_t t;
    pool_pointer k;

    if (pool_ptr + 1 > pool_size)
        overflow("pool size", pool_size - init_pool_ptr);

    p = TEMP_HEAD;
    LLIST_link(p) = TEX_NULL;
    k = b;

    while (k < pool_ptr) {
        t = str_pool[k];

        if (t == ' ' && cat == 0) {
            t = SPACE_TOKEN;
        } else {
            if (t >= 0xD800 && t < 0xDC00 && k + 1 < pool_ptr && str_pool[k + 1] >= 0xDC00 && str_pool[k + 1] < 0xE000) {
                k++;
                t = 0x10000 + ((t - 0xD800) << 10) + (str_pool[k] - 0xDC00);
            }

            if (cat == 0)
                t = OTHER_TOKEN + t;
            else if (cat == ACTIVE_CHAR)
                t = CS_TOKEN_FLAG + 1 + t;
            else
                t = MAX_CHAR_VAL * cat + t;
        }

        q = avail;

        if (q == TEX_NULL) {
            q = get_avail();
        } else {
            avail = LLIST_link(q);
            LLIST_link(q) = TEX_NULL;
        }

        LLIST_link(p) = q;
        LLIST_info(q) = t;
        p = q;
        k++;
    }

    pool_ptr = b;
    return p;
}


int32_t
str_toks(pool_pointer b)
{
    return str_toks_cat(b, 0);
}
#endif


/* the_toks: translated to Rust in src/scan.rs */
#if 0
int32_t the_toks(void)
{
    unsigned char /*max_selector */ old_setting;
    int32_t p, q, r;
    pool_pointer b;
    small_number c;
    if (odd(cur_chr)) {
        c = cur_chr;
        scan_general_text();
        if (c == 1)
            return cur_val;
        else {

            old_setting = selector;
            selector = SELECTOR_NEW_STRING ;
            b = pool_ptr;
            p = get_avail();
            mem[p].b32.s1 = mem[TEMP_HEAD].b32.s1;
            token_show(p);
            flush_list(p);
            selector = old_setting;
            return str_toks(b);
        }
    }
    get_x_token();
    scan_something_internal(TOK_VAL, false);
    if (cur_val_level >= IDENT_VAL) {   /*485: */
        p = TEMP_HEAD;
        mem[p].b32.s1 = TEX_NULL;
        if (cur_val_level == IDENT_VAL) {
            q = get_avail();
            mem[p].b32.s1 = q;
            mem[q].b32.s0 = CS_TOKEN_FLAG + cur_val;
            p = q;
        } else if (cur_val != TEX_NULL) {
            r = mem[cur_val].b32.s1;
            while (r != TEX_NULL) {

                {
                    {
                        q = avail;
                        if (q == TEX_NULL)
                            q = get_avail();
                        else {

                            avail = mem[q].b32.s1;
                            mem[q].b32.s1 = TEX_NULL;
                        }
                    }
                    mem[p].b32.s1 = q;
                    mem[q].b32.s0 = mem[r].b32.s0;
                    p = q;
                }
                r = LLIST_link(r);
            }
        }
        return p;
    } else {

        old_setting = selector;
        selector = SELECTOR_NEW_STRING ;
        b = pool_ptr;
        switch (cur_val_level) {
        case 0:
            print_int(cur_val);
            break;
        case 1:
            {
                print_scaled(cur_val);
                print_cstr("pt");
            }
            break;
        case 2:
            {
                print_spec(cur_val, "pt");
                delete_glue_ref(cur_val);
            }
            break;
        case 3:
            {
                print_spec(cur_val, "mu");
                delete_glue_ref(cur_val);
            }
            break;
        }
        selector = old_setting;
        return str_toks(b);
    }
}
#endif

/* ins_the_toks: translated to Rust in src/scan.rs */
#if 0
void ins_the_toks(void)
{
    mem[GARBAGE].b32.s1 = the_toks();
    begin_token_list(mem[TEMP_HEAD].b32.s1, INSERTED);
}
#endif


/* conv_toks: translated to Rust in src/scan.rs */
#if 0
void
conv_toks(void)
{
    unsigned char old_setting;
    int32_t save_warning_index, save_def_ref;
    bool boolvar;
    str_number s;
    str_number u;
    small_number c;
    small_number save_scanner_status;
    pool_pointer b;
    int32_t fnt = 0, arg1 = 0, arg2 = 0;
    str_number font_name_str;
    small_number i;
    UTF16_code quote_char;
    small_number cat;
    UnicodeScalar saved_chr;
    int32_t p = TEX_NULL, q, j;

    cat = 0;
    c = cur_chr;

    switch (c) {
    case NUMBER_CODE:
    case ROMAN_NUMERAL_CODE:
        scan_int();
        break;

    case STRING_CODE:
    case MEANING_CODE:
        save_scanner_status = scanner_status;
        scanner_status = NORMAL;
        get_token();
        scanner_status = save_scanner_status;
        break;

    case FONT_NAME_CODE:
        scan_font_ident();
        break;

    case ETEX_REVISION_CODE:
        break;

    case EXPANDED_CODE:
        save_scanner_status = scanner_status;
        save_warning_index = warning_index;
        save_def_ref = def_ref;
        if (str_start[str_ptr - TOO_BIG_CHAR] < pool_ptr)
            u = make_string();
        else
            u = 0;
        scan_pdf_ext_toks();
        warning_index = save_warning_index;
        scanner_status = save_scanner_status;
        begin_token_list(mem[def_ref].b32.s1, INSERTED);
        mem[def_ref].b32.s1 = avail;
        avail = def_ref;
        def_ref = save_def_ref;
        if (u != 0)
            str_ptr--;
        return;

    case LEFT_MARGIN_KERN_CODE:
    case RIGHT_MARGIN_KERN_CODE:
        scan_register_num();

        if (cur_val < 256) {
            p = BOX_REG(cur_val);
        } else {
            find_sa_element(4, cur_val, false);
            if (cur_ptr == TEX_NULL)
                p = TEX_NULL;
            else
                p = mem[cur_ptr + 1].b32.s1;
        }

        if (p == TEX_NULL || NODE_type(p) != HLIST_NODE)
            pdf_error("marginkern", "a non-empty hbox expected");
        break;

    case PDF_CREATION_DATE_CODE:
        b = pool_ptr;
        getcreationdate();
        mem[GARBAGE].b32.s1 = str_toks(b);
        begin_token_list(mem[TEMP_HEAD].b32.s1, INSERTED);
        break;

    case PDF_FILE_MOD_DATE_CODE:
        save_scanner_status = scanner_status;
        save_warning_index = warning_index;
        save_def_ref = def_ref;
        if (str_start[str_ptr - TOO_BIG_CHAR] < pool_ptr)
            u = make_string();
        else
            u = 0;
        scan_pdf_ext_toks();

        if (selector == SELECTOR_NEW_STRING)
            pdf_error("tokens", "tokens_to_string() called while selector = new_string");

        old_setting = selector;
        selector = SELECTOR_NEW_STRING;
        show_token_list(mem[def_ref].b32.s1, TEX_NULL, pool_size - pool_ptr);
        selector = old_setting;
        s = make_string();
        delete_token_ref(def_ref);
        def_ref = save_def_ref;
        warning_index = save_warning_index;
        scanner_status = save_scanner_status;
        b = pool_ptr;
        getfilemoddate(s);  /* <= the difference-maker */
        mem[GARBAGE].b32.s1 = str_toks(b);

        if (s == str_ptr - 1) {
            str_ptr--;
            pool_ptr = str_start[str_ptr - TOO_BIG_CHAR];
        }

        begin_token_list(mem[TEMP_HEAD].b32.s1, INSERTED);
        if (u != 0)
            str_ptr--;
        return;

    case PDF_FILE_SIZE_CODE:
        save_scanner_status = scanner_status;
        save_warning_index = warning_index;
        save_def_ref = def_ref;
        if (str_start[str_ptr - TOO_BIG_CHAR] < pool_ptr)
            u = make_string();
        else
            u = 0;
        scan_pdf_ext_toks();

        if (selector == SELECTOR_NEW_STRING)
            pdf_error("tokens", "tokens_to_string() called while selector = new_string");

        old_setting = selector;
        selector = SELECTOR_NEW_STRING;
        show_token_list(mem[def_ref].b32.s1, TEX_NULL, pool_size - pool_ptr);
        selector = old_setting;
        s = make_string();
        delete_token_ref(def_ref);
        def_ref = save_def_ref;
        warning_index = save_warning_index;
        scanner_status = save_scanner_status;
        b = pool_ptr;
        getfilesize(s);  /* <= the difference-maker */
        mem[GARBAGE].b32.s1 = str_toks(b);

        if (s == str_ptr - 1) {
            str_ptr--;
            pool_ptr = str_start[str_ptr - TOO_BIG_CHAR];
        }

        begin_token_list(mem[TEMP_HEAD].b32.s1, INSERTED);
        if (u != 0)
            str_ptr--;
        return;

    case PDF_MDFIVE_SUM_CODE:
        save_scanner_status = scanner_status;
        save_warning_index = warning_index;
        save_def_ref = def_ref;

        if (str_start[str_ptr - TOO_BIG_CHAR] < pool_ptr)
            u = make_string();
        else
            u = 0;

        boolvar = scan_keyword("file");
        scan_pdf_ext_toks();

        if (selector == SELECTOR_NEW_STRING)
            pdf_error("tokens", "tokens_to_string() called while selector = new_string");

        old_setting = selector;
        selector = SELECTOR_NEW_STRING;
        show_token_list(mem[def_ref].b32.s1, TEX_NULL, pool_size - pool_ptr);
        selector = old_setting;
        s = make_string();
        delete_token_ref(def_ref);
        def_ref = save_def_ref;
        warning_index = save_warning_index;
        scanner_status = save_scanner_status;
        b = pool_ptr;
        getmd5sum(s, boolvar); /* <== the difference-maker */
        mem[GARBAGE].b32.s1 = str_toks(b);

        if (s == str_ptr - 1) {
            str_ptr--;
            pool_ptr = str_start[str_ptr - TOO_BIG_CHAR];
        }

        begin_token_list(mem[TEMP_HEAD].b32.s1, INSERTED);
        if (u != 0)
            str_ptr--;
        return;
        break;

    case PDF_FILE_DUMP_CODE:
        save_scanner_status = scanner_status;
        save_warning_index = warning_index;
        save_def_ref = def_ref;

        if (str_start[str_ptr - TOO_BIG_CHAR] < pool_ptr)
            u = make_string();
        else
            u = 0;

        cur_val = 0;

        if (scan_keyword("offset")) {
            scan_int();

            if (cur_val < 0) {
                error_here_with_diagnostic("Bad file offset");
                capture_to_diagnostic(NULL);
                help_ptr = 2;
                help_line[1] = "A file offset must be between 0 and 2^_31_-1,";
                help_line[0] = "I changed this one to zero.";
                int_error(cur_val);
                cur_val = 0;
            }
        }

        i = cur_val;
        cur_val = 0;

        if (scan_keyword("length")) {
            scan_int();

            if (cur_val < 0) {
                error_here_with_diagnostic("Bad dump length");
                capture_to_diagnostic(NULL);
                help_ptr = 2;
                help_line[1] = "A dump length must be between 0 and 2^_31_-1,";
                help_line[0] = "I changed this one to zero.";
                int_error(cur_val);
                cur_val = 0;
            }
        }

        j = cur_val;

        scan_pdf_ext_toks();

        if (selector == SELECTOR_NEW_STRING)
            pdf_error("tokens", "tokens_to_string() called while selector = new_string");

        old_setting = selector;
        selector = SELECTOR_NEW_STRING;
        show_token_list(mem[def_ref].b32.s1, TEX_NULL, pool_size - pool_ptr);
        selector = old_setting;
        s = make_string();
        delete_token_ref(def_ref);
        def_ref = save_def_ref;
        warning_index = save_warning_index;
        scanner_status = save_scanner_status;
        b = pool_ptr;
        getfiledump(s, i, j); /* <=== non-boilerplate */
        mem[GARBAGE].b32.s1 = str_toks(b);

        if (s == str_ptr - 1) {
            str_ptr--;
            pool_ptr = str_start[str_ptr - TOO_BIG_CHAR];
        }

        begin_token_list(mem[TEMP_HEAD].b32.s1, INSERTED);
        if (u != 0)
            str_ptr--;
        return;

    case PDF_STRCMP_CODE:
        save_scanner_status = scanner_status;
        save_warning_index = warning_index;
        save_def_ref = def_ref;
        if (str_start[str_ptr - TOO_BIG_CHAR] < pool_ptr)
            u = make_string();
        else
            u = 0;
        compare_strings();
        def_ref = save_def_ref;
        warning_index = save_warning_index;
        scanner_status = save_scanner_status;
        if (u != 0)
            str_ptr--;
        break;

    case XETEX_UCHAR_CODE:
        scan_usv_num();
        break;

    case XETEX_UCHARCAT_CODE:
        scan_usv_num();
        saved_chr = cur_val;
        scan_int();

        if (cur_val < LEFT_BRACE || cur_val > ACTIVE_CHAR || cur_val == OUT_PARAM || cur_val == IGNORE) {
            error_here_with_diagnostic("Invalid code (");
            print_int(cur_val);
            print_cstr("), should be in the ranges 1..4, 6..8, 10..13");
            capture_to_diagnostic(NULL);

            help_ptr = 1;
            help_line[0] = "I'm going to use 12 instead of that illegal code value.";
            error();
            cat = 12;
        } else {
            cat = cur_val;
        }

        cur_val = saved_chr;
        break;

    case XETEX_REVISION_CODE:
        break;

    case XETEX_VARIATION_NAME_CODE:
        scan_font_ident();
        fnt = cur_val;
        if (font_area[fnt] == AAT_FONT_FLAG) {
            scan_int();
            arg1 = cur_val;
            arg2 = 0;
        } else {
            not_aat_font_error(CONVERT, c, fnt);
        }
        break;

    case XETEX_FEATURE_NAME_CODE:
        scan_font_ident();
        fnt = cur_val;
        if (font_area[fnt] == AAT_FONT_FLAG ||
            (font_area[fnt] == OTGR_FONT_FLAG && usingGraphite(font_layout_engine[fnt]))) {
            scan_int();
            arg1 = cur_val;
            arg2 = 0;
        } else {
            not_aat_gr_font_error(CONVERT, c, fnt);
        }
        break;

    case XETEX_SELECTOR_NAME_CODE:
        scan_font_ident();
        fnt = cur_val;
        if (font_area[fnt] == AAT_FONT_FLAG
            || (font_area[fnt] == OTGR_FONT_FLAG && usingGraphite(font_layout_engine[fnt]))) {
            scan_int();
            arg1 = cur_val;
            scan_int();
            arg2 = cur_val;
        } else {
            not_aat_gr_font_error(CONVERT, c, fnt);
        }
        break;

    case XETEX_GLYPH_NAME_CODE:
        scan_font_ident();
        fnt = cur_val;
        if (font_area[fnt] == AAT_FONT_FLAG || font_area[fnt] == OTGR_FONT_FLAG) {
            scan_int();
            arg1 = cur_val;
        } else {
            not_native_font_error(CONVERT, c, fnt);
        }
        break;

    case JOB_NAME_CODE:
        if (job_name == 0)
            open_log_file();
        break;

    case UNIFORM_DEVIATE_CODE:
        scan_int();
        break;

    case NORMAL_DEVIATE_CODE:
        break;
    }

    old_setting = selector;
    selector = SELECTOR_NEW_STRING;
    b = pool_ptr;

    switch (c) {
    case NUMBER_CODE:
        print_int(cur_val);
        break;

    case ROMAN_NUMERAL_CODE:
        print_roman_int(cur_val);
        break;

    case STRING_CODE:
        if (cur_cs != 0)
            sprint_cs(cur_cs);
        else
            print_char(cur_chr);
        break;

    case MEANING_CODE:
        print_meaning();
        break;

    case FONT_NAME_CODE:
        font_name_str = font_name[cur_val];

        if (font_area[cur_val] == AAT_FONT_FLAG || font_area[cur_val] == OTGR_FONT_FLAG) {
            quote_char = '"' ;

            for (i = 0; i <= length(font_name_str) - 1; i++)
                if (str_pool[str_start[(font_name_str) - 65536L] + i] == '"' )
                    quote_char = '\'' ;

            print_char(quote_char);
            print(font_name_str);
            print_char(quote_char);
        } else {
            print(font_name_str);
        }

        if (font_size[cur_val] != font_dsize[cur_val]) {
            print_cstr(" at ");
            print_scaled(font_size[cur_val]);
            print_cstr("pt");
        }
        break;

    case ETEX_REVISION_CODE:
        print_cstr(".6");
        break;

    case LEFT_MARGIN_KERN_CODE:
        p = mem[p + 5].b32.s1;
        while (p != TEX_NULL &&
               ((p < hi_mem_min
                 && (NODE_type(p) == INS_NODE ||
                     NODE_type(p) == MARK_NODE ||
                     NODE_type(p) == ADJUST_NODE ||
                     NODE_type(p) == PENALTY_NODE ||
                     (NODE_type(p) == DISC_NODE &&
                      mem[p + 1].b32.s0 == TEX_NULL &&
                      mem[p + 1].b32.s1 == TEX_NULL &&
                      mem[p].b16.s0 == 0) ||
                     (NODE_type(p) == MATH_NODE &&
                      mem[p + 1].b32.s1 == 0) ||
                     (NODE_type(p) == KERN_NODE &&
                      (mem[p + 1].b32.s1 == 0 || mem[p].b16.s0 == NORMAL)) ||
                     (NODE_type(p) == GLUE_NODE &&
                      mem[p + 1].b32.s0 == 0) ||
                     (NODE_type(p) == HLIST_NODE &&
                      mem[p + 1].b32.s1 == 0 &&
                      mem[p + 3].b32.s1 == 0 &&
                      mem[p + 2].b32.s1 == 0 &&
                      mem[p + 5].b32.s1 == TEX_NULL)
                     )) ||
                (p < hi_mem_min && NODE_type(p) == GLUE_NODE && mem[p].b16.s0 == (GLUE_PAR__left_skip + 1))))
            p = LLIST_link(p);

        if (p != TEX_NULL && p < hi_mem_min && NODE_type(p) == MARGIN_KERN_NODE && mem[p].b16.s0 == 0)
            print_scaled(mem[p + 1].b32.s1);
        else
            print('0');
        print_cstr("pt");
        break;

    case RIGHT_MARGIN_KERN_CODE:
        q = mem[p + 5].b32.s1;
        p = prev_rightmost(q, TEX_NULL);
        while (p != TEX_NULL &&
               ((p < hi_mem_min &&
                 (NODE_type(p) == INS_NODE ||
                  NODE_type(p) == MARK_NODE ||
                  NODE_type(p) == ADJUST_NODE ||
                  NODE_type(p) == PENALTY_NODE ||
                  (NODE_type(p) == DISC_NODE &&
                   mem[p + 1].b32.s0 == TEX_NULL &&
                   mem[p + 1].b32.s1 == TEX_NULL &&
                   mem[p].b16.s0 == 0) ||
                  (NODE_type(p) == MATH_NODE &&
                   mem[p + 1].b32.s1 == 0) ||
                  (NODE_type(p) == KERN_NODE &&
                   (mem[p + 1].b32.s1 == 0 || mem[p].b16.s0 == NORMAL)) ||
                  (NODE_type(p) == GLUE_NODE &&
                   mem[p + 1].b32.s0 == 0) ||
                  (NODE_type(p) == HLIST_NODE &&
                   mem[p + 1].b32.s1 == 0 &&
                   mem[p + 3].b32.s1 == 0 &&
                   mem[p + 2].b32.s1 == 0 &&
                   mem[p + 5].b32.s1 == TEX_NULL)
                  )) ||
                (p < hi_mem_min && NODE_type(p) == GLUE_NODE && mem[p].b16.s0 == (GLUE_PAR__right_skip + 1))))
            p = prev_rightmost(q, p);

        if (p != TEX_NULL && p < hi_mem_min && NODE_type(p) == MARGIN_KERN_NODE && mem[p].b16.s0 == 1)
            print_scaled(mem[p + 1].b32.s1);
        else
            print('0');
        print_cstr("pt");
        break;

    case PDF_STRCMP_CODE:
        print_int(cur_val);
        break;

    case UNIFORM_DEVIATE_CODE:
        print_int(unif_rand(cur_val));
        break;

    case NORMAL_DEVIATE_CODE:
        print_int(norm_rand());
        break;

    case XETEX_UCHAR_CODE:
    case XETEX_UCHARCAT_CODE:
        print_char(cur_val);
        break;

    case XETEX_REVISION_CODE:
        print_cstr(".999994");
        break;

    case XETEX_VARIATION_NAME_CODE:
        if (font_area[fnt] == AAT_FONT_FLAG)
            aat_print_font_name(c, font_layout_engine[fnt], arg1, arg2);
        break;

    case XETEX_FEATURE_NAME_CODE:
    case XETEX_SELECTOR_NAME_CODE:
        if (font_area[fnt] == AAT_FONT_FLAG)
            aat_print_font_name(c, font_layout_engine[fnt], arg1, arg2);
        else if (font_area[fnt] == OTGR_FONT_FLAG && usingGraphite(font_layout_engine[fnt]))
            gr_print_font_name(c, font_layout_engine[fnt], arg1, arg2);
        break;

    case XETEX_GLYPH_NAME_CODE:
        if (font_area[fnt] == AAT_FONT_FLAG || font_area[fnt] == OTGR_FONT_FLAG)
            print_glyph_name(fnt, arg1);
        break;

    case JOB_NAME_CODE:
        print_file_name(job_name, 0, 0);
        break;
    }

    selector = old_setting;
    mem[GARBAGE].b32.s1 = str_toks_cat(b, cat);
    begin_token_list(mem[TEMP_HEAD].b32.s1, INSERTED);
}
#endif


/* scan_toks: translated to Rust in src/scan.rs */
#if 0
int32_t scan_toks(bool macro_def, bool xpand)
{
    int32_t t;
    int32_t s;
    int32_t p;
    int32_t q;
    int32_t unbalance;
    int32_t hash_brace;
    if (macro_def)
        scanner_status = DEFINING;
    else
        scanner_status = ABSORBING;
    warning_index = cur_cs;
    def_ref = get_avail();
    mem[def_ref].b32.s0 = TEX_NULL;
    p = def_ref;
    hash_brace = 0;
    t = ZERO_TOKEN;
    if (macro_def) {            /*493: */
        while (true) {

            get_token();
            if (cur_tok < RIGHT_BRACE_LIMIT)
                goto done1;
            if (cur_cmd == MAC_PARAM) { /*495: */
                s = MATCH_TOKEN + cur_chr;
                get_token();
                if (cur_tok < LEFT_BRACE_LIMIT) {
                    hash_brace = cur_tok;
                    {
                        q = get_avail();
                        mem[p].b32.s1 = q;
                        mem[q].b32.s0 = cur_tok;
                        p = q;
                    }
                    {
                        q = get_avail();
                        mem[p].b32.s1 = q;
                        mem[q].b32.s0 = END_MATCH_TOKEN;
                        p = q;
                    }
                    goto done;
                }
                if (t == (ZERO_TOKEN + 9)) {
                    error_here_with_diagnostic("You already have nine parameters");
                    capture_to_diagnostic(NULL);
                    {
                        help_ptr = 2;
                        help_line[1] = "I'm going to ignore the # sign you just used,";
                        help_line[0] = "as well as the token that followed it.";
                    }
                    error();
                    continue;
                } else {

                    t++;
                    if (cur_tok != t) {
                        error_here_with_diagnostic("Parameters must be numbered consecutively");
                        {
                            help_ptr = 2;
                            help_line[1] = "I've inserted the digit you should have used after the #.";
                            help_line[0] = "Type `1' to delete what you did use.";
                        }
                        back_error();
                    }
                    cur_tok = s;
                }
            }
            {
                q = get_avail();
                mem[p].b32.s1 = q;
                mem[q].b32.s0 = cur_tok;
                p = q;
            }
        }
    done1:
        {
            q = get_avail();
            mem[p].b32.s1 = q;
            mem[q].b32.s0 = END_MATCH_TOKEN;
            p = q;
        }
        if (cur_cmd == RIGHT_BRACE) {   /*494: */
            error_here_with_diagnostic("Missing { inserted");
            capture_to_diagnostic(NULL);

            align_state++;
            help_ptr = 2;
            help_line[1] = "Where was the left brace? You said something like `\\def\\a}',";
            help_line[0] = "which I'm going to interpret as `\\def\\a{}'.";
            error();
            goto found;
        }
    done:
        ;
    } else
        scan_left_brace();
    unbalance = 1;
    while (true) {

        if (xpand) {            /*497: */
            while (true) {

                get_next();
                if (cur_cmd >= CALL) {

                    if (mem[mem[cur_chr].b32.s1].b32.s0 == PROTECTED_TOKEN) {
                        cur_cmd = RELAX;
                        cur_chr = NO_EXPAND_FLAG;
                    }
                }
                if (cur_cmd <= MAX_COMMAND)
                    goto done2;
                if (cur_cmd != THE)
                    expand();
                else {

                    q = the_toks();
                    if (mem[TEMP_HEAD].b32.s1 != TEX_NULL) {
                        mem[p].b32.s1 = mem[TEMP_HEAD].b32.s1;
                        p = q;
                    }
                }
            }
        done2:
            x_token();
        } else
            get_token();
        if (cur_tok < RIGHT_BRACE_LIMIT) {

            if (cur_cmd < RIGHT_BRACE)
                unbalance++;
            else {

                unbalance--;
                if (unbalance == 0)
                    goto found;
            }
        } else if (cur_cmd == MAC_PARAM) {

            if (macro_def) {    /*498: */
                s = cur_tok;
                if (xpand)
                    get_x_token();
                else
                    get_token();
                if (cur_cmd != MAC_PARAM) {

                    if ((cur_tok <= ZERO_TOKEN) || (cur_tok > t)) {
                        error_here_with_diagnostic("Illegal parameter number in definition of ");
                        sprint_cs(warning_index);
                        capture_to_diagnostic(NULL);
                        {
                            help_ptr = 3;
                            help_line[2] = "You meant to type ## instead of #, right?";
                            help_line[1] = "Or maybe a } was forgotten somewhere earlier, and things";
                            help_line[0] = "are all screwed up? I'm going to assume that you meant ##.";
                        }
                        back_error();
                        cur_tok = s;
                    } else
                        cur_tok = (OUT_PARAM_TOKEN - 48) + cur_chr;
                }
            }
        }
        {
            q = get_avail();
            mem[p].b32.s1 = q;
            mem[q].b32.s0 = cur_tok;
            p = q;
        }
    }
found:
    scanner_status = NORMAL;
    if (hash_brace != 0) {
        q = get_avail();
        mem[p].b32.s1 = q;
        mem[q].b32.s0 = hash_brace;
        p = q;
    }
    return p;
}
#endif


/* read_toks: translated to Rust in src/scan.rs */
#if 0
void
read_toks(int32_t n, int32_t r, int32_t j)
{
    int32_t p;
    int32_t q;
    int32_t s;
    small_number m;

    scanner_status = DEFINING;
    warning_index = r;
    def_ref = get_avail();
    mem[def_ref].b32.s0 = TEX_NULL;
    p = def_ref;

    q = get_avail();
    mem[p].b32.s1 = q;
    mem[q].b32.s0 = END_MATCH_TOKEN;
    p = q;

    if (n < 0 || n > 15)
        m = 16;
    else
        m = n;

    s = align_state;
    align_state = 1000000L;

    do { /*502:*/
        begin_file_reading();
        cur_input.name = m + 1;

        if (read_open[m] == CLOSED) { /*503:*/
            _tt_abort ("terminal input forbidden");
        } else if (read_open[m] == JUST_OPEN) { /*504:*/
            if (input_line(read_file[m])) {
                read_open[m] = NORMAL;
            } else {
                u_close(read_file[m]);
                read_open[m] = CLOSED;
            }
        } else { /*505:*/
            if (!input_line(read_file[m])) {
                u_close(read_file[m]);
                read_open[m] = CLOSED;
                if (align_state != 1000000L) {
                    runaway();
                    error_here_with_diagnostic("File ended within ");
                    print_esc_cstr("read");
                    capture_to_diagnostic(NULL);
                    help_ptr = 1;
                    help_line[0] = "This \\read has unbalanced braces.";
                    align_state = 1000000L;
                    cur_input.limit = 0;
                    error();
                }
            }
        }

        cur_input.limit = last;

        if (INTPAR(end_line_char) < 0 || INTPAR(end_line_char) > 255)
            cur_input.limit--;
        else
            buffer[cur_input.limit] = INTPAR(end_line_char);

        first = cur_input.limit + 1;
        cur_input.loc = cur_input.start;
        cur_input.state = NEW_LINE;

        if (j == 1) {
            while (cur_input.loc <= cur_input.limit) {
                cur_chr = buffer[cur_input.loc];
                cur_input.loc++;
                if (cur_chr == ' ' )
                    cur_tok = SPACE_TOKEN;
                else
                    cur_tok = cur_chr + OTHER_TOKEN;

                q = get_avail();
                mem[p].b32.s1 = q;
                mem[q].b32.s0 = cur_tok;
                p = q;
            }
            goto done;
        }

        while (true) {
            get_token();
            if (cur_tok == 0)
                goto done;

            if (align_state < 1000000L) {
                do {
                    get_token();
                } while (cur_tok != 0);
                align_state = 1000000L;
                goto done;
            }

            q = get_avail();
            mem[p].b32.s1 = q;
            mem[q].b32.s0 = cur_tok;
            p = q;
        }

    done:
        end_file_reading();
    } while (align_state != 1000000L);

    cur_val = def_ref;
    scanner_status = NORMAL;
    align_state = s;
}
#endif


/* pass_text: translated to Rust in src/scan.rs */
#if 0
void pass_text(void)
{
    int32_t l;
    small_number save_scanner_status;

    save_scanner_status = scanner_status;
    scanner_status = SKIPPING;
    l = 0;
    skip_line = line;

    while (true) {

        get_next();
        if (cur_cmd == FI_OR_ELSE) {
            if (l == 0)
                goto done;
            if (cur_chr == FI_CODE)
                l--;
        } else if (cur_cmd == IF_TEST)
            l++;
    }
done:
    scanner_status = save_scanner_status;
    if (INTPAR(tracing_ifs) > 0)
        show_cur_cmd_chr();
}
#endif

/* change_if_limit: translated to Rust in src/output.rs */


/* conditional: translated to Rust in src/scan.rs */


/* begin_name: translated to Rust in src/scan.rs */
#if 0
void
begin_name(void)
{
    area_delimiter = 0;
    ext_delimiter = 0;
    quoted_filename = false;
    file_name_quote_char = 0;
}
#endif


/* more_name: translated to Rust in src/scan.rs */
#if 0
bool
more_name(UTF16_code c)
{
    if (stop_at_space && file_name_quote_char == 0 && c == ' ' )
        return false;

    if (stop_at_space && file_name_quote_char != 0 && c == file_name_quote_char) {
        file_name_quote_char = 0;
        return true;
    }

    if (stop_at_space && file_name_quote_char == 0 && (c == '"'  || c == '\'' )) {
        file_name_quote_char = c;
        quoted_filename = true;
        return true;
    }

    if (pool_ptr + 1 > pool_size)
        overflow("pool size", pool_size - init_pool_ptr);

    str_pool[pool_ptr++] = c;

    if (IS_DIR_SEP(c)) {
        area_delimiter = cur_length();
        ext_delimiter = 0;
    } else if (c == '.' ) {
        ext_delimiter = cur_length();
    }

    return true;
}
#endif


/* end_name: translated to Rust in src/scan.rs */
#if 0
void
end_name(void)
{
    str_number temp_str;
    pool_pointer j;

    if (str_ptr + 3 > max_strings)
        overflow("number of strings", max_strings - init_str_ptr);

    /* area_delimiter is the length from the start of the filename to the
     * directory seperator "/", which we use to construct the stringpool
     * string `cur_area`. If there was already a string in the stringpool for
     * the area, reuse it. */

    if (area_delimiter == 0) {
        cur_area = EMPTY_STRING;
    } else {
        cur_area = str_ptr;
        str_start[(str_ptr + 1) - 65536L] = str_start[str_ptr - TOO_BIG_CHAR] + area_delimiter;
        str_ptr++;
        temp_str = search_string(cur_area);

        if (temp_str > 0) {
            cur_area = temp_str;
            str_ptr--;

            for (j = str_start[(str_ptr + 1) - 65536L]; j <= pool_ptr - 1; j++)
                str_pool[j - area_delimiter] = str_pool[j];

            pool_ptr = pool_ptr - area_delimiter;
        }
    }

    /* ext_delimiter is the length from the start of the filename to the
     * extension '.' delimiter, which we use to construct the stringpool
     * strings `cur_ext` and `cur_name`. */

    if (ext_delimiter == 0) {
        cur_ext = EMPTY_STRING;
        cur_name = slow_make_string();
    } else {
        cur_name = str_ptr;
        str_start[(str_ptr + 1) - 65536L] = str_start[str_ptr - TOO_BIG_CHAR] + ext_delimiter - area_delimiter - 1;
        str_ptr++;

        cur_ext = make_string();
        str_ptr--;
        temp_str = search_string(cur_name);

        if (temp_str > 0) {
            cur_name = temp_str;
            str_ptr--;

            for (j = str_start[(str_ptr + 1) - 65536L]; j <= pool_ptr - 1; j++)
                str_pool[j - ext_delimiter + area_delimiter + 1] = str_pool[j];

            pool_ptr = pool_ptr - ext_delimiter + area_delimiter + 1;
        }

        cur_ext = slow_make_string();
    }
}
#endif


/* pack_file_name: translated to Rust in src/scan.rs */


/* make_name_string: translated to Rust in src/scan.rs */


/* scan_file_name_braced + scan_file_name: translated to Rust in src/scan.rs */


/* pack_job_name: translated to Rust in src/scan.rs */


/* open_log_file: translated to Rust in src/scan.rs */


/* start_input: translated to Rust in src/scan.rs */


/* effective_char_info: translated to Rust in src/scan.rs */


/* char_warning: translated to Rust in src/scan.rs */


int32_t
new_native_word_node(internal_font_number f, int32_t n)
{
    int32_t l;
    int32_t q;

    l = NATIVE_NODE_SIZE + (n * sizeof(UTF16_code) + sizeof(memory_word) - 1) / sizeof(memory_word);
    q = get_node(l);
    NODE_type(q) = WHATSIT_NODE;

    if (INTPAR(xetex_generate_actual_text) > 0)
        NODE_subtype(q) = NATIVE_WORD_NODE_AT;
    else
        NODE_subtype(q) = NATIVE_WORD_NODE;

    NATIVE_NODE_size(q) = l;
    NATIVE_NODE_font(q) = f;
    NATIVE_NODE_length(q) = n;
    NATIVE_NODE_glyph_count(q) = 0;
    NATIVE_NODE_glyph_info_ptr(q) = NULL;
    return q;
}


int32_t
new_native_character(internal_font_number f, UnicodeScalar c)
{
    int32_t p;
    int32_t i, len;

    if (font_mapping[f] != NULL) {
        if (c > 65535L) {
            if (pool_ptr + 2 > pool_size)
                overflow("pool size", pool_size - init_pool_ptr);

            str_pool[pool_ptr] = (c - 65536L) / 1024 + 0xD800;
            pool_ptr++;
            str_pool[pool_ptr] = (c - 65536L) % 1024 + 0xDC00;
            pool_ptr++;
        } else {
            if (pool_ptr + 1 > pool_size)
                overflow("pool size", pool_size - init_pool_ptr);

            str_pool[pool_ptr] = c;
            pool_ptr++;
        }

        len = apply_mapping(
            font_mapping[f],
            &str_pool[str_start[str_ptr - TOO_BIG_CHAR]],
            cur_length()
        );
        pool_ptr = str_start[str_ptr - TOO_BIG_CHAR];

        i = 0;

        while (i < len) {
            if (mapped_text[i] >= 0xD800 && mapped_text[i] < 0xDC00) {
                c = (mapped_text[i] - 0xD800) * 1024 + mapped_text[i + 1] + 9216;
                if (map_char_to_glyph(f, c) == 0)
                    char_warning(f, c);

                i += 2;
            } else {
                if (map_char_to_glyph(f, mapped_text[i]) == 0)
                    char_warning(f, mapped_text[i]);

                i += 1;
            }
        }

        p = new_native_word_node(f, len);

        for (i = 0; i <= len - 1; i++)
            NATIVE_NODE_text(p)[i] = mapped_text[i];
    } else {
        if (INTPAR(tracing_lost_chars) > 0)
            if (map_char_to_glyph(f, c) == 0)
                char_warning(f, c);

        p = get_node(NATIVE_NODE_SIZE + 1);
        NODE_type(p) = WHATSIT_NODE;
        NODE_subtype(p) = NATIVE_WORD_NODE;
        NATIVE_NODE_size(p) = NATIVE_NODE_SIZE + 1;
        NATIVE_NODE_glyph_count(p) = 0;
        NATIVE_NODE_glyph_info_ptr(p) = NULL;
        NATIVE_NODE_font(p) = f;

        if (c > 65535L) {
            NATIVE_NODE_length(p) = 2;
            NATIVE_NODE_text(p)[0] = (c - 65536L) / 1024 + 0xD800;
            NATIVE_NODE_text(p)[1] = (c - 65536L) % 1024 + 0xDC00;
        } else {
            NATIVE_NODE_length(p) = 1;
            NATIVE_NODE_text(p)[0] = c;
        }
    }

    set_native_metrics(p, INTPAR(xetex_use_glyph_metrics) > 0);
    return p;
}


/* font_feature_warning: translated to Rust in src/output.rs */
/* font_mapping_warning: translated to Rust in src/output.rs */
/* graphite_warning: translated to Rust in src/output.rs */


internal_font_number
load_native_font(int32_t u, str_number nom, str_number aire, scaled_t s)
{
    int32_t k, num_font_dimens;
    void *font_engine; /* "really a CFDictionaryRef or XeTeXLayoutEngine" */
    scaled_t actual_size;
    int32_t p;
    scaled_t ascent, descent, font_slant, x_ht, cap_ht;
    internal_font_number f;
    str_number full_name;

    font_engine = find_native_font(name_of_file, s);
    if (!font_engine)
        return FONT_BASE;

    if (s >= 0)
        actual_size = s;
    else if (s != -1000)
        actual_size = xn_over_d(get_loaded_font_design_size(), -(int32_t) s, 1000);
    else
        actual_size = get_loaded_font_design_size();

    if (pool_ptr + name_length > pool_size)
        overflow("pool size", pool_size - init_pool_ptr);

    for (k = 0; k < name_length; k++)
        str_pool[pool_ptr++] = name_of_file[k];

    full_name = make_string();

    for (f = FONT_BASE + 1; f <= font_ptr; f++) {
        if (font_area[f] == native_font_type_flag && str_eq_str(font_name[f], full_name) && font_size[f] == actual_size) {
            release_font_engine(font_engine, native_font_type_flag);

            str_ptr--;
            pool_ptr = str_start[str_ptr - TOO_BIG_CHAR];

            return f;
        }
    }

    if (native_font_type_flag == OTGR_FONT_FLAG && isOpenTypeMathFont(font_engine))
        num_font_dimens = 65; /* = first_math_fontdimen (=10) + lastMathConstant (= radicalDegreeBottomRaisePercent = 55) */
    else
        num_font_dimens = 8;

    if (font_ptr == font_max || fmem_ptr + num_font_dimens > font_mem_size) {
        error_here_with_diagnostic("Font ");
        sprint_cs(u);
        print_char('=');
        if (file_name_quote_char != 0)
            print_char(file_name_quote_char);
        print_file_name(nom, aire, cur_ext);
        if (file_name_quote_char != 0)
            print_char(file_name_quote_char);
        if (s >= 0) {
            print_cstr(" at ");
            print_scaled(s);
            print_cstr("pt");
        } else if (s != -1000) {
            print_cstr(" scaled ");
            print_int(-(int32_t) s);
        }
        print_cstr(" not loaded: Not enough room left");
        capture_to_diagnostic(NULL);

        help_ptr = 4;
        help_line[3] = "I'm afraid I won't be able to make use of this font,";
        help_line[2] = "because my memory for character-size data is too small.";
        help_line[1] = "If you're really stuck, ask a wizard to enlarge me.";
        help_line[0] = "Or maybe try `I\\font<same font id>=<name of loaded font>'.";

        error();
        return FONT_BASE;
    }

    font_ptr++;
    font_area[font_ptr] = native_font_type_flag;
    font_name[font_ptr] = full_name;
    font_check[font_ptr].s3 = 0;
    font_check[font_ptr].s2 = 0;
    font_check[font_ptr].s1 = 0;
    font_check[font_ptr].s0 = 0;
    font_glue[font_ptr] = TEX_NULL;
    font_dsize[font_ptr] = get_loaded_font_design_size();
    font_size[font_ptr] = actual_size;

    if (native_font_type_flag == AAT_FONT_FLAG)
        aat_get_font_metrics(font_engine, &ascent, &descent, &x_ht, &cap_ht, &font_slant);
    else
        ot_get_font_metrics(font_engine, &ascent, &descent, &x_ht, &cap_ht, &font_slant);

    height_base[font_ptr] = ascent;
    depth_base[font_ptr] = -(int32_t) descent;
    font_params[font_ptr] = num_font_dimens;
    font_bc[font_ptr] = 0;
    font_ec[font_ptr] = 65535L;
    font_used[font_ptr] = false;
    hyphen_char[font_ptr] = INTPAR(default_hyphen_char);
    skew_char[font_ptr] = INTPAR(default_skew_char);
    param_base[font_ptr] = fmem_ptr - 1;
    font_layout_engine[font_ptr] = font_engine;
    font_mapping[font_ptr] = 0;
    font_letter_space[font_ptr] = loaded_font_letter_space;

    /* "measure the width of the space character and set up font parameters" */
    p = new_native_character(font_ptr, ' ' );
    s = BOX_width(p) + loaded_font_letter_space;
    /* Free up the memory */
    if(NATIVE_NODE_glyph_info_ptr(p)) {
       NATIVE_NODE_glyph_info_ptr(p) = mfree(NATIVE_NODE_glyph_info_ptr(p));
    }
    free_node(p, NATIVE_NODE_size(p));

    font_info[fmem_ptr++].b32.s1 = font_slant;
    font_info[fmem_ptr++].b32.s1 = s;
    font_info[fmem_ptr++].b32.s1 = s / 2; /* space_stretch */
    font_info[fmem_ptr++].b32.s1 = s / 3; /* space_shrink */
    font_info[fmem_ptr++].b32.s1 = x_ht;
    font_info[fmem_ptr++].b32.s1 = font_size[font_ptr]; /* quad */
    font_info[fmem_ptr++].b32.s1 = s / 3; /* extra_space */
    font_info[fmem_ptr++].b32.s1 = cap_ht;

    if (num_font_dimens == 65) {
        font_info[fmem_ptr++].b32.s1 = num_font_dimens;

        for (k = 0; k <= 55; k++) /* 55 = lastMathConstant */
            font_info[fmem_ptr++].b32.s1 = get_ot_math_constant(font_ptr, k);
    }

    font_mapping[font_ptr] = loaded_font_mapping;
    font_flags[font_ptr] = loaded_font_flags;
    return font_ptr;
}


void do_locale_linebreaks(int32_t s, int32_t len)
{
    int32_t offs, prevOffs, i;
    bool use_penalty, use_skip;

    if ((INTPAR(xetex_linebreak_locale) == 0) || (len == 1)) {
        mem[cur_list.tail].b32.s1 = new_native_word_node(main_f, len);
        cur_list.tail = LLIST_link(cur_list.tail);
        {
            register int32_t for_end;
            i = 0;
            for_end = len - 1;
            if (i <= for_end)
                do
                    NATIVE_NODE_text(cur_list.tail)[i] = native_text[s + i];
                while (i++ < for_end);
        }
        set_native_metrics(cur_list.tail, (INTPAR(xetex_use_glyph_metrics) > 0));
    } else {

        use_skip = GLUEPAR(xetex_linebreak_skip) != 0;
        use_penalty = INTPAR(xetex_linebreak_penalty) != 0 || !use_skip;
        linebreak_start(main_f, INTPAR(xetex_linebreak_locale), native_text + s, len);
        offs = 0;
        do {
            prevOffs = offs;
            offs = linebreak_next(main_f);
            if (offs > 0) {
                if (prevOffs != 0) {
                    if (use_penalty) {
                        mem[cur_list.tail].b32.s1 = new_penalty(INTPAR(xetex_linebreak_penalty));
                        cur_list.tail = LLIST_link(cur_list.tail);
                    }
                    if (use_skip) {
                        mem[cur_list.tail].b32.s1 = new_param_glue(GLUE_PAR__xetex_linebreak_skip);
                        cur_list.tail = LLIST_link(cur_list.tail);
                    }
                }
                mem[cur_list.tail].b32.s1 = new_native_word_node(main_f, offs - prevOffs);
                cur_list.tail = LLIST_link(cur_list.tail);
                {
                    register int32_t for_end;
                    i = prevOffs;
                    for_end = offs - 1;
                    if (i <= for_end)
                        do
                            NATIVE_NODE_text(cur_list.tail)[i - prevOffs] = native_text[s + i];
                        while (i++ < for_end);
                }
                set_native_metrics(cur_list.tail, (INTPAR(xetex_use_glyph_metrics) > 0));
            }
        } while (!(offs < 0));
    }
}

/* bad_utf8_warning: translated to Rust in src/output.rs */

/* get_input_normalization_state: translated to Rust in src/scan.rs */
/* get_tracing_fonts_state: translated to Rust in src/scan.rs */

internal_font_number
read_font_info(int32_t u, str_number nom, str_number aire, scaled_t s)
{
    font_index k;
    bool name_too_long;
    bool file_opened;
    int32_t lf, lh, bc, ec, nw, nh, nd, ni, nl, nk, ne, np;
    internal_font_number f;
    internal_font_number g;
    int a, b, c, d;
    b16x4 qw;
    scaled_t sw;
    int32_t bch_label;
    short bchar;
    scaled_t z;
    int32_t alpha;
    unsigned char beta;
    rust_input_handle_t tfm_file;

    g = FONT_BASE;

    file_opened = false;
    pack_file_name(nom, aire, cur_ext);

    if (INTPAR(xetex_tracing_fonts) > 0) {
        begin_diagnostic();
        diagnostic_begin_capture_warning_here();
        print_nl_cstr("Requested font \"");
        print_c_string(name_of_file);
        print('"');
        if (s < 0) {
            print_cstr(" scaled ");
            print_int(-(int32_t) s);
        } else {
            print_cstr(" at ");
            print_scaled(s);
            print_cstr("pt");
        }
        capture_to_diagnostic(NULL);
        end_diagnostic(false);
    }

    if (quoted_filename) {
        g = load_native_font(u, nom, aire, s);
        if (g != FONT_BASE)
            goto done;
    }

    name_too_long = (length(nom) > 255 || length(aire) > 255);
    if (name_too_long)
        goto bad_tfm;

    pack_file_name(nom, aire, EMPTY_STRING);
    check_for_tfm_font_mapping();

    tfm_file = tt_xetex_open_input (TTBC_FILE_FORMAT_TFM);
    if (tfm_file == INVALID_HANDLE) {
        if (!quoted_filename) {
            g = load_native_font(u, nom, aire, s);
            if (g != FONT_BASE)
                goto done;
        }
        goto bad_tfm;
    }

    file_opened = true; /*:582*/

    /* We are a bit cavalier about EOF-checking since we can't very
     * conveniently implement feof() in the Rust layer, and it only ever is
     * used in this one place. */

#define READFIFTEEN(x) do { \
        x = ttstub_input_getc (tfm_file); \
        if (x > 127 || x == EOF) \
            goto bad_tfm; \
        x *= 256; \
        x += ttstub_input_getc (tfm_file);\
    } while (0)

    READFIFTEEN(lf);
    READFIFTEEN(lh);
    READFIFTEEN(bc);
    READFIFTEEN(ec);

    if (bc > ec + 1 || ec > 255)
        goto bad_tfm;

    if (bc > 255) {
        bc = 1;
        ec = 0;
    }

    READFIFTEEN(nw);
    READFIFTEEN(nh);
    READFIFTEEN(nd);
    READFIFTEEN(ni);
    READFIFTEEN(nl);
    READFIFTEEN(nk);
    READFIFTEEN(ne);
    READFIFTEEN(np);

    if (lf != 6 + lh + (ec - bc + 1) + nw + nh + nd + ni + nl + nk + ne + np)
        goto bad_tfm;
    if (nw == 0 || nh == 0 || nd == 0 || ni == 0)
        goto bad_tfm;

    lf = lf - 6 - lh;
    if (np < 7)
        lf = lf + 7 - np;

    if (font_ptr == font_max || fmem_ptr + lf > font_mem_size)
        _tt_abort("not enough memory to load another font");

    f = font_ptr + 1;
    char_base[f] = fmem_ptr - bc;
    width_base[f] = char_base[f] + ec + 1;
    height_base[f] = width_base[f] + nw;
    depth_base[f] = height_base[f] + nh;
    italic_base[f] = depth_base[f] + nd;
    lig_kern_base[f] = italic_base[f] + ni;
    kern_base[f] = lig_kern_base[f] + nl - 256 * (128);
    exten_base[f] = kern_base[f] + 256 * (128) + nk;
    param_base[f] = exten_base[f] + /*:585 */ ne;

    if (lh < 2)
        goto bad_tfm;

    qw.s3 = a = ttstub_input_getc (tfm_file);
    qw.s2 = b = ttstub_input_getc (tfm_file);
    qw.s1 = c = ttstub_input_getc (tfm_file);
    qw.s0 = d = ttstub_input_getc (tfm_file);
    if (a == EOF || b == EOF || c == EOF || d == EOF)
        goto bad_tfm;
    font_check[f] = qw;

    READFIFTEEN(z);
    z = z * 256 + ttstub_input_getc (tfm_file);
    z = (z * 16) + (ttstub_input_getc (tfm_file) / 16);
    if (z < 65536L)
        goto bad_tfm;

    while (lh > 2) {
        ttstub_input_getc (tfm_file);
        ttstub_input_getc (tfm_file);
        ttstub_input_getc (tfm_file);
        ttstub_input_getc (tfm_file);
        lh--;
    }

    font_dsize[f] = z;
    if (s != -1000) {
        if (s >= 0)
            z = s;
        else
            z = xn_over_d(z, -(int32_t) s, 1000);
    }

    font_size[f] = z;

    for (k = fmem_ptr; k <= width_base[f] - 1; k++) {
        qw.s3 = a = ttstub_input_getc (tfm_file);
        qw.s2 = b = ttstub_input_getc (tfm_file);
        qw.s1 = c = ttstub_input_getc (tfm_file);
        qw.s0 = d = ttstub_input_getc (tfm_file);
        if (a == EOF || b == EOF || c == EOF || d == EOF)
            goto bad_tfm;
        font_info[k].b16 = qw;

        if (a >= nw || b / 16 >= nh || b % 16 >= nd || c / 4 >= ni)
            goto bad_tfm;

        switch (c % 4) {
        case 1:
            if (d >= nl)
                goto bad_tfm;
            break;
        case 3:
            if (d >= ne)
                goto bad_tfm;
            break;
        case 2:
            if (d < bc || d > ec)
                goto bad_tfm;

            while (d < k + bc - fmem_ptr) {
                qw = FONT_CHARACTER_INFO(f, d);
                if ((qw.s1 % 4) != LIST_TAG)
                    goto not_found;
                d = qw.s0;
            }

            if (d == k + bc - fmem_ptr)
                goto bad_tfm;

        not_found:
            break;
        }
    }

    alpha = 16;
    while (z >= 0x800000) {
        z = z / 2;
        alpha = alpha + alpha;
    }
    beta = 256 / alpha;
    alpha = alpha * z;

    for (k = width_base[f]; k <= lig_kern_base[f] - 1; k++) {
        a = ttstub_input_getc (tfm_file);
        b = ttstub_input_getc (tfm_file);
        c = ttstub_input_getc (tfm_file);
        d = ttstub_input_getc (tfm_file);
        if (a == EOF || b == EOF || c == EOF || d == EOF)
            goto bad_tfm;
        sw = (((((d * z) / 256) + c * z) / 256) + b * z) / beta;

        if (a == 0)
            font_info[k].b32.s1 = sw;
        else if (a == 255)
            font_info[k].b32.s1 = sw - alpha;
        else
            goto bad_tfm;
    }

    if (font_info[width_base[f]].b32.s1 != 0)
        goto bad_tfm;
    if (font_info[height_base[f]].b32.s1 != 0)
        goto bad_tfm;
    if (font_info[depth_base[f]].b32.s1 != 0)
        goto bad_tfm;
    if (font_info[italic_base[f]].b32.s1 != 0)
        goto bad_tfm;

    bch_label = 32767;
    bchar = 256;

    if (nl > 0) {
        for (k = lig_kern_base[f]; k <= kern_base[f] + 256 * 128 - 1; k++) {
            qw.s3 = a = ttstub_input_getc (tfm_file);
            qw.s2 = b = ttstub_input_getc (tfm_file);
            qw.s1 = c = ttstub_input_getc (tfm_file);
            qw.s0 = d = ttstub_input_getc (tfm_file);
            if (a == EOF || b == EOF || c == EOF || d == EOF)
                goto bad_tfm;
            font_info[k].b16 = qw;

            if (a > 128) {
                if (256 * c + d >= nl)
                    goto bad_tfm;

                if (a == 255 && k == lig_kern_base[f])
                    bchar = b;
            } else {
                if (b != bchar) {
                    if ((b < bc) || (b > ec))
                        goto bad_tfm;

                    qw = FONT_CHARACTER_INFO(f, b);
                    if (!(qw.s3 > 0))
                        goto bad_tfm;
                }

                if (c < 128) {
                    if ((d < bc) || (d > ec))
                        goto bad_tfm;
                    qw = FONT_CHARACTER_INFO(f, d);
                    if (!(qw.s3 > 0))
                        goto bad_tfm;
                } else if (256 * (c - 128) + d >= nk)
                    goto bad_tfm;

                if (a < 128 && k - lig_kern_base[f] + a + 1 >= nl)
                    goto bad_tfm;
            }
        }

        if (a == 255)
            bch_label = 256 * c + d;
    }

    for (k = kern_base[f] + 256 * 128; k <= exten_base[f] - 1; k++) {
        a = ttstub_input_getc (tfm_file);
        b = ttstub_input_getc (tfm_file);
        c = ttstub_input_getc (tfm_file);
        d = ttstub_input_getc (tfm_file);
        if (a == EOF || b == EOF || c == EOF || d == EOF)
            goto bad_tfm;
        sw = (((((d * z) / 256) + c * z) / 256) + b * z) / beta;

        if (a == 0)
            font_info[k].b32.s1 = sw;
        else if (a == 255)
            font_info[k].b32.s1 = sw - alpha;
        else
            goto bad_tfm;
    }

    for (k = exten_base[f]; k <= param_base[f] - 1; k++) {
        qw.s3 = a = ttstub_input_getc (tfm_file);
        qw.s2 = b = ttstub_input_getc (tfm_file);
        qw.s1 = c = ttstub_input_getc (tfm_file);
        qw.s0 = d = ttstub_input_getc (tfm_file);
        if (a == EOF || b == EOF || c == EOF || d == EOF)
            goto bad_tfm;
        font_info[k].b16 = qw;

        if (a != 0) {
            if ((a < bc) || (a > ec))
                goto bad_tfm;
            qw = FONT_CHARACTER_INFO(f, a);
            if (!(qw.s3 > 0))
                goto bad_tfm;
        }

        if (b != 0) {
            if ((b < bc) || (b > ec))
                goto bad_tfm;
            qw = FONT_CHARACTER_INFO(f, b);
            if (!(qw.s3 > 0))
                goto bad_tfm;
        }

        if (c != 0) {
            if ((c < bc) || (c > ec))
                goto bad_tfm;
            qw = FONT_CHARACTER_INFO(f, c);
            if (!(qw.s3 > 0))
                goto bad_tfm;
        }

        if ((d < bc) || (d > ec))
            goto bad_tfm;
        qw = FONT_CHARACTER_INFO(f, d);
        if (!(qw.s3 > 0))
            goto bad_tfm;
    }

    for (k = 1; k <= np; k++) {
        if (k == 1) {
            sw = ttstub_input_getc (tfm_file);
            if (sw == EOF)
                goto bad_tfm;
            if (sw > 127)
                sw = sw - 256;

            sw = sw * 256 + ttstub_input_getc (tfm_file);
            sw = sw * 256 + ttstub_input_getc (tfm_file);
            font_info[param_base[f]].b32.s1 = (sw * 16) + (ttstub_input_getc (tfm_file) / 16);
        } else {
            a = ttstub_input_getc (tfm_file);
            b = ttstub_input_getc (tfm_file);
            c = ttstub_input_getc (tfm_file);
            d = ttstub_input_getc (tfm_file);
            if (a == EOF || b == EOF || c == EOF || d == EOF)
                goto bad_tfm;
            sw = (((((d * z) / 256) + c * z) / 256) + b * z) / beta;

            if (a == 0)
                font_info[param_base[f] + k - 1].b32.s1 = sw;
            else if (a == 255)
                font_info[param_base[f] + k - 1].b32.s1 = sw - alpha;
            else
                goto bad_tfm;
        }
    }

    for (k = np + 1; k <= 7; k++)
        font_info[param_base[f] + k - 1].b32.s1 = 0;

    if (np >= 7)
        font_params[f] = np;
    else
        font_params[f] = 7;

    hyphen_char[f] = INTPAR(default_hyphen_char);
    skew_char[f] = INTPAR(default_skew_char);
    if (bch_label < nl)
        bchar_label[f] = bch_label + lig_kern_base[f];
    else
        bchar_label[f] = NON_ADDRESS;
    font_bchar[f] = bchar;
    font_false_bchar[f] = bchar;

    if (bchar <= ec) {
        if (bchar >= bc) {
            qw = FONT_CHARACTER_INFO(f, bchar);
            if ((qw.s3 > 0))
                font_false_bchar[f] = TOO_BIG_CHAR;
        }
    }

    font_name[f] = nom;
    font_area[f] = aire;
    font_bc[f] = bc;
    font_ec[f] = ec;
    font_glue[f] = TEX_NULL;
    param_base[f]--;
    fmem_ptr = fmem_ptr + lf;
    font_ptr = f;
    g = f;
    font_mapping[f] = load_tfm_font_mapping();
    goto done;

bad_tfm:
    if (INTPAR(suppress_fontnotfound_error) == 0) {
        /* NOTE: must preserve this path to keep passing the TRIP tests */

        error_here_with_diagnostic("Font ");
        sprint_cs(u);
        print_char('=');
        if (file_name_quote_char != 0)
            print_char(file_name_quote_char);
        print_file_name(nom, aire, cur_ext);
        if (file_name_quote_char != 0)
            print_char(file_name_quote_char);
        if (s >= 0) {
            print_cstr(" at ");
            print_scaled(s);
            print_cstr("pt");
        } else if (s != -1000) {
            print_cstr(" scaled ");
            print_int(-(int32_t) s);
        }

        if (file_opened)
            print_cstr(" not loadable: Bad metric (TFM) file");
        else if (name_too_long)
            print_cstr(" not loadable: Metric (TFM) file name too long");
        else
            print_cstr(" not loadable: Metric (TFM) file or installed font not found");
        capture_to_diagnostic(NULL);

        help_ptr = 5;
        help_line[4] = "I wasn't able to read the size data for this font,";
        help_line[3] = "so I will ignore the font specification.";
        help_line[2] = "[Wizards can fix TFM files using TFtoPL/PLtoTF.]";
        help_line[1] = "You might try inserting a different font spec;";
        help_line[0] = "e.g., type `I\\font<same font id>=<substitute font name>'.";

        error();
    }

done:
    if (file_opened)
        ttstub_input_close (tfm_file);

    if (INTPAR(xetex_tracing_fonts) > 0) {
        if (g == FONT_BASE || file_opened) {
            begin_diagnostic();
            diagnostic_begin_capture_warning_here();

            print_nl_cstr(" -> ");
            if (g == FONT_BASE)
                print_c_string("font not found, using \"nullfont\"");
            else
                print_c_string(name_of_file);

            capture_to_diagnostic(NULL);
            end_diagnostic(false);
        }
    }

    return g;
}

/* new_character: translated to Rust in src/scan.rs */


/* scan_spec: translated to Rust in src/scan.rs */

/* char_pw: translated to Rust in src/scan.rs */

/* new_margin_kern: translated to Rust in src/mem.rs */

/* hpack: translated to Rust in src/scan.rs */

/* vpackage: translated to Rust in src/scan.rs */
/* append_to_vlist: translated to Rust in src/scan.rs */

/* new_noad: translated to Rust in src/mem.rs */
/* new_style: translated to Rust in src/mem.rs */
/* new_choice: translated to Rust in src/mem.rs */

/* show_info: translated to Rust in src/scan.rs */
#if 0
void show_info(void)
{

    show_node_list(mem[temp_ptr].b32.s0);
}
#endif

/* push_alignment: translated to Rust in src/scan.rs */

/* pop_alignment: translated to Rust in src/scan.rs */

/* get_preamble_token: translated to Rust in src/scan.rs */


/* init_align: translated to Rust in src/scan.rs */


/* init_span: translated to Rust in src/scan.rs */
/* init_row: translated to Rust in src/scan.rs */
/* init_col: translated to Rust in src/scan.rs */

/* fin_col: translated to Rust in src/scan.rs */

/* fin_row: translated to Rust in src/scan.rs */

/* fin_align: translated to Rust in src/scan.rs */

/* align_peek: translated to Rust in src/scan.rs */

/* max_hyphenatable_length: translated to Rust in src/scan.rs */

/* eTeX_enabled: translated to Rust in src/scan.rs */


/* show_save_groups: translated to Rust in src/scan.rs */


/* vert_break: translated to Rust in src/scan.rs */

/* vsplit: translated to Rust in src/scan.rs */

/* print_totals: translated to Rust in src/output.rs */

/* box_error: translated to Rust in src/scan.rs */


/* app_space: translated to Rust in src/scan.rs */

/* insert_dollar_sign: translated to Rust in src/scan.rs */

/* you_cant: translated to Rust in src/scan.rs */

/* report_illegal_case: translated to Rust in src/scan.rs */

/* privileged: translated to Rust in src/scan.rs */

/* its_all_over: translated to Rust in src/scan.rs */

/* append_glue: translated to Rust in src/scan.rs */

/* append_kern: translated to Rust in src/scan.rs */


/* off_save: translated to Rust in src/scan.rs */


/* extra_right_brace: translated to Rust in src/scan.rs */
#if 0
void
extra_right_brace(void)
{
    error_here_with_diagnostic("Extra }, or forgotten ");

    switch (cur_group) {
    case SEMI_SIMPLE_GROUP:
        print_esc_cstr("endgroup");
        break;
    case MATH_SHIFT_GROUP:
        print_char('$');
        break;
    case MATH_LEFT_GROUP:
        print_esc_cstr("right");
        break;
    }

    capture_to_diagnostic(NULL);

    help_ptr = 5;
    help_line[4] = "I've deleted a group-closing symbol because it seems to be";
    help_line[3] = "spurious, as in `$x}$'. But perhaps the } is legitimate and";
    help_line[2] = "you forgot something else, as in `\\hbox{$x}'. In such cases";
    help_line[1] = "the way to recover is to insert both the forgotten and the";
    help_line[0] = "deleted material, e.g., by typing `I$}'.";
    error();
    align_state++;
}
#endif


/* normal_paragraph: translated to Rust in src/scan.rs */


/* box_end: translated to Rust in src/scan.rs */


/* begin_box: translated to Rust in src/scan.rs */
#if 0
void
begin_box(int32_t box_context)
{
    int32_t p, q;
    int32_t r;
    bool fm;
    int32_t tx;
    uint16_t m;
    int32_t k;
    int32_t n;

    switch (cur_chr) {
    case BOX_CODE:
        scan_register_num();

        if (cur_val < 256) {
            cur_box = BOX_REG(cur_val);
        } else {
            find_sa_element(4, cur_val, false);
            if (cur_ptr == TEX_NULL)
                cur_box = TEX_NULL;
            else
                cur_box = mem[cur_ptr + 1].b32.s1;
        }

        if (cur_val < 256) {
            BOX_REG(cur_val) = TEX_NULL;
        } else {
            find_sa_element(4, cur_val, false);
            if (cur_ptr != TEX_NULL) {
                mem[cur_ptr + 1].b32.s1 = TEX_NULL;
                mem[cur_ptr + 1].b32.s0++;
                delete_sa_ref(cur_ptr);
            }
        }
        break;

    case COPY_CODE:
        scan_register_num();

        if (cur_val < 256) {
            q = BOX_REG(cur_val);
        } else {
            find_sa_element(4, cur_val, false);
            if (cur_ptr == TEX_NULL)
                q = TEX_NULL;
            else
                q = mem[cur_ptr + 1].b32.s1;
        }

        cur_box = copy_node_list(q);
        break;

    case LAST_BOX_CODE:
        cur_box = TEX_NULL;

        if (abs(cur_list.mode) == MMODE) {
            you_cant();
            help_ptr = 1;
            help_line[0] = "Sorry; this \\lastbox will be void.";
            error();
        } else if (cur_list.mode == VMODE && cur_list.head == cur_list.tail) {
            you_cant();
            help_ptr = 2;
            help_line[1] = "Sorry...I usually can't take things from the current page.";
            help_line[0] = "This \\lastbox will therefore be void.";
            error();
        } else {
            tx = cur_list.tail;

            if (tx < hi_mem_min) {
                if (NODE_type(tx) == MATH_NODE && mem[tx].b16.s0 == END_M_CODE) {
                    r = cur_list.head;
                    do {
                        q = r;
                        r = mem[q].b32.s1;
                    } while (r != tx);
                    tx = q;
                }
            }

            if (tx < hi_mem_min) {
                if (NODE_type(tx) == HLIST_NODE || NODE_type(tx) == VLIST_NODE) { /*1116:*/
                    q = cur_list.head;
                    p = TEX_NULL;

                    do {
                        r = p;
                        p = q;
                        fm = false;

                        if (q < hi_mem_min) {
                            if (NODE_type(q) == DISC_NODE) {
                                for (m = 1; m <= mem[q].b16.s0; m++)
                                    p = LLIST_link(p);

                                if (p == tx)
                                    goto done;
                            } else if (NODE_type(q) == MATH_NODE && mem[q].b16.s0 == BEGIN_M_CODE) {
                                fm = true;
                            }
                        }

                        q = mem[p].b32.s1;
                    } while (q != tx);

                    q = mem[tx].b32.s1;
                    mem[p].b32.s1 = q;
                    mem[tx].b32.s1 = TEX_NULL;

                    if (q == TEX_NULL) {
                        if (fm)
                            confusion("tail1");
                        else
                            cur_list.tail = p;
                    } else if (fm) {
                        cur_list.tail = r;
                        mem[r].b32.s1 = TEX_NULL;
                        flush_node_list(p);
                    }

                    cur_box = tx;
                    mem[cur_box + 4].b32.s1 = 0;
                }
            }
        done:
            ;
        }
        break;

    case VSPLIT_CODE:
        scan_register_num();
        n = cur_val;

        if (!scan_keyword("to")) {
            error_here_with_diagnostic("Missing `to' inserted");
            capture_to_diagnostic(NULL);

            help_ptr = 2;
            help_line[1] = "I'm working on `\\vsplit<box number> to <dimen>';";
            help_line[0] = "will look for the <dimen> next.";
            error();
        }

        scan_dimen(false, false, false);
        cur_box = vsplit(n, cur_val);
        break;

    default:
        k = cur_chr - 4;
        save_stack[save_ptr + 0].b32.s1 = box_context;
        if (k == HMODE) {
            if (box_context < BOX_FLAG && abs(cur_list.mode) == VMODE)
                scan_spec(ADJUSTED_HBOX_GROUP, true);
            else
                scan_spec(HBOX_GROUP, true);
        } else {
            if (k == VMODE)
                scan_spec(VBOX_GROUP, true);
            else {
                scan_spec(VTOP_GROUP, true);
                k = VMODE;
            }
            normal_paragraph();
        }

        push_nest();
        cur_list.mode = -(int32_t) k;

        if (k == VMODE) {
            cur_list.aux.b32.s1 = IGNORE_DEPTH;
            if (LOCAL(every_vbox) != TEX_NULL)
                begin_token_list(LOCAL(every_vbox), EVERY_VBOX_TEXT);
        } else {
            cur_list.aux.b32.s0 = 1000;
            if (LOCAL(every_hbox) != TEX_NULL)
                begin_token_list(LOCAL(every_hbox), EVERY_HBOX_TEXT);
        }

        return;
    }

    box_end(box_context);
}
#endif

/* scan_box: translated to Rust in src/scan.rs */


/* package: translated to Rust in src/scan.rs */

/* norm_min: translated to Rust in src/helpers.rs */


/* new_graf: translated to Rust in src/scan.rs */

/* indent_in_hmode: translated to Rust in src/scan.rs */

/* head_for_vmode: translated to Rust in src/scan.rs */

/* end_graf: translated to Rust in src/scan.rs */

/* begin_insert_or_adjust: translated to Rust in src/scan.rs */

/* make_mark: translated to Rust in src/scan.rs */

/* append_penalty: translated to Rust in src/scan.rs */

/* delete_last: translated to Rust in src/scan.rs */

/* unpackage: translated to Rust in src/scan.rs */

/* append_italic_correction: translated to Rust in src/scan.rs */

/* append_discretionary: translated to Rust in src/scan.rs */

/* build_discretionary: translated to Rust in src/scan.rs */

/* make_accent: translated to Rust in src/scan.rs */

/* align_error: translated to Rust in src/scan.rs */

/* no_align_error: translated to Rust in src/scan.rs */

/* omit_error: translated to Rust in src/scan.rs */

/* do_endv: translated to Rust in src/scan.rs */

/* cs_error: translated to Rust in src/scan.rs */

/* push_math: translated to Rust in src/scan.rs */


/* just_copy: translated to Rust in src/scan.rs */

/* just_reverse: translated to Rust in src/scan.rs */


/* get_r_token: translated to Rust in src/scan.rs */


/* trap_zero_glue: translated to Rust in src/scan.rs */


/* do_register_command: translated to Rust in src/scan.rs */


/* alter_aux: translated to Rust in src/scan.rs */

/* alter_prev_graf: translated to Rust in src/scan.rs */

/* alter_page_so_far: translated to Rust in src/scan.rs */

/* alter_integer: translated to Rust in src/scan.rs */

/* alter_box_dimen: translated to Rust in src/scan.rs */

/* new_font: translated to Rust in src/scan.rs */

/* new_interaction: translated to Rust in src/scan.rs */

/* issue_message: translated to Rust in src/scan.rs */


/* shift_case: translated to Rust in src/scan.rs */


/* show_whatever: translated to Rust in src/scan.rs */

/* new_write_whatsit: translated to Rust in src/scan.rs */

/* scan_and_pack_name: translated to Rust in src/scan.rs */

/* do_extension: translated to Rust in src/scan.rs */

/* fix_language: translated to Rust in src/scan.rs */


/* insert_src_special: translated to Rust in src/scan.rs */


/* append_src_special: translated to Rust in src/scan.rs */


/* tt_insert_special: translated to Rust in src/scan.rs */

/* handle_right_brace: translated to Rust in src/scan.rs */


void
main_control(void)
{
    int32_t t;

    if (LOCAL(every_job) != TEX_NULL)
        begin_token_list(LOCAL(every_job), EVERY_JOB_TEXT);

big_switch: /* big_switch */
    get_x_token();

reswitch:
    /*1066: */

    if (INTPAR(tracing_commands) > 0)
        show_cur_cmd_chr();

    switch (abs(cur_list.mode) + cur_cmd) {
    case HMODE + LETTER:
    case HMODE + OTHER_CHAR:
    case HMODE + CHAR_GIVEN:
        goto main_loop;
        break;

    case HMODE + CHAR_NUM:
        scan_usv_num();
        cur_chr = cur_val;
        goto main_loop;
        break;

    case HMODE + NO_BOUNDARY:
        get_x_token();
        if (cur_cmd == LETTER || cur_cmd == OTHER_CHAR || cur_cmd == CHAR_GIVEN || cur_cmd == CHAR_NUM)
            cancel_boundary = true;
        goto reswitch;
        break;
    }

    if (abs(cur_list.mode) == HMODE) {
        if (
            INTPAR(xetex_inter_char_tokens) > 0
            && space_class != CHAR_CLASS_LIMIT
            && prev_class != CHAR_CLASS_LIMIT - 1
        ) {
            prev_class = CHAR_CLASS_LIMIT - 1;

            find_sa_element(
                INTER_CHAR_VAL,
                space_class * CHAR_CLASS_LIMIT + CHAR_CLASS_LIMIT - 1,
                false
            );

            if (cur_ptr != TEX_NULL && ETEX_SA_ptr(cur_ptr) != TEX_NULL) {
                if (cur_cs == 0) {
                    if (cur_cmd == CHAR_NUM)
                        cur_cmd = OTHER_CHAR;
                    cur_tok = (cur_cmd * MAX_CHAR_VAL) + cur_chr;
                } else
                    cur_tok = CS_TOKEN_FLAG + cur_cs;

                back_input();
                begin_token_list(ETEX_SA_ptr(cur_ptr), INTER_CHAR_TEXT);
                goto big_switch;
            }
        }
    }

    switch (abs(cur_list.mode) + cur_cmd) {
    case HMODE + SPACER:
        if (cur_list.aux.b32.s0 /*space_factor*/ == 1000)
            goto append_normal_space;
        else
            app_space();
        break;

    case HMODE + EX_SPACE:
    case MMODE + EX_SPACE:
        goto append_normal_space;
        break;

    /* 1101*: Cases of main_control that are not part of the inner loop: */

    #define ANY_MODE(cmd) \
        HMODE + (cmd): \
        case VMODE + (cmd): \
        case MMODE + (cmd) /* note no leading "case" or trailing colon */

    #define NON_MATH(cmd) \
        VMODE + (cmd): \
        case HMODE + (cmd) /* no leading "case", no trailing colon */

    case ANY_MODE(RELAX):
    case VMODE + SPACER:
    case MMODE + SPACER:
    case MMODE + NO_BOUNDARY:
        break;

    case ANY_MODE(IGNORE_SPACES):
        if (cur_chr == 0) {
            do {
                get_x_token();
            } while (cur_cmd == SPACER);
        } else {
            t = scanner_status;
            scanner_status = NORMAL;
            get_next();
            scanner_status = t;

            if (cur_cs < HASH_BASE)
                cur_cs = prim_lookup(cur_cs - SINGLE_BASE);
            else
                cur_cs = prim_lookup(hash[cur_cs].s1);

            if (cur_cs != UNDEFINED_PRIMITIVE) {
                cur_cmd = eqtb[PRIM_EQTB_BASE + cur_cs].b16.s1;
                cur_chr = eqtb[PRIM_EQTB_BASE + cur_cs].b32.s1;
                cur_tok = CS_TOKEN_FLAG + PRIM_EQTB_BASE + cur_cs;
            }
        }
        goto reswitch;
        break;

    case VMODE + STOP:
        if (its_all_over())
            return; /* "this is the only way out" */
        break;

    case VMODE + VMOVE: /* 1104*: Forbidden cases detected in main_control */
    case HMODE + HMOVE:
    case MMODE + HMOVE:
    case ANY_MODE(LAST_ITEM):
    case VMODE + VADJUST:
    case VMODE + ITAL_CORR:
    case NON_MATH(EQ_NO):
    case ANY_MODE(MAC_PARAM):
        report_illegal_case();
        break;

    case NON_MATH(SUP_MARK): /* 1102*: math-only cases in non-math modes, or vice versa */
    case NON_MATH(SUB_MARK):
    case NON_MATH(MATH_CHAR_NUM):
    case NON_MATH(MATH_GIVEN):
    case NON_MATH(XETEX_MATH_GIVEN):
    case NON_MATH(MATH_COMP):
    case NON_MATH(DELIM_NUM):
    case NON_MATH(LEFT_RIGHT):
    case NON_MATH(ABOVE):
    case NON_MATH(RADICAL):
    case NON_MATH(MATH_STYLE):
    case NON_MATH(MATH_CHOICE):
    case NON_MATH(VCENTER):
    case NON_MATH(NON_SCRIPT):
    case NON_MATH(MKERN):
    case NON_MATH(LIMIT_SWITCH):
    case NON_MATH(MSKIP):
    case NON_MATH(MATH_ACCENT):
    case MMODE + ENDV:
    case MMODE + PAR_END:
    case MMODE + STOP:
    case MMODE + VSKIP:
    case MMODE + UN_VBOX:
    case MMODE + VALIGN:
    case MMODE + HRULE:
        insert_dollar_sign();
        break;

    case VMODE + HRULE: /* 1112*: cases of main_control that build boxes and lists */
    case HMODE + VRULE:
    case MMODE + VRULE:
        mem[cur_list.tail].b32.s1 = scan_rule_spec();
        cur_list.tail = LLIST_link(cur_list.tail);

        if (abs(cur_list.mode) == VMODE)
            cur_list.aux.b32.s1 = IGNORE_DEPTH;
        else if (abs(cur_list.mode) == HMODE)
            cur_list.aux.b32.s0 = 1000;
        break;

    case VMODE + VSKIP:
    case HMODE + HSKIP:
    case MMODE + HSKIP:
    case MMODE + MSKIP:
        append_glue();
        break;

    case ANY_MODE(KERN):
    case MMODE + MKERN:
        append_kern();
        break;

    case NON_MATH(LEFT_BRACE):
        new_save_level(SIMPLE_GROUP);
        break;

    case ANY_MODE(BEGIN_GROUP):
        new_save_level(SEMI_SIMPLE_GROUP);
        break;

    case ANY_MODE(END_GROUP):
        if (cur_group == SEMI_SIMPLE_GROUP)
            unsave();
        else
            off_save();
        break;

    case ANY_MODE(RIGHT_BRACE):
        handle_right_brace();
        break;

    case VMODE + HMOVE:
    case HMODE + VMOVE:
    case MMODE + VMOVE:
        t = cur_chr;
        scan_dimen(false, false, false);
        if (t == 0)
            scan_box(cur_val);
        else
            scan_box(-(int32_t) cur_val);
        break;

    case ANY_MODE(LEADER_SHIP):
        scan_box(LEADER_FLAG - A_LEADERS + cur_chr);
        break;

    case ANY_MODE(MAKE_BOX):
        begin_box(0);
        break;

    case VMODE + START_PAR:
        new_graf(cur_chr > 0);
        break;

    case VMODE + LETTER:
    case VMODE + OTHER_CHAR:
    case VMODE + CHAR_NUM:
    case VMODE + CHAR_GIVEN:
    case VMODE + MATH_SHIFT:
    case VMODE + UN_HBOX:
    case VMODE + VRULE:
    case VMODE + ACCENT:
    case VMODE + DISCRETIONARY:
    case VMODE + HSKIP:
    case VMODE + VALIGN:
    case VMODE + EX_SPACE:
    case VMODE + NO_BOUNDARY:
        back_input();
        new_graf(true);
        break;

    case HMODE + START_PAR:
    case MMODE + START_PAR:
        indent_in_hmode();
        break;

    case VMODE + PAR_END:
        normal_paragraph();
        if (cur_list.mode > 0)
            build_page();
        break;

    case HMODE + PAR_END:
        if (align_state < 0)
            off_save();
        end_graf();
        if (cur_list.mode == VMODE)
            build_page();
        break;

    case HMODE + STOP:
    case HMODE + VSKIP:
    case HMODE + HRULE:
    case HMODE + UN_VBOX:
    case HMODE + HALIGN:
        head_for_vmode();
        break;

    case ANY_MODE(INSERT):
    case HMODE + VADJUST:
    case MMODE + VADJUST:
        begin_insert_or_adjust();
        break;

    case ANY_MODE(MARK):
        make_mark();
        break;

    case ANY_MODE(BREAK_PENALTY):
        append_penalty();
        break;

    case ANY_MODE(REMOVE_ITEM):
        delete_last();
        break;

    case VMODE + UN_VBOX:
    case HMODE + UN_HBOX:
    case MMODE + UN_HBOX:
        unpackage();
        break;

    case HMODE + ITAL_CORR: /* 1168*: */
        append_italic_correction();
        break;

    case MMODE + ITAL_CORR:
        mem[cur_list.tail].b32.s1 = new_kern(0);
        cur_list.tail = LLIST_link(cur_list.tail);
        break;

    case HMODE + DISCRETIONARY:
    case MMODE + DISCRETIONARY:
        append_discretionary();
        break;

    case HMODE + ACCENT:
        make_accent();
        break;

    case ANY_MODE(CAR_RET):
    case ANY_MODE(TAB_MARK):
        align_error();
        break;

    case ANY_MODE(NO_ALIGN):
        no_align_error();
        break;

    case ANY_MODE(OMIT):
        omit_error();
        break;

    case VMODE + HALIGN:
        init_align();
        break;

    case HMODE + VALIGN:
        if (cur_chr > 0) {
            if (eTeX_enabled(INTPAR(texxet) > 0, cur_cmd, cur_chr)) {
                mem[cur_list.tail].b32.s1 = new_math(0, cur_chr);
                cur_list.tail = LLIST_link(cur_list.tail);
            }
        } else /*:1490 */
            init_align();
        break;

    case MMODE + HALIGN:
        if (privileged()) {
            if (cur_group == MATH_SHIFT_GROUP)
                init_align();
            else
                off_save();
        }
        break;

    case VMODE + ENDV:
    case HMODE + ENDV:
        do_endv();
        break;

    case ANY_MODE(END_CS_NAME):
        cs_error();
        break;

    /* Math-related commands */

    case HMODE + MATH_SHIFT: /* 1193*: */
        init_math();
        break;

    case MMODE + EQ_NO:
        if (privileged()) {
            if (cur_group == MATH_SHIFT_GROUP)
                start_eq_no();
            else
                off_save();
        }
        break;

    case MMODE + LEFT_BRACE:
        mem[cur_list.tail].b32.s1 = new_noad();
        cur_list.tail = LLIST_link(cur_list.tail);
        back_input();
        scan_math(cur_list.tail + 1);
        break;

    case MMODE + LETTER:
    case MMODE + OTHER_CHAR:
    case MMODE + CHAR_GIVEN:
        set_math_char(MATH_CODE(cur_chr));
        break;

    case MMODE + CHAR_NUM:
        scan_char_num();
        cur_chr = cur_val;
        set_math_char(MATH_CODE(cur_chr));
        break;

    case MMODE + MATH_CHAR_NUM:
        if (cur_chr == 2) { /* \Umathchar? */
            scan_math_class_int();
            t = set_class(cur_val);
            scan_math_fam_int();
            t = t + set_family(cur_val);
            scan_usv_num();
            t = t + cur_val;
            set_math_char(t);
        } else if (cur_chr == 1) { /* \Umathcharnum/ */
            scan_xetex_math_char_int();
            set_math_char(cur_val);
        } else {
            scan_fifteen_bit_int();
            set_math_char(set_class(cur_val / 4096) + set_family((cur_val % 4096) / 256) +
                            (cur_val % 256));
        }
        break;

    case MMODE + MATH_GIVEN:
        set_math_char(
            set_class(cur_chr / 4096) + set_family((cur_chr % 4096) / 256) + (cur_chr % 256)
        );
        break;

    case MMODE + XETEX_MATH_GIVEN:
        set_math_char(cur_chr);
        break;

    case MMODE + DELIM_NUM:
        if (cur_chr == 1) { /* \Udelimiter? */
            scan_math_class_int();
            t = set_class(cur_val);
            scan_math_fam_int();
            t = t + set_family(cur_val);
            scan_usv_num();
            t = t + cur_val;
            set_math_char(t);
        } else {
            scan_delimiter_int();
            cur_val = cur_val / 4096;
            set_math_char(set_class(cur_val / 4096) + set_family((cur_val % 4096) / 256) +
                            (cur_val % 256));
        }
        break;

    case MMODE + MATH_COMP:
        mem[cur_list.tail].b32.s1 = new_noad();
        cur_list.tail = LLIST_link(cur_list.tail);
        mem[cur_list.tail].b16.s1 = cur_chr;
        scan_math(cur_list.tail + 1);
        break;

    case MMODE + LIMIT_SWITCH:
        math_limit_switch();
        break;

    case MMODE + RADICAL:
        math_radical();
        break;

    case MMODE + ACCENT: /* 1220*: */
    case MMODE + MATH_ACCENT:
        math_ac();
        break;

    case MMODE + VCENTER:
        scan_spec(VCENTER_GROUP, false);
        normal_paragraph();
        push_nest();
        cur_list.mode = -1;
        cur_list.aux.b32.s1 = IGNORE_DEPTH;
        if ((insert_src_special_every_vbox))
            insert_src_special();
        if (LOCAL(every_vbox) != TEX_NULL)
            begin_token_list(LOCAL(every_vbox), EVERY_VBOX_TEXT);
        break;

    case MMODE + MATH_STYLE:
        mem[cur_list.tail].b32.s1 = new_style(cur_chr);
        cur_list.tail = LLIST_link(cur_list.tail);
        break;

    case MMODE + NON_SCRIPT:
        mem[cur_list.tail].b32.s1 = new_glue(0);
        cur_list.tail = LLIST_link(cur_list.tail);
        mem[cur_list.tail].b16.s0 = COND_MATH_GLUE;
        break;

    case MMODE + MATH_CHOICE:
        append_choices();
        break;

    case MMODE + SUB_MARK:
    case MMODE + SUP_MARK:
        sub_sup();
        break;

    case MMODE + ABOVE:
        math_fraction();
        break;

    case MMODE + LEFT_RIGHT:
        math_left_right();
        break;

    case MMODE + MATH_SHIFT:
        /* "Here is the only way out of math mode" */
        if (cur_group == MATH_SHIFT_GROUP)
            after_math();
        else
            off_save();
        break;

    /* 1266*: "cases of main_control that don't depend on mode" */

    case ANY_MODE(TOKS_REGISTER):
    case ANY_MODE(ASSIGN_TOKS):
    case ANY_MODE(ASSIGN_INT):
    case ANY_MODE(ASSIGN_DIMEN):
    case ANY_MODE(ASSIGN_GLUE):
    case ANY_MODE(ASSIGN_MU_GLUE):
    case ANY_MODE(ASSIGN_FONT_DIMEN):
    case ANY_MODE(ASSIGN_FONT_INT):
    case ANY_MODE(SET_AUX):
    case ANY_MODE(SET_PREV_GRAF):
    case ANY_MODE(SET_PAGE_DIMEN):
    case ANY_MODE(SET_PAGE_INT):
    case ANY_MODE(SET_BOX_DIMEN):
    case ANY_MODE(SET_SHAPE):
    case ANY_MODE(DEF_CODE):
    case ANY_MODE(XETEX_DEF_CODE):
    case ANY_MODE(DEF_FAMILY):
    case ANY_MODE(SET_FONT):
    case ANY_MODE(DEF_FONT):
    case ANY_MODE(REGISTER):
    case ANY_MODE(ADVANCE):
    case ANY_MODE(MULTIPLY):
    case ANY_MODE(DIVIDE):
    case ANY_MODE(PREFIX):
    case ANY_MODE(LET):
    case ANY_MODE(SHORTHAND_DEF):
    case ANY_MODE(READ_TO_CS):
    case ANY_MODE(DEF):
    case ANY_MODE(SET_BOX):
    case ANY_MODE(HYPH_DATA):
    case ANY_MODE(SET_INTERACTION):
        prefixed_command();
        break;

    case ANY_MODE(AFTER_ASSIGNMENT):
        get_token();
        after_token = cur_tok;
        break;

    case ANY_MODE(AFTER_GROUP):
        get_token();
        save_for_after(cur_tok);
        break;

    case ANY_MODE(IN_STREAM):
        open_or_close_in();
        break;

    case ANY_MODE(MESSAGE):
        issue_message();
        break;

    case ANY_MODE(CASE_SHIFT):
        shift_case();
        break;

    case ANY_MODE(XRAY):
        show_whatever();
        break;

    case ANY_MODE(EXTENSION): /* 1404*: extensions */
        do_extension();
        break;
    }

    goto big_switch;

main_loop: /*1069: */
    if (cur_list.head == cur_list.tail && cur_list.mode > 0) {
        if (insert_src_special_auto)
            append_src_special();
    }

    prev_class = CHAR_CLASS_LIMIT - 1;

    if (font_area[eqtb[CUR_FONT_LOC].b32.s1] == AAT_FONT_FLAG || font_area[eqtb[CUR_FONT_LOC].b32.s1] == OTGR_FONT_FLAG) {
        if (cur_list.mode > 0) {
            if (INTPAR(language) != cur_list.aux.b32.s1)
                fix_language();
        }

        main_h = 0;
        main_f = eqtb[CUR_FONT_LOC].b32.s1;
        native_len = 0;

collect_native:
        main_s = SF_CODE(cur_chr) % 65536L;

        if (main_s == 1000) {
            cur_list.aux.b32.s0 = 1000;
        } else if (main_s < 1000) {
            if (main_s > 0)
                cur_list.aux.b32.s0 = main_s;
        } else if (cur_list.aux.b32.s0 < 1000) {
            cur_list.aux.b32.s0 = 1000;
        } else {
            cur_list.aux.b32.s0 = main_s;
        }

        cur_ptr = TEX_NULL;
        space_class = SF_CODE(cur_chr) / 65536L;

        if (INTPAR(xetex_inter_char_tokens) > 0 && space_class != CHAR_CLASS_LIMIT) {
            if (prev_class == CHAR_CLASS_LIMIT - 1) {
                if (cur_input.state != TOKEN_LIST || cur_input.index != BACKED_UP_CHAR) {
                    find_sa_element(INTER_CHAR_VAL,
                                    ((CHAR_CLASS_LIMIT - 1)) * CHAR_CLASS_LIMIT + space_class,
                                    false);
                    if (cur_ptr != TEX_NULL && ETEX_SA_ptr(cur_ptr) != TEX_NULL) {
                        if (cur_cmd != LETTER)
                            cur_cmd = OTHER_CHAR;
                        cur_tok = (cur_cmd * MAX_CHAR_VAL) + cur_chr;
                        back_input();
                        cur_input.index = BACKED_UP_CHAR;
                        begin_token_list(mem[cur_ptr + 1].b32.s1, INTER_CHAR_TEXT);
                        goto big_switch;
                    }
                }
            } else {
                find_sa_element(INTER_CHAR_VAL, prev_class * CHAR_CLASS_LIMIT + space_class, false);
                if (cur_ptr != TEX_NULL && ETEX_SA_ptr(cur_ptr) != TEX_NULL) {
                    if (cur_cmd != LETTER)
                        cur_cmd = OTHER_CHAR;
                    cur_tok = (cur_cmd * MAX_CHAR_VAL) + cur_chr;
                    back_input();
                    cur_input.index = BACKED_UP_CHAR;
                    begin_token_list(mem[cur_ptr + 1].b32.s1, INTER_CHAR_TEXT);
                    prev_class = ((CHAR_CLASS_LIMIT - 1));
                    goto collected;
                }
            }
            prev_class = space_class;
        }

        if (cur_chr > 65535L) {
            while (native_text_size <= native_len + 2) {
                native_text_size = native_text_size + 128;
                native_text = xrealloc(native_text, native_text_size * sizeof(UTF16_code));
            }

            native_text[native_len] = (cur_chr - 65536L) / 1024 + 0xD800;
            native_len++;
            native_text[native_len] = (cur_chr - 65536L) % 1024 + 0xDC00;
            native_len++;
        } else {
            while (native_text_size <= native_len + 1) {
                native_text_size = native_text_size + 128;
                native_text = xrealloc(native_text, native_text_size * sizeof(UTF16_code));
            }

            native_text[native_len] = cur_chr;
            native_len++;
        }

        is_hyph = (cur_chr == hyphen_char[main_f]) || ((INTPAR(xetex_dash_break) > 0)
                                                       && ((cur_chr == 8212) || (cur_chr == 8211)));

        if ((main_h == 0) && is_hyph)
            main_h = native_len;

        get_next();
        if (cur_cmd == LETTER || cur_cmd == OTHER_CHAR || cur_cmd == CHAR_GIVEN)
            goto collect_native;

        x_token();
        if (cur_cmd == LETTER || cur_cmd == OTHER_CHAR || cur_cmd == CHAR_GIVEN)
            goto collect_native;

        if (cur_cmd == CHAR_NUM) {
            scan_usv_num();
            cur_chr = cur_val;
            goto collect_native;
        }

        if ((INTPAR(xetex_inter_char_tokens) > 0) && (space_class != CHAR_CLASS_LIMIT)
            && (prev_class != ((CHAR_CLASS_LIMIT - 1)))) {
            prev_class = ((CHAR_CLASS_LIMIT - 1));
            find_sa_element(INTER_CHAR_VAL,
                            space_class * CHAR_CLASS_LIMIT + ((CHAR_CLASS_LIMIT - 1)), false);
            if (cur_ptr != TEX_NULL && ETEX_SA_ptr(cur_ptr) != TEX_NULL) {
                if (cur_cs == 0) {
                    if (cur_cmd == CHAR_NUM)
                        cur_cmd = OTHER_CHAR;
                    cur_tok = (cur_cmd * MAX_CHAR_VAL) + cur_chr;
                } else
                    cur_tok = CS_TOKEN_FLAG + cur_cs;
                back_input();
                begin_token_list(mem[cur_ptr + 1].b32.s1, INTER_CHAR_TEXT);
                goto collected;
            }
        }

collected:
        if ((font_mapping[main_f] != NULL)) {
            main_k = apply_mapping(font_mapping[main_f], native_text, native_len);
            native_len = 0;

            while (native_text_size <= native_len + main_k) {
                native_text_size = native_text_size + 128;
                native_text = xrealloc(native_text, native_text_size * sizeof(UTF16_code));
            }

            main_h = 0;

            {
                register int32_t for_end;
                main_p = 0;
                for_end = main_k - 1;
                if (main_p <= for_end)
                    do {
                        {
                            native_text[native_len] = mapped_text[main_p];
                            native_len++;
                        }
                        if ((main_h == 0)
                            && ((mapped_text[main_p] == hyphen_char[main_f])
                                || ((INTPAR(xetex_dash_break) > 0)
                                    && ((mapped_text[main_p] == 8212) || (mapped_text[main_p] == 8211)))))
                            main_h = native_len;
                    }
                    while (main_p++ < for_end);
            }
        }

        if (INTPAR(tracing_lost_chars) > 0) {
            temp_ptr = 0;
            while ((temp_ptr < native_len)) {
                main_k = native_text[temp_ptr];
                temp_ptr++;
                if ((main_k >= 0xD800) && (main_k < 0xDC00)) {
                    main_k = 65536L + (main_k - 0xD800) * 1024;
                    main_k = main_k + native_text[temp_ptr] - 0xDC00;
                    temp_ptr++;
                }
                if (map_char_to_glyph(main_f, main_k) == 0)
                    char_warning(main_f, main_k);
            }
        }

        main_k = native_len;
        main_pp = cur_list.tail;

        if (cur_list.mode == HMODE) {
            main_ppp = cur_list.head;

            while (main_ppp != main_pp && mem[main_ppp].b32.s1 != main_pp) {
                if (!is_char_node(main_ppp) && NODE_type(main_ppp) == DISC_NODE) {
                    temp_ptr = main_ppp;
                    {
                        register int32_t for_end;
                        main_p = 1;
                        for_end = mem[temp_ptr].b16.s0;
                        if (main_p <= for_end)
                            do
                                main_ppp = LLIST_link(main_ppp);
                            while (main_p++ < for_end);
                    }
                }

                if (main_ppp != main_pp)
                    main_ppp = LLIST_link(main_ppp);
            }

            temp_ptr = 0;

            do {
                if (main_h == 0)
                    main_h = main_k;

                if ((((main_pp) != TEX_NULL && (!(is_char_node(main_pp)))
                      && (NODE_type(main_pp) == WHATSIT_NODE)
                      && ((mem[main_pp].b16.s0 == NATIVE_WORD_NODE)
                          || (mem[main_pp].b16.s0 == NATIVE_WORD_NODE_AT))))
                    && (mem[main_pp + 4].b16.s2 == main_f) && (main_ppp != main_pp) && (!(is_char_node(main_ppp)))
                    && (NODE_type(main_ppp) != DISC_NODE)
                ) {
                    main_k = main_h + mem[main_pp + 4].b16.s1;

                    while (native_text_size <= native_len + main_k) {
                        native_text_size = native_text_size + 128;
                        native_text = xrealloc(native_text, native_text_size * sizeof(UTF16_code));
                    }

                    save_native_len = native_len;

                    {
                        register int32_t for_end;
                        main_p = 0;
                        for_end = mem[main_pp + 4].b16.s1 - 1;
                        if (main_p <= for_end)
                            do {
                                native_text[native_len] = NATIVE_NODE_text(main_pp)[main_p];
                                native_len++;
                            }
                            while (main_p++ < for_end);
                    }
                    {
                        register int32_t for_end;
                        main_p = 0;
                        for_end = main_h - 1;
                        if (main_p <= for_end)
                            do {
                                native_text[native_len] = native_text[temp_ptr + main_p];
                                native_len++;
                            }
                            while (main_p++ < for_end);
                    }

                    do_locale_linebreaks(save_native_len, main_k);
                    native_len = save_native_len;
                    main_k = native_len - main_h - temp_ptr;
                    temp_ptr = main_h;
                    main_h = 0;

                    while ((main_h < main_k) && (native_text[temp_ptr + main_h] != hyphen_char[main_f])
                           && ((!(INTPAR(xetex_dash_break) > 0))
                               || ((native_text[temp_ptr + main_h] != 8212)
                                   && (native_text[temp_ptr + main_h] != 8211))))
                        main_h++;

                    if ((main_h < main_k))
                        main_h++;

                    mem[main_ppp].b32.s1 = mem[main_pp].b32.s1;
                    mem[main_pp].b32.s1 = TEX_NULL;
                    flush_node_list(main_pp);
                    main_pp = cur_list.tail;

                    while ((mem[main_ppp].b32.s1 != main_pp))
                        main_ppp = LLIST_link(main_ppp);
                } else {
                    do_locale_linebreaks(temp_ptr, main_h);
                    temp_ptr = temp_ptr + main_h;
                    main_k = main_k - main_h;
                    main_h = 0;
                    while ((main_h < main_k) && (native_text[temp_ptr + main_h] != hyphen_char[main_f])
                           && ((!(INTPAR(xetex_dash_break) > 0))
                               || ((native_text[temp_ptr + main_h] != 8212)
                                   && (native_text[temp_ptr + main_h] != 8211))))
                        main_h++;
                    if ((main_h < main_k))
                        main_h++;
                }

                if ((main_k > 0) || is_hyph) {
                    mem[cur_list.tail].b32.s1 = new_disc();
                    cur_list.tail = LLIST_link(cur_list.tail);
                    main_pp = cur_list.tail;
                }
            } while (!(main_k == 0));
        } else {
            main_ppp = cur_list.head;

            while (main_ppp != main_pp && mem[main_ppp].b32.s1 != main_pp) {
                if (!is_char_node(main_ppp) && NODE_type(main_ppp) == DISC_NODE) {
                    temp_ptr = main_ppp;
                    {
                        register int32_t for_end;
                        main_p = 1;
                        for_end = mem[temp_ptr].b16.s0;
                        if (main_p <= for_end)
                            do
                                main_ppp = LLIST_link(main_ppp);
                            while (main_p++ < for_end);
                    }
                }

                if (main_ppp != main_pp)
                    main_ppp = LLIST_link(main_ppp);
            }

            if ((((main_pp) != TEX_NULL && (!(is_char_node(main_pp))) && (NODE_type(main_pp) == WHATSIT_NODE)
                  && ((mem[main_pp].b16.s0 == NATIVE_WORD_NODE)
                      || (mem[main_pp].b16.s0 == NATIVE_WORD_NODE_AT)))) && (mem[main_pp + 4].b16.s2 == main_f)
                && (main_ppp != main_pp) && (!(is_char_node(main_ppp))) && (NODE_type(main_ppp) != DISC_NODE)
            ) {
                mem[main_pp].b32.s1 = new_native_word_node(main_f, main_k + mem[main_pp + 4].b16.s1);
                cur_list.tail = mem[main_pp].b32.s1;
                {
                    register int32_t for_end;
                    main_p = 0;
                    for_end = mem[main_pp + 4].b16.s1 - 1;
                    if (main_p <= for_end)
                        do
                            NATIVE_NODE_text(cur_list.tail)[main_p] = NATIVE_NODE_text(main_pp)[main_p];
                        while (main_p++ < for_end);
                }
                {
                    register int32_t for_end;
                    main_p = 0;
                    for_end = main_k - 1;
                    if (main_p <= for_end)
                        do
                            NATIVE_NODE_text(cur_list.tail)[main_p + mem[main_pp + 4].b16.s1] = native_text[main_p];
                        while (main_p++ < for_end);
                }
                set_native_metrics(cur_list.tail, (INTPAR(xetex_use_glyph_metrics) > 0));
                main_p = cur_list.head;
                if (main_p != main_pp)
                    while (mem[main_p].b32.s1 != main_pp)
                        main_p = LLIST_link(main_p);
                mem[main_p].b32.s1 = mem[main_pp].b32.s1;
                mem[main_pp].b32.s1 = TEX_NULL;
                flush_node_list(main_pp);
            } else {
                mem[main_pp].b32.s1 = new_native_word_node(main_f, main_k);
                cur_list.tail = mem[main_pp].b32.s1;
                {
                    register int32_t for_end;
                    main_p = 0;
                    for_end = main_k - 1;
                    if (main_p <= for_end)
                        do
                            NATIVE_NODE_text(cur_list.tail)[main_p] =  native_text[main_p];
                        while (main_p++ < for_end);
                }
                set_native_metrics(cur_list.tail, (INTPAR(xetex_use_glyph_metrics) > 0));
            }
        }

        if (INTPAR(xetex_interword_space_shaping) > 0) {
            main_p = cur_list.head;
            main_pp = TEX_NULL;
            while (main_p != cur_list.tail) {

                if ((((main_p) != TEX_NULL && (!(is_char_node(main_p)))
                      && (NODE_type(main_p) == WHATSIT_NODE)
                      && ((mem[main_p].b16.s0 == NATIVE_WORD_NODE)
                          || (mem[main_p].b16.s0 == NATIVE_WORD_NODE_AT)))))
                    main_pp = main_p;
                main_p = LLIST_link(main_p);
            }

            if ((main_pp != TEX_NULL)) {
                if (mem[main_pp + 4].b16.s2 == main_f) {
                    main_p = mem[main_pp].b32.s1;
                    while (!(is_char_node(main_p))
                           && ((NODE_type(main_p) == PENALTY_NODE) || (NODE_type(main_p) == INS_NODE)
                               || (NODE_type(main_p) == MARK_NODE) || (NODE_type(main_p) == ADJUST_NODE)
                               || ((NODE_type(main_p) == WHATSIT_NODE) && (mem[main_p].b16.s0 <= 4))))
                        main_p = LLIST_link(main_p);
                    if (!(is_char_node(main_p)) && (NODE_type(main_p) == GLUE_NODE)) {
                        main_ppp = mem[main_p].b32.s1;
                        while (!(is_char_node(main_ppp))
                               && ((NODE_type(main_ppp) == PENALTY_NODE)
                                   || (NODE_type(main_ppp) == INS_NODE)
                                   || (NODE_type(main_ppp) == MARK_NODE)
                                   || (NODE_type(main_ppp) == ADJUST_NODE)
                                   || ((NODE_type(main_ppp) == WHATSIT_NODE) && (mem[main_ppp].b16.s0 <= 4))))
                            main_ppp = LLIST_link(main_ppp);
                        if (main_ppp == cur_list.tail) {
                            temp_ptr =
                                new_native_word_node(main_f,
                                                     mem[main_pp + 4].b16.s1 + 1 + mem[cur_list.tail +
                                                                                        4].b16.s1);
                            main_k = 0;
                            {
                                register int32_t for_end;
                                t = 0;
                                for_end = mem[main_pp + 4].b16.s1 - 1;
                                if (t <= for_end)
                                    do {
                                        NATIVE_NODE_text(temp_ptr)[main_k] = NATIVE_NODE_text(main_pp)[t];
                                        main_k++;
                                    }
                                    while (t++ < for_end);
                            }
                            NATIVE_NODE_text(temp_ptr)[main_k] = ' ';
                            main_k++;
                            {
                                register int32_t for_end;
                                t = 0;
                                for_end = mem[cur_list.tail + 4].b16.s1 - 1;
                                if (t <= for_end)
                                    do {
                                        NATIVE_NODE_text(temp_ptr)[main_k] = NATIVE_NODE_text(cur_list.tail)[t];
                                        main_k++;
                                    }
                                    while (t++ < for_end);
                            }
                            set_native_metrics(temp_ptr, (INTPAR(xetex_use_glyph_metrics) > 0));
                            t = mem[temp_ptr + 1].b32.s1 - mem[main_pp + 1].b32.s1 - mem[cur_list.tail + 1].b32.s1;
                            free_node(temp_ptr, mem[temp_ptr + 4].b16.s3);
                            if (t != mem[font_glue[main_f] + 1].b32.s1) {
                                temp_ptr = new_kern(t - mem[font_glue[main_f] + 1].b32.s1);
                                NODE_subtype(temp_ptr) = SPACE_ADJUSTMENT;
                                mem[temp_ptr].b32.s1 = mem[main_p].b32.s1;
                                mem[main_p].b32.s1 = temp_ptr;
                            }
                        }
                    }
                }
            }
        }

        if (cur_ptr != TEX_NULL)
            goto big_switch;
        else
            goto reswitch;
    }

    main_s = SF_CODE(cur_chr) % 65536L;

    if (main_s == 1000) {
        cur_list.aux.b32.s0 = 1000;
    } else if (main_s < 1000) {
        if (main_s > 0)
            cur_list.aux.b32.s0 = main_s;
    } else if (cur_list.aux.b32.s0 < 1000) {
        cur_list.aux.b32.s0 = 1000;
    } else {
        cur_list.aux.b32.s0 = main_s;
    }

    cur_ptr = TEX_NULL;
    space_class = SF_CODE(cur_chr) / 65536L;

    if ((INTPAR(xetex_inter_char_tokens) > 0) && space_class != CHAR_CLASS_LIMIT) {
        if (prev_class == ((CHAR_CLASS_LIMIT - 1))) {
            if ((cur_input.state != TOKEN_LIST) || (cur_input.index != BACKED_UP_CHAR)) {
                find_sa_element(INTER_CHAR_VAL,
                                ((CHAR_CLASS_LIMIT - 1)) * CHAR_CLASS_LIMIT + space_class, false);
                if (cur_ptr != TEX_NULL && ETEX_SA_ptr(cur_ptr) != TEX_NULL) {
                    if (cur_cmd != LETTER)
                        cur_cmd = OTHER_CHAR;
                    cur_tok = (cur_cmd * MAX_CHAR_VAL) + cur_chr;
                    back_input();
                    cur_input.index = BACKED_UP_CHAR;
                    begin_token_list(mem[cur_ptr + 1].b32.s1, INTER_CHAR_TEXT);
                    goto big_switch;
                }
            }
        } else {
            find_sa_element(INTER_CHAR_VAL, prev_class * CHAR_CLASS_LIMIT + space_class, false);
            if (cur_ptr != TEX_NULL && ETEX_SA_ptr(cur_ptr) != TEX_NULL) {
                if (cur_cmd != LETTER)
                    cur_cmd = OTHER_CHAR;
                cur_tok = (cur_cmd * MAX_CHAR_VAL) + cur_chr;
                back_input();
                cur_input.index = BACKED_UP_CHAR;
                begin_token_list(mem[cur_ptr + 1].b32.s1, INTER_CHAR_TEXT);
                prev_class = ((CHAR_CLASS_LIMIT - 1));
                goto big_switch;
            }
        }

        prev_class = space_class;
    }

    main_f = eqtb[CUR_FONT_LOC].b32.s1;
    bchar = font_bchar[main_f];
    false_bchar = font_false_bchar[main_f];

    if (cur_list.mode > 0) {
        if (INTPAR(language) != cur_list.aux.b32.s1)
            fix_language();
    }

    {
        lig_stack = avail;
        if (lig_stack == TEX_NULL)
            lig_stack = get_avail();
        else {

            avail = mem[lig_stack].b32.s1;
            mem[lig_stack].b32.s1 = TEX_NULL;
        }
    }

    mem[lig_stack].b16.s1 = main_f;
    cur_l = cur_chr;
    mem[lig_stack].b16.s0 = cur_l;
    cur_q = cur_list.tail;

    if (cancel_boundary) {
        cancel_boundary = false;
        main_k = NON_ADDRESS;
    } else {
        main_k = bchar_label[main_f];
    }

    if (main_k == NON_ADDRESS)
        goto main_loop_move_2;

    cur_r = cur_l;
    cur_l = TOO_BIG_CHAR;
    goto main_lig_loop_1;

 main_loop_wrapup: /*1070: */
    if (cur_l < TOO_BIG_CHAR) {
        if (mem[cur_q].b32.s1 > TEX_NULL) {
            if (mem[cur_list.tail].b16.s0 == hyphen_char[main_f])
                ins_disc = true;
        }

        if (ligature_present) {
            main_p = new_ligature(main_f, cur_l, mem[cur_q].b32.s1);
            if (lft_hit) {
                mem[main_p].b16.s0 = 2;
                lft_hit = false;
            }
            if (rt_hit) {

                if (lig_stack == TEX_NULL) {
                    mem[main_p].b16.s0++;
                    rt_hit = false;
                }
            }
            mem[cur_q].b32.s1 = main_p;
            cur_list.tail = main_p;
            ligature_present = false;
        }

        if (ins_disc) {
            ins_disc = false;
            if (cur_list.mode > 0) {
                mem[cur_list.tail].b32.s1 = new_disc();
                cur_list.tail = LLIST_link(cur_list.tail);
            }
        }
    }

main_loop_move: /*1071: */
    if (lig_stack == TEX_NULL)
        goto reswitch;

    cur_q = cur_list.tail;
    cur_l = mem[lig_stack].b16.s0;

main_loop_move_1:
    if (!is_char_node(lig_stack))
        goto main_loop_move_lig;

main_loop_move_2:
    if ((effective_char(false, main_f, cur_chr) > font_ec[main_f])
                              || (effective_char(false, main_f, cur_chr) < font_bc[main_f])) {
        char_warning(main_f, cur_chr);
        mem[lig_stack].b32.s1 = avail;
        avail = lig_stack;
        goto big_switch;
    }

    main_i = effective_char_info(main_f, cur_l);

    if (!(main_i.s3 > 0)) {
        char_warning(main_f, cur_chr);
        mem[lig_stack].b32.s1 = avail;
        avail = lig_stack;
        goto big_switch;
    }

    mem[cur_list.tail].b32.s1 = lig_stack;
    cur_list.tail = lig_stack; /*:1071 */

main_loop_lookahead: /*1073: */
    get_next();

    if (cur_cmd == LETTER)
        goto main_loop_lookahead_1;
    if (cur_cmd == OTHER_CHAR)
        goto main_loop_lookahead_1;
    if (cur_cmd == CHAR_GIVEN)
        goto main_loop_lookahead_1;

    x_token();
    if (cur_cmd == LETTER)
        goto main_loop_lookahead_1;
    if (cur_cmd == OTHER_CHAR)
        goto main_loop_lookahead_1;
    if (cur_cmd == CHAR_GIVEN)
        goto main_loop_lookahead_1;

    if (cur_cmd == CHAR_NUM) {
        scan_char_num();
        cur_chr = cur_val;
        goto main_loop_lookahead_1;
    }

    if (cur_cmd == NO_BOUNDARY)
        bchar = TOO_BIG_CHAR;

    cur_r = bchar;
    lig_stack = TEX_NULL;
    goto main_lig_loop;

main_loop_lookahead_1:
    main_s = SF_CODE(cur_chr) % 65536L;

    if (main_s == 1000) {
        cur_list.aux.b32.s0 = 1000;
    } else if (main_s < 1000) {
        if (main_s > 0)
            cur_list.aux.b32.s0 = main_s;
    } else if (cur_list.aux.b32.s0 < 1000) {
        cur_list.aux.b32.s0 = 1000;
    } else {
        cur_list.aux.b32.s0 = main_s;
    }

    cur_ptr = TEX_NULL;
    space_class = SF_CODE(cur_chr) / 65536L;

    if ((INTPAR(xetex_inter_char_tokens) > 0) && space_class != CHAR_CLASS_LIMIT) {
        if (prev_class == ((CHAR_CLASS_LIMIT - 1))) {
            if ((cur_input.state != TOKEN_LIST) || (cur_input.index != BACKED_UP_CHAR)) {
                find_sa_element(INTER_CHAR_VAL,
                                ((CHAR_CLASS_LIMIT - 1)) * CHAR_CLASS_LIMIT + space_class, false);
                if (cur_ptr != TEX_NULL && ETEX_SA_ptr(cur_ptr) != TEX_NULL) {
                    if (cur_cmd != LETTER)
                        cur_cmd = OTHER_CHAR;
                    cur_tok = (cur_cmd * MAX_CHAR_VAL) + cur_chr;
                    back_input();
                    cur_input.index = BACKED_UP_CHAR;
                    begin_token_list(mem[cur_ptr + 1].b32.s1, INTER_CHAR_TEXT);
                    goto big_switch;
                }
            }
        } else {
            find_sa_element(INTER_CHAR_VAL, prev_class * CHAR_CLASS_LIMIT + space_class, false);
            if (cur_ptr != TEX_NULL && ETEX_SA_ptr(cur_ptr) != TEX_NULL) {
                if (cur_cmd != LETTER)
                    cur_cmd = OTHER_CHAR;
                cur_tok = (cur_cmd * MAX_CHAR_VAL) + cur_chr;
                back_input();
                cur_input.index = BACKED_UP_CHAR;
                begin_token_list(mem[cur_ptr + 1].b32.s1, INTER_CHAR_TEXT);
                prev_class = ((CHAR_CLASS_LIMIT - 1));
                goto big_switch;
            }
        }
        prev_class = space_class;
    }

    {
        lig_stack = avail;
        if (lig_stack == TEX_NULL)
            lig_stack = get_avail();
        else {

            avail = mem[lig_stack].b32.s1;
            mem[lig_stack].b32.s1 = TEX_NULL;
        }
    }

    mem[lig_stack].b16.s1 = main_f;
    cur_r = cur_chr;
    mem[lig_stack].b16.s0 = cur_r;

    if (cur_r == false_bchar)
        cur_r = TOO_BIG_CHAR; /*:1073 */

main_lig_loop: /*1074: */
    if (((main_i.s1) % 4) != LIG_TAG)
        goto main_loop_wrapup;
    if (cur_r == TOO_BIG_CHAR)
        goto main_loop_wrapup;

    main_k = lig_kern_base[main_f] + main_i.s0;
    main_j = font_info[main_k].b16;
    if (main_j.s3 <= 128)
        goto main_lig_loop_2;

    main_k = lig_kern_base[main_f] + 256 * main_j.s1 + main_j.s0 + 32768L - 256 * 128;

main_lig_loop_1:
    main_j = font_info[main_k].b16;

main_lig_loop_2:
    if (main_j.s2 == cur_r) {
        if (main_j.s3 <= 128) { /*1075: */
            if (main_j.s1 >= 128) {
                if (cur_l < TOO_BIG_CHAR) {
                    if (mem[cur_q].b32.s1 > TEX_NULL) {
                        if (mem[cur_list.tail].b16.s0 == hyphen_char[main_f])
                            ins_disc = true;
                    }

                    if (ligature_present) {
                        main_p = new_ligature(main_f, cur_l, mem[cur_q].b32.s1);
                        if (lft_hit) {
                            mem[main_p].b16.s0 = 2;
                            lft_hit = false;
                        }
                        if (rt_hit) {

                            if (lig_stack == TEX_NULL) {
                                mem[main_p].b16.s0++;
                                rt_hit = false;
                            }
                        }
                        mem[cur_q].b32.s1 = main_p;
                        cur_list.tail = main_p;
                        ligature_present = false;
                    }

                    if (ins_disc) {
                        ins_disc = false;
                        if (cur_list.mode > 0) {
                            mem[cur_list.tail].b32.s1 = new_disc();
                            cur_list.tail = LLIST_link(cur_list.tail);
                        }
                    }
                }

                mem[cur_list.tail].b32.s1 =
                    new_kern(font_info[kern_base[main_f] + 256 * main_j.s1 + main_j.s0].b32.s1);
                cur_list.tail = LLIST_link(cur_list.tail);
                goto main_loop_move;
            }

            if (cur_l == TOO_BIG_CHAR)
                lft_hit = true;
            else if (lig_stack == TEX_NULL)
                rt_hit = true;

            switch (main_j.s1) {
            case 1:
            case 5:
                cur_l = main_j.s0;
                main_i = FONT_CHARACTER_INFO(main_f, effective_char(true, main_f, cur_l));
                ligature_present = true;
                break;

            case 2:
            case 6:
                cur_r = main_j.s0;

                if (lig_stack == TEX_NULL) {
                    lig_stack = new_lig_item(cur_r);
                    bchar = TOO_BIG_CHAR;
                } else if (is_char_node(lig_stack)) {
                    main_p = lig_stack;
                    lig_stack = new_lig_item(cur_r);
                    mem[lig_stack + 1].b32.s1 = main_p;
                } else {
                    mem[lig_stack].b16.s0 = cur_r;
                }
                break;

            case 3:
                cur_r = main_j.s0;
                main_p = lig_stack;
                lig_stack = new_lig_item(cur_r);
                mem[lig_stack].b32.s1 = main_p;
                break;

            case 7:
            case 11:
                if (cur_l < TOO_BIG_CHAR) {
                    if (mem[cur_q].b32.s1 > TEX_NULL) {

                        if (mem[cur_list.tail].b16.s0 == hyphen_char[main_f])
                            ins_disc = true;
                    }
                    if (ligature_present) {
                        main_p = new_ligature(main_f, cur_l, mem[cur_q].b32.s1);
                        if (lft_hit) {
                            mem[main_p].b16.s0 = 2;
                            lft_hit = false;
                        }
                        if (false) {

                            if (lig_stack == TEX_NULL) {
                                mem[main_p].b16.s0++;
                                rt_hit = false;
                            }
                        }
                        mem[cur_q].b32.s1 = main_p;
                        cur_list.tail = main_p;
                        ligature_present = false;
                    }
                    if (ins_disc) {
                        ins_disc = false;
                        if (cur_list.mode > 0) {
                            mem[cur_list.tail].b32.s1 = new_disc();
                            cur_list.tail = LLIST_link(cur_list.tail);
                        }
                    }
                }
                cur_q = cur_list.tail;
                cur_l = main_j.s0;
                main_i = FONT_CHARACTER_INFO(main_f, effective_char(true, main_f, cur_l));
                ligature_present = true;
                break;

            default:
                cur_l = main_j.s0;
                ligature_present = true;
                if (lig_stack == TEX_NULL)
                    goto main_loop_wrapup;
                else
                    goto main_loop_move_1;
                break;
            }

            if (main_j.s1 > 4) {
                if (main_j.s1 != 7)
                    goto main_loop_wrapup;
            }

            if (cur_l < TOO_BIG_CHAR)
                goto main_lig_loop;

            main_k = bchar_label[main_f];
            goto main_lig_loop_1;
        }
    }

    if (main_j.s3 == 0)
        main_k++;
    else {
        if (main_j.s3 >= 128)
            goto main_loop_wrapup;
        main_k = main_k + main_j.s3 + 1;
    }

    goto main_lig_loop_1;

main_loop_move_lig: /*1072: */
    main_p = mem[lig_stack + 1].b32.s1;

    if (main_p > TEX_NULL) {
        mem[cur_list.tail].b32.s1 = main_p;
        cur_list.tail = LLIST_link(cur_list.tail);
    }

    temp_ptr = lig_stack;
    lig_stack = mem[temp_ptr].b32.s1;
    free_node(temp_ptr, SMALL_NODE_SIZE);
    main_i = FONT_CHARACTER_INFO(main_f, effective_char(true, main_f, cur_l));
    ligature_present = true;

    if (lig_stack == TEX_NULL) {
        if (main_p > TEX_NULL)
            goto main_loop_lookahead;
        else
            cur_r = bchar;
    } else
        cur_r = mem[lig_stack].b16.s0;

    goto main_lig_loop;

append_normal_space:
    if ((INTPAR(xetex_inter_char_tokens) > 0)
                                 && (space_class != CHAR_CLASS_LIMIT)
                                 && (prev_class != ((CHAR_CLASS_LIMIT - 1)))) {
        prev_class = ((CHAR_CLASS_LIMIT - 1));
        find_sa_element(INTER_CHAR_VAL,
                        space_class * CHAR_CLASS_LIMIT + ((CHAR_CLASS_LIMIT - 1)), false);
        if (cur_ptr != TEX_NULL && ETEX_SA_ptr(cur_ptr) != TEX_NULL) {
            if (cur_cs == 0) {
                if (cur_cmd == CHAR_NUM)
                    cur_cmd = OTHER_CHAR;
                cur_tok = (cur_cmd * MAX_CHAR_VAL) + cur_chr;
            } else
                cur_tok = CS_TOKEN_FLAG + cur_cs;
            back_input();
            begin_token_list(mem[cur_ptr + 1].b32.s1, INTER_CHAR_TEXT);
            goto big_switch;
        }
    }

    if (GLUEPAR(space_skip) == 0) {
        main_p = font_glue[eqtb[CUR_FONT_LOC].b32.s1];
        if (main_p == TEX_NULL) {
            main_p = new_spec(0);
            main_k = param_base[eqtb[CUR_FONT_LOC].b32.s1] + 2;
            mem[main_p + 1].b32.s1 = font_info[main_k].b32.s1;
            mem[main_p + 2].b32.s1 = font_info[main_k + 1].b32.s1;
            mem[main_p + 3].b32.s1 = font_info[main_k + 2].b32.s1;
            font_glue[eqtb[CUR_FONT_LOC].b32.s1] = main_p;
        }
        temp_ptr = new_glue(main_p);
    } else
        temp_ptr = new_param_glue(GLUE_PAR__space_skip);

    mem[cur_list.tail].b32.s1 = temp_ptr;
    cur_list.tail = temp_ptr;
    goto big_switch;
}


/* give_err_help: translated to Rust in src/errors.rs */


/* close_files_and_terminate: translated to Rust in src/scan.rs */


/* flush_str: translated to Rust in src/string_pool.rs */

/* tokens_to_string: translated to Rust in src/scan.rs */

/* scan_pdf_ext_toks: translated to Rust in src/scan.rs */

/* compare_strings: translated to Rust in src/scan.rs */


/* prune_page_top: translated to Rust in src/scan.rs */


/* do_marks: translated to Rust in src/scan.rs */


/* do_assignments: translated to Rust in src/scan.rs */


/* new_whatsit: translated to Rust in src/mem.rs */
