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
/* Still C file-statics (used only by untranslated functions): */
static int32_t g;
static int32_t lq, lr;


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


/*639: Output an hlist */
void
hlist_out(void)
{
    scaled_t base_line;
    scaled_t left_edge;
    scaled_t save_h, save_v;
    int32_t this_box;
    glue_ord g_order;
    unsigned char g_sign;
    int32_t p;
    int32_t save_loc;
    int32_t leader_box;
    scaled_t leader_wd;
    scaled_t lx;
    bool outer_doing_leaders;
    scaled_t edge;
    int32_t prev_p;
    int32_t len;
    int32_t q, r;
    int32_t k, j;
    double glue_temp;
    double cur_glue;
    scaled_t cur_g;
    uint16_t c;
    internal_font_number f;

    cur_g = 0;
    cur_glue = 0.0;
    this_box = temp_ptr;
    g_order = BOX_glue_order(this_box);
    g_sign = BOX_glue_sign(this_box);

    if (INTPAR(xetex_interword_space_shaping) > 1) {
        /*640: "Extra stuff for justifiable AAT text..." "Merge sequences of
         * words using native fonts and inter-word spaces into single
         * nodes" */

        p = BOX_list_ptr(this_box);
        prev_p = this_box + 5; /* this gets the list within the box */

        while (p != TEX_NULL) {
            if (LLIST_link(p) != TEX_NULL) {
                if (p != TEX_NULL
                    && !is_char_node(p)
                    && NODE_type(p) == WHATSIT_NODE
                    && (NODE_subtype(p) == NATIVE_WORD_NODE || NODE_subtype(p) == NATIVE_WORD_NODE_AT)
                    && font_letter_space[NATIVE_NODE_font(p)] == 0
                ) {
                    /* "got a word in an AAT font, might be the start of a run" */
                    r = p;
                    k = NATIVE_NODE_length(r);
                    q = LLIST_link(p);

                check_next:
                    /*641: "Advance `q` past ignorable nodes." This test is
                     * mostly `node_is_invisible_to_interword_space`. 641 is
                     * reused a few times here. */

                    while (q != TEX_NULL
                           && !is_char_node(q)
                           && (NODE_type(q) == PENALTY_NODE
                               || NODE_type(q) == INS_NODE
                               || NODE_type(q) == MARK_NODE
                               || NODE_type(q) == ADJUST_NODE
                               || (NODE_type(q) == WHATSIT_NODE && NODE_subtype(q) <= 4)
                    ))
                        q = LLIST_link(q);

                    if (q != TEX_NULL && !is_char_node(q)) {
                        if (NODE_type(q) == GLUE_NODE && GLUE_SPEC_shrink_order(q) == NORMAL) {
                            if (GLUE_NODE_glue_ptr(q) == font_glue[NATIVE_NODE_font(r)]) {
                                /* "Found a normal space; if the next node is
                                 * another word in the same font, we'll
                                 * merge." */

                                q = LLIST_link(q);

                                while (q != TEX_NULL && !is_char_node(q)
                                       && (NODE_type(q) == PENALTY_NODE
                                           || NODE_type(q) == INS_NODE
                                           || NODE_type(q) == MARK_NODE
                                           || NODE_type(q) == ADJUST_NODE
                                           || (NODE_type(q) == WHATSIT_NODE && NODE_subtype(q) <= 4)
                                           ))
                                    q = LLIST_link(q);

                                if (q != TEX_NULL
                                    && !is_char_node(q)
                                    && NODE_type(q) == WHATSIT_NODE
                                    && (NODE_subtype(q) == NATIVE_WORD_NODE || NODE_subtype(q) == NATIVE_WORD_NODE_AT)
                                    && (mem[q + 4].b16.s2 == mem[r + 4].b16.s2)
                                ) {
                                    p = q;
                                    k += 1 + NATIVE_NODE_length(q);
                                    q = LLIST_link(q);
                                    goto check_next;
                                }
                            } else {
                                q = LLIST_link(q);
                            }

                            if (q != TEX_NULL
                                && !is_char_node(q)
                                && NODE_type(q) == KERN_NODE
                                && NODE_subtype(q) == SPACE_ADJUSTMENT
                            ) {
                                q = LLIST_link(q);

                                while (q != TEX_NULL
                                       && !is_char_node(q)
                                       && (NODE_type(q) == PENALTY_NODE
                                           || NODE_type(q) == INS_NODE
                                           || NODE_type(q) == MARK_NODE
                                           || NODE_type(q) == ADJUST_NODE
                                           || (NODE_type(q) == WHATSIT_NODE && NODE_subtype(q) <= 4)
                                ))
                                    q = LLIST_link(q);

                                if (q != TEX_NULL
                                    && !is_char_node(q)
                                    && NODE_type(q) == WHATSIT_NODE
                                    && (NODE_subtype(q) == NATIVE_WORD_NODE || NODE_subtype(q) == NATIVE_WORD_NODE_AT)
                                    && NATIVE_NODE_font(q) == NATIVE_NODE_font(r)
                                ) {
                                    p = q;
                                    k += 1 + NATIVE_NODE_length(q);
                                    q = LLIST_link(q);
                                    goto check_next;
                                }
                            }
                            goto end_node_run;
                        }

                        if (q != TEX_NULL
                            && !is_char_node(q)
                            && NODE_type(q) == WHATSIT_NODE
                            && (NODE_subtype(q) == NATIVE_WORD_NODE || NODE_subtype(q) == NATIVE_WORD_NODE_AT)
                            && NATIVE_NODE_font(q) == NATIVE_NODE_font(r)
                        ) {
                            p = q;
                            q = LLIST_link(q);
                            goto check_next;
                        }
                    }

                end_node_run:
                    /* "Now r points to the first native_word_node of the run,
                     * and p to the last." */

                    if (p != r) {
                        if (pool_ptr + k > pool_size)
                            overflow("pool size", pool_size - init_pool_ptr);

                        k = 0;
                        q = r;

                        while (true) {
                            if (NODE_type(q) == WHATSIT_NODE) {
                                if (NODE_subtype(q) == NATIVE_WORD_NODE || NODE_subtype(q) == NATIVE_WORD_NODE_AT) {
                                    for (j = 0; j < NATIVE_NODE_length(q); j++) {
                                        str_pool[pool_ptr] = NATIVE_NODE_text(q)[j];
                                        pool_ptr++;
                                    }

                                    k += BOX_width(q);
                                }
                            } else if (NODE_type(q) == GLUE_NODE) {
                                str_pool[pool_ptr] = ' ';
                                pool_ptr++;
                                g = GLUE_NODE_glue_ptr(q);
                                k += BOX_width(g);

                                if (g_sign != NORMAL) {
                                    if (g_sign == STRETCHING) {
                                        if (GLUE_SPEC_stretch_order(g) == g_order)
                                            k += tex_round(BOX_glue_set(this_box) * GLUE_SPEC_stretch(g));
                                    } else {
                                        if (GLUE_SPEC_shrink_order(g) == g_order)
                                            k -= tex_round(BOX_glue_set(this_box) * GLUE_SPEC_shrink(g));
                                    }
                                }
                            } else if (NODE_type(q) == KERN_NODE) {
                                k += BOX_width(q);
                            }

                            if (q == p)
                                break;
                            else
                                q = LLIST_link(q);
                        }

                        q = new_native_word_node(NATIVE_NODE_font(r), cur_length());
                        NODE_subtype(q) = NODE_subtype(r);

                        for (j = 0; j < cur_length(); j++)
                            NATIVE_NODE_text(q)[j] = str_pool[str_start[str_ptr - TOO_BIG_CHAR] + j];

                        /* "Link q into the list in place of r...p" */

                        BOX_width(q) = k;
                        set_justified_native_glyphs(q);
                        LLIST_link(prev_p) = q;
                        LLIST_link(q) = LLIST_link(p);
                        LLIST_link(p) = TEX_NULL;
                        prev_p = r;
                        p = LLIST_link(r);

                        /* "Extract any 'invisible' nodes from the old list
                         * and insert them after the new node, so we don't
                         * lose them altogether. Note that the first node
                         * cannot be one of these, as we always start merging
                         * at a native_word node." */

                        while (p != TEX_NULL) {
                            if (!is_char_node(p)
                                && (NODE_type(p) == PENALTY_NODE
                                    || NODE_type(p) == INS_NODE
                                    || NODE_type(p) == MARK_NODE
                                    || NODE_type(p) == ADJUST_NODE
                                    || (NODE_type(p) == WHATSIT_NODE && NODE_subtype(p) <= 4))
                            ) {
                                LLIST_link(prev_p) = LLIST_link(p);
                                LLIST_link(p) = LLIST_link(q);
                                LLIST_link(q) = p;
                                q = p;
                            }

                            prev_p = p;
                            p = LLIST_link(p);
                        }

                        flush_node_list(r);
                        pool_ptr = str_start[str_ptr - TOO_BIG_CHAR];
                        p = q;
                    }
                }

                prev_p = p;
            }

            p = LLIST_link(p);
        }
    }

    /* ... resuming 639 ... */

    p = BOX_list_ptr(this_box);
    cur_s++;
    if (cur_s > 0)
        dvi_out(PUSH);

    if (cur_s > max_push)
        max_push = cur_s;

    save_loc = dvi_offset + dvi_ptr;
    base_line = cur_v;
    prev_p = this_box + 5; /* this is list_offset, the offset of the box list pointer */

    /*1501: "Initialize hlist_out for mixed direction typesetting" */

    temp_ptr = get_avail();
    LLIST_info(temp_ptr) = BEFORE;
    LLIST_link(temp_ptr) = LR_ptr;
    LR_ptr = temp_ptr;

    if (BOX_lr_mode(this_box) == DLIST) {
        if (cur_dir == RIGHT_TO_LEFT) {
            cur_dir = LEFT_TO_RIGHT;
            cur_h -= BOX_width(this_box);
        } else {
            BOX_lr_mode(this_box) = 0;
        }
    }

    if (cur_dir == RIGHT_TO_LEFT && BOX_lr_mode(this_box) != REVERSED) {
        /*1508: "Reverse the complete hlist and set the subtype to reversed." */
        save_h = cur_h;
        temp_ptr = p;
        p = new_kern(0);
        SYNCTEX_tag(p, MEDIUM_NODE_SIZE) = 0; /* "SyncTeX: do nothing, it is too late" */
        LLIST_link(prev_p) = p;
        cur_h = 0;
        LLIST_link(p) = reverse(this_box, TEX_NULL, &cur_g, &cur_glue);
        BOX_width(p) = -cur_h;
        cur_h = save_h;
        BOX_lr_mode(this_box) = REVERSED;
    }

    /* ... resuming 639 ... */

    left_edge = cur_h;
    synctex_hlist(this_box);

    while (p != TEX_NULL) {
        /*642: "Output node `p` for `hlist_out` and move to the next node,
        * maintaining the condition `cur_v = base_line`." ... "We ought to
        * give special care to the efficiency [here] since it belongs to TeX's
        * inner loop. When a `char_node` is encountered, we save a little time
        * by processing several nodes in succession[.] The program uses the
        * fact that `set_char_0 = 0`. */

    reswitch:
        if (is_char_node(p)) {
            if (cur_h != dvi_h) {
                movement(cur_h - dvi_h, RIGHT1);
                dvi_h = cur_h;
            }

            if (cur_v != dvi_v) {
                movement(cur_v - dvi_v, DOWN1);
                dvi_v = cur_v;
            }

            do {
                f = CHAR_NODE_font(p);
                c = CHAR_NODE_character(p);
                if (p != LIG_TRICK && font_mapping[f] != NULL)
                    c = apply_tfm_font_mapping(font_mapping[f], c);

                if (f != dvi_f) {
                    /*643: "Change font dvi_f to f" */
                    if (!font_used[f]) {
                        dvi_font_def(f);
                        font_used[f] = true;
                    }

                    if (f <= 64) {
                        dvi_out(f + FNT_NUM_0 - 1);
                    } else if (f <= 256) {
                        dvi_out(FNT1);
                        dvi_out(f - 1);
                    } else {
                        dvi_out(FNT1 + 1);
                        dvi_out((f - 1) / 256);
                        dvi_out((f - 1) % 256);
                    }

                    dvi_f = f;
                }

                if (font_ec[f] >= c) {
                    if (font_bc[f] <= c) {
                        if (FONT_CHARACTER_INFO(f, c).s3 > 0) { /* if (char_exists(orig_char_info(f)(c))) */
                            if (c >= 128)
                                dvi_out(SET1);
                            dvi_out(c);
                            cur_h += FONT_CHARACTER_WIDTH(f, c);
                        }
                    }
                }

                prev_p = LLIST_link(prev_p);
                p = LLIST_link(p);
            } while (is_char_node(p));

            synctex_current();
            dvi_h = cur_h;
        } else {
            /*644: "Output the non-char_node `p` and move to the next node" */

            switch (NODE_type(p)) {
            case HLIST_NODE:
            case VLIST_NODE:
                if (BOX_list_ptr(p) == TEX_NULL) {
                    if (NODE_type(p) == VLIST_NODE) {
                        synctex_void_vlist(p, this_box);
                    } else {
                        synctex_void_hlist(p, this_box);
                    }
                    cur_h += BOX_width(p);
                } else {
                    save_h = dvi_h;
                    save_v = dvi_v;
                    cur_v = base_line + BOX_shift_amount(p);
                    temp_ptr = p;
                    edge = cur_h + BOX_width(p);

                    if (cur_dir == RIGHT_TO_LEFT)
                        cur_h = edge;

                    if (NODE_type(p) == VLIST_NODE)
                        vlist_out();
                    else
                        hlist_out();

                    dvi_h = save_h;
                    dvi_v = save_v;
                    cur_h = edge;
                    cur_v = base_line;
                }
                break;

            case RULE_NODE:
                rule_ht = BOX_height(p);
                rule_dp = BOX_depth(p);
                rule_wd = BOX_width(p);
                goto fin_rule;
                break;

            case WHATSIT_NODE:
                /*1407: "Output the whatsit node p in an hlist" */
                switch (NODE_subtype(p)) {
                case NATIVE_WORD_NODE:
                case NATIVE_WORD_NODE_AT:
                case GLYPH_NODE:
                    if (cur_h != dvi_h) {
                        movement(cur_h - dvi_h, RIGHT1);
                        dvi_h = cur_h;
                    }

                    if (cur_v != dvi_v) {
                        movement(cur_v - dvi_v, DOWN1);
                        dvi_v = cur_v;
                    }

                    f = NATIVE_NODE_font(p);

                    if (f != dvi_f) {
                        if (!font_used[f]) {
                            dvi_font_def(f);
                            font_used[f] = true;
                        }

                        if (f <= 64) {
                            dvi_out(f + 170);
                        } else if (f <= 256) {
                            dvi_out(FNT1);
                            dvi_out(f - 1);
                        } else {
                            dvi_out(FNT1 + 1);
                            dvi_out((f - 1) / 256);
                            dvi_out((f - 1) % 256);
                        }

                        dvi_f = f;
                    }

                    if (NODE_subtype(p) == GLYPH_NODE) {
                        dvi_out(SET_GLYPHS);
                        dvi_four(BOX_width(p));
                        dvi_two(1); /* glyph count */
                        dvi_four(0); /* x offset, as fixed-point */
                        dvi_four(0); /* y offset, as fixed-point */
                        dvi_two(NATIVE_NODE_glyph(p));
                        cur_h += BOX_width(p);
                    } else {
                        if (NODE_subtype(p) == NATIVE_WORD_NODE_AT) {
                            if (NATIVE_NODE_length(p) > 0 || NATIVE_NODE_glyph_info_ptr(p) != NULL) {
                                dvi_out(SET_TEXT_AND_GLYPHS);
                                len = NATIVE_NODE_length(p);
                                dvi_two(len);

                                for (k = 0; k < len; k++)
                                            dvi_two(NATIVE_NODE_text(p)[k]);

                                len = make_xdv_glyph_array_data(p);

                                for (k = 0; k < len; k++)
                                    dvi_out(xdv_buffer[k]);
                            }
                        } else {
                            if (NATIVE_NODE_glyph_info_ptr(p) != NULL) {
                                dvi_out(SET_GLYPHS);
                                len = make_xdv_glyph_array_data(p);

                                for (k = 0; k < len; k++)
                                    dvi_out(xdv_buffer[k]);
                            }
                        }

                        cur_h += BOX_width(p);
                    }

                    dvi_h = cur_h;
                    break;

                case PIC_NODE:
                case PDF_NODE:
                    save_h = dvi_h;
                    save_v = dvi_v;
                    cur_v = base_line;
                    edge = cur_h + BOX_width(p);
                    pic_out(p);
                    dvi_h = save_h;
                    dvi_v = save_v;
                    cur_h = edge;
                    cur_v = base_line;
                    break;

                case PDF_SAVE_POS_NODE:
                    /* These magic numbers are in the original XeTeX source. */
                    pdf_last_x_pos = cur_h + 4736286L;
                    pdf_last_y_pos = cur_page_height - cur_v - 4736286L;
                    break;

                default:
                    out_what(p);
                    break;
                }
                break; /* end of WHATSIT_NODE case */

            case GLUE_NODE:
                /*647: "Move right or output leaders" */
                g = GLUE_NODE_glue_ptr(p);
                rule_wd = BOX_width(g) - cur_g;

                if (g_sign != NORMAL) {
                    if (g_sign == STRETCHING) {
                        if (GLUE_SPEC_stretch_order(g) == g_order) {
                            cur_glue += GLUE_SPEC_stretch(g);
                            glue_temp = BOX_glue_set(this_box) * cur_glue;

                            if (glue_temp > 1000000000.0)
                                glue_temp = 1000000000.0;
                            else if (glue_temp < -1000000000.0)
                                glue_temp = -1000000000.0;

                            cur_g = tex_round(glue_temp);
                        }
                    } else if (GLUE_SPEC_shrink_order(g) == g_order) {
                        cur_glue -= GLUE_SPEC_shrink(g);
                        glue_temp = BOX_glue_set(this_box) * cur_glue;

                        if (glue_temp > 1000000000.0)
                            glue_temp = 1000000000.0;
                        else if (glue_temp < -1000000000.0)
                            glue_temp = -1000000000.0;

                        cur_g = tex_round(glue_temp);
                    }
                }

                rule_wd += cur_g;

                /*1486: "Handle a glue node for mixed direction typesetting". */

                if ((g_sign == STRETCHING && GLUE_SPEC_stretch_order(g) == g_order) ||
                    (g_sign == SHRINKING && GLUE_SPEC_shrink_order(g) == g_order)) {
                    if (GLUE_SPEC_ref_count(g) == TEX_NULL)
                        free_node(g, GLUE_SPEC_SIZE);
                    else
                        GLUE_SPEC_ref_count(g)--;

                    if (NODE_subtype(p) < A_LEADERS) {
                        NODE_type(p) = KERN_NODE;
                        BOX_width(p) = rule_wd;
                    } else {
                        g = get_node(GLUE_SPEC_SIZE);
                        GLUE_SPEC_stretch_order(g) = FILLL + 1; /* "will never match" */
                        GLUE_SPEC_shrink_order(g) = FILLL + 1;
                        BOX_width(g) = rule_wd;
                        GLUE_SPEC_stretch(g) = 0;
                        GLUE_SPEC_shrink(g) = 0;
                        GLUE_NODE_glue_ptr(p) = g;
                    }
                }

                if (NODE_subtype(p) >= A_LEADERS) {
                    /*648: "Output leaders into an hlist, goto fin_rule if a
                     * rule or next_p if done." */

                    leader_box = GLUE_NODE_leader_ptr(p);

                    if (NODE_type(leader_box) == RULE_NODE) {
                        rule_ht = BOX_height(leader_box);
                        rule_dp = BOX_depth(leader_box);
                        goto fin_rule;
                    }

                    leader_wd = BOX_width(leader_box);

                    if (leader_wd > 0 && rule_wd > 0) {
                        rule_wd += 10; /* "compensate for floating-point rounding" ?? */

                        if (cur_dir == RIGHT_TO_LEFT)
                            cur_h -= 10;

                        edge = cur_h + rule_wd;
                        lx = 0;

                        /*649: "Let cur_h be the position of the first pox,
                         * and set leader_wd + lx to the spacing between
                         * corresponding parts of boxes". Additional
                         * explanator comments in XTTP. */

                        if (NODE_subtype(p) == A_LEADERS) {
                            save_h = cur_h;
                            cur_h = left_edge + leader_wd * ((cur_h - left_edge) / leader_wd);
                            if (cur_h < save_h)
                                cur_h = cur_h + leader_wd;
                        } else {
                            lq = rule_wd / leader_wd;
                            lr = rule_wd % leader_wd;

                            if (NODE_subtype(p) == C_LEADERS)
                                cur_h = cur_h + (lr / 2);
                            else {
                                lx = lr / (lq + 1);
                                cur_h = cur_h + ((lr - (lq - 1) * lx) / 2);
                            }
                        }

                        while (cur_h + leader_wd <= edge) {
                            /*650: "Output a leader box at cur_h, then advance cur_h by leader_wd + lx" */

                            cur_v = base_line + BOX_shift_amount(leader_box);

                            if (cur_v != dvi_v) {
                                movement(cur_v - dvi_v, DOWN1);
                                dvi_v = cur_v;
                            }

                            save_v = dvi_v;

                            if (cur_h != dvi_h) {
                                movement(cur_h - dvi_h, RIGHT1);
                                dvi_h = cur_h;
                            }

                            save_h = dvi_h;

                            temp_ptr = leader_box;
                            if (cur_dir == RIGHT_TO_LEFT)
                                cur_h += leader_wd;

                            outer_doing_leaders = doing_leaders;
                            doing_leaders = true;

                            if (NODE_type(leader_box) == VLIST_NODE)
                                vlist_out();
                            else
                                hlist_out();

                            doing_leaders = outer_doing_leaders;
                            dvi_v = save_v;
                            dvi_h = save_h;
                            cur_v = base_line;
                            cur_h = save_h + leader_wd + lx;
                        }

                        if (cur_dir == RIGHT_TO_LEFT)
                            cur_h = edge;
                        else
                            cur_h = edge - 10;

                        goto next_p;
                    }
                }

                goto move_past;
                break; /* end GLUE_NODE case */

            case MARGIN_KERN_NODE:
                cur_h += BOX_width(p);
                break;

            case KERN_NODE:
                synctex_kern(p, this_box);
                cur_h += BOX_width(p);
                break;

            case MATH_NODE:
                synctex_math(p, this_box);

                /* 1504: "Adjust the LR stack...; if necessary reverse and
                 * hlist segment and goto reswitch." "Breaking a paragraph
                 * into lines while TeXXeT is disabled may result in lines
                 * with unpaired math nodes. Such hlists are silently accepted
                 * in the absence of text direction directives." */

                if (odd(NODE_subtype(p))) { /* <= this is end_LR(p) */
                    if (LLIST_info(LR_ptr) == MATH_NODE_end_lr_type(p)) {
                        temp_ptr = LR_ptr;
                        LR_ptr = LLIST_link(temp_ptr);
                        LLIST_link(temp_ptr) = avail;
                        avail = temp_ptr;
                    } else {
                        if (NODE_subtype(p) > L_CODE)
                            LR_problems++;
                    }
                } else {
                    temp_ptr = get_avail();
                    LLIST_info(temp_ptr) = MATH_NODE_end_lr_type(p);
                    LLIST_link(temp_ptr) = LR_ptr;
                    LR_ptr = temp_ptr;

                    if (MATH_NODE_lr_dir(p) != cur_dir) {
                        /*1509: "Reverse an hlist segment and goto reswitch" */
                        save_h = cur_h;
                        temp_ptr = LLIST_link(p);
                        rule_wd = BOX_width(p);
                        free_node(p, MEDIUM_NODE_SIZE);
                        cur_dir = 1 - cur_dir;
                        p = new_edge(cur_dir, rule_wd);
                        LLIST_link(prev_p) = p;
                        cur_h = cur_h - left_edge + rule_wd;
                        LLIST_link(p) = reverse(this_box, new_edge(1 - cur_dir, 0), &cur_g, &cur_glue);
                        EDGE_NODE_edge_dist(p) = cur_h;
                        cur_dir = 1 - cur_dir;
                        cur_h = save_h;
                        goto reswitch;
                    }
                }

                NODE_type(p) = KERN_NODE;
                cur_h += BOX_width(p);
                break;

            case LIGATURE_NODE:
                /* 675: "Make node p look like a char_node and goto reswitch" */
                mem[LIG_TRICK] = mem[p + 1]; /* = lig_char(p) */
                LLIST_link(LIG_TRICK) = LLIST_link(p);
                p = LIG_TRICK;
                xtx_ligature_present = true;
                goto reswitch;
                break;

            /*1507: "Cases of hlist_out that arise in mixed direction text only" */
            case EDGE_NODE:
                cur_h += BOX_width(p);
                left_edge = cur_h + EDGE_NODE_edge_dist(p);
                cur_dir = NODE_subtype(p);
                break;

            default:
                break;
            }

            goto next_p;

        fin_rule:
            /*646: "Output a rule in an hlist" */
            if (rule_ht == NULL_FLAG)
                rule_ht = BOX_height(this_box);

            if (rule_dp == NULL_FLAG)
                rule_dp = BOX_depth(this_box);

            rule_ht += rule_dp;

            if (rule_ht > 0 && rule_wd > 0) {
                if (cur_h != dvi_h) {
                    movement(cur_h - dvi_h, RIGHT1);
                    dvi_h = cur_h;
                }

                cur_v = base_line + rule_dp;

                if (cur_v != dvi_v) {
                    movement(cur_v - dvi_v, DOWN1);
                    dvi_v = cur_v;
                }

                dvi_out(SET_RULE);
                dvi_four(rule_ht);
                dvi_four(rule_wd);
                cur_v = base_line;
                dvi_h += rule_wd;
            }

            /* ... resuming 644 ... */
        move_past:
            cur_h += rule_wd;
            synctex_horizontal_rule_or_glue(p, this_box);

        next_p:
            prev_p = p;
            p = LLIST_link(p);
        }
    }

    synctex_tsilh(this_box);

    /*1502: "Finish hlist_out for mixed direction typesetting" */
    /*1505: "Check for LR anomalies" */

    while (LLIST_info(LR_ptr) != BEFORE) {
        if (LLIST_info(LR_ptr) > L_CODE)
            LR_problems += 10000;

        temp_ptr = LR_ptr;
        LR_ptr = LLIST_link(temp_ptr);
        LLIST_link(temp_ptr) = avail;
        avail = temp_ptr;
    }

    temp_ptr = LR_ptr;
    LR_ptr = LLIST_link(temp_ptr);
    LLIST_link(temp_ptr) = avail;
    avail = temp_ptr;

    if (BOX_lr_mode(this_box) == DLIST)
        cur_dir = RIGHT_TO_LEFT;

    /* ... finishing 639 */

    prune_movements(save_loc);
    if (cur_s > 0)
        dvi_pop(save_loc);
    cur_s--;
}


/*651: "When vlist_out is called, its duty is to output the box represented by
 * the vlist_node pointed to by temp_ptr. The reference point of that box has
 * coordinates (cur_h, cur_v)." */
void
vlist_out(void)
{
    scaled_t left_edge;
    scaled_t top_edge;
    scaled_t save_h, save_v;
    int32_t this_box;
    glue_ord g_order;
    unsigned char g_sign;
    int32_t p;
    int32_t save_loc;
    int32_t leader_box;
    scaled_t leader_ht;
    scaled_t lx;
    bool outer_doing_leaders;
    scaled_t edge;
    double glue_temp;
    double cur_glue;
    scaled_t cur_g;
    bool upwards;
    internal_font_number f;

    cur_g = 0;
    cur_glue = 0.0;
    this_box = temp_ptr;
    g_order = BOX_glue_order(this_box);
    g_sign = BOX_glue_sign(this_box);
    p = BOX_list_ptr(this_box);
    upwards = (NODE_subtype(this_box) == 1);

    cur_s++;
    if (cur_s > 0)
        dvi_out(PUSH);

    if (cur_s > max_push)
        max_push = cur_s;

    save_loc = dvi_offset + dvi_ptr;
    left_edge = cur_h;
    synctex_vlist(this_box);

    if (upwards)
        cur_v += BOX_depth(this_box);
    else
        cur_v -= BOX_height(this_box);

    top_edge = cur_v;

    while (p != TEX_NULL) {
        /*652: "Output node p and move to the next node, maintaining the
         * condition cur_h = left_edge" */

        if (is_char_node(p))
            confusion("vlistout");
        else {
            /*653: "Output the non-char_node p" */

            switch (NODE_type(p)) {
            case HLIST_NODE:
            case VLIST_NODE:
                /*654: "Output a box in a vlist" */
                if (BOX_list_ptr(p) == TEX_NULL) {
                    if (upwards)
                        cur_v -= BOX_depth(p);
                    else
                        cur_v += BOX_height(p);

                    if (NODE_type(p) == VLIST_NODE) {
                        synctex_void_vlist(p, this_box);
                    } else {
                        synctex_void_hlist(p, this_box);
                    }

                    if (upwards)
                        cur_v -= BOX_height(p);
                    else
                        cur_v += BOX_depth(p);
                } else {
                    if (upwards)
                        cur_v -= BOX_depth(p);
                    else
                        cur_v += BOX_height(p);

                    if (cur_v != dvi_v) {
                        movement(cur_v - dvi_v, DOWN1);
                        dvi_v = cur_v;
                    }

                    save_h = dvi_h;
                    save_v = dvi_v;

                    if (cur_dir == RIGHT_TO_LEFT)
                        cur_h = left_edge - BOX_shift_amount(p);
                    else
                        cur_h = left_edge + BOX_shift_amount(p);

                    temp_ptr = p;
                    if (NODE_type(p) == VLIST_NODE)
                        vlist_out();
                    else
                        hlist_out();

                    dvi_h = save_h;
                    dvi_v = save_v;

                    if (upwards)
                        cur_v = save_v - BOX_height(p);
                    else
                        cur_v = save_v + BOX_depth(p);

                    cur_h = left_edge;
                }
                break;

            case RULE_NODE:
                rule_ht = BOX_height(p);
                rule_dp = BOX_depth(p);
                rule_wd = BOX_width(p);
                goto fin_rule;
                break;

            case WHATSIT_NODE:
                /*1403: "Output the whatsit node p in a vlist" */
                switch (NODE_subtype(p)) {
                case GLYPH_NODE:
                    cur_v = cur_v + BOX_height(p);
                    cur_h = left_edge;

                    if (cur_h != dvi_h) {
                        movement(cur_h - dvi_h, RIGHT1);
                        dvi_h = cur_h;
                    }

                    if (cur_v != dvi_v) {
                        movement(cur_v - dvi_v, DOWN1);
                        dvi_v = cur_v;
                    }

                    f = NATIVE_NODE_font(p);

                    if (f != dvi_f) {
                        /*643:*/
                        if (!font_used[f]) {
                            dvi_font_def(f);
                            font_used[f] = true;
                        }

                        if (f <= 64) {
                            dvi_out(f + 170);
                        } else if (f <= 256) {
                            dvi_out(FNT1);
                            dvi_out(f - 1);
                        } else {
                            dvi_out(FNT1 + 1);
                            dvi_out((f - 1) / 256);
                            dvi_out((f - 1) % 256);
                        }

                        dvi_f = f;
                    }

                    dvi_out(SET_GLYPHS);
                    dvi_four(0); /* width */
                    dvi_two(1); /* glyph count */
                    dvi_four(0); /* x offset as fixed-point */
                    dvi_four(0); /* y offset as fixed-point */
                    dvi_two(NATIVE_NODE_glyph(p));

                    cur_v += BOX_depth(p);
                    cur_h = left_edge;
                    break;

                case PIC_NODE:
                case PDF_NODE:
                    save_h = dvi_h;
                    save_v = dvi_v;
                    cur_v = cur_v + BOX_height(p);
                    pic_out(p);
                    dvi_h = save_h;
                    dvi_v = save_v;
                    cur_v = save_v + BOX_depth(p);
                    cur_h = left_edge;
                    break;

                case PDF_SAVE_POS_NODE:
                    /* These magic numbers are in the original XeTeX source. */
                    pdf_last_x_pos = cur_h + 4736286L;
                    pdf_last_y_pos = cur_page_height - cur_v - 4736286L;
                    break;

                default:
                    out_what(p);
                    break;
                }
                break; /* end WHATSIT_NODE case */

            case GLUE_NODE:
                /*656: "Move down or output leaders" */
                g = GLUE_NODE_glue_ptr(p);
                rule_ht = BOX_width(g) - cur_g;

                if (g_sign != NORMAL) {
                    if (g_sign == STRETCHING) {
                        if (GLUE_SPEC_stretch_order(g) == g_order) {
                            cur_glue += GLUE_SPEC_stretch(g);
                            glue_temp = BOX_glue_set(this_box) * cur_glue;

                            if (glue_temp > 1000000000.0)
                                glue_temp = 1000000000.0;
                            else if (glue_temp < -1000000000.0)
                                glue_temp = -1000000000.0;

                            cur_g = tex_round(glue_temp);
                        }
                    } else if (GLUE_SPEC_shrink_order(g) == g_order) {
                        cur_glue -= GLUE_SPEC_shrink(g);
                        glue_temp = BOX_glue_set(this_box) * cur_glue;

                        if (glue_temp > 1000000000.0)
                            glue_temp = 1000000000.0;
                        else if (glue_temp < -1000000000.0)
                            glue_temp = -1000000000.0;

                        cur_g = tex_round(glue_temp);
                    }
                }

                rule_ht += cur_g;

                if (NODE_subtype(p) >= A_LEADERS) {
                    /*657: "Output leaders in a vlist, goto fin_rule if a rule
                     * or next_p if done" */

                    leader_box = GLUE_NODE_leader_ptr(p);

                    if (NODE_type(leader_box) == RULE_NODE) {
                        rule_wd = BOX_width(leader_box);
                        rule_dp = 0;
                        goto fin_rule;
                    }

                    leader_ht = BOX_height(leader_box) + BOX_depth(leader_box);

                    if (leader_ht > 0 && rule_ht > 0) {
                        rule_ht += 10; /* "compensate for floating-point rounding" */
                        edge = cur_v + rule_ht;
                        lx = 0;

                        /*658: "Let cur_v be the position of the first box,
                         * and set leader_ht + lx to the spacing between
                         * corresponding parts of boxes" */

                        if (NODE_subtype(p) == A_LEADERS) {
                            save_v = cur_v;
                            cur_v = top_edge + leader_ht * ((cur_v - top_edge) / leader_ht);

                            if (cur_v < save_v)
                                cur_v = cur_v + leader_ht;
                        } else {
                            lq = rule_ht / leader_ht;
                            lr = rule_ht % leader_ht;

                            if (mem[p].b16.s0 == C_LEADERS)
                                cur_v = cur_v + (lr / 2);
                            else {
                                lx = lr / (lq + 1);
                                cur_v = cur_v + ((lr - (lq - 1) * lx) / 2);
                            }
                        }

                        while (cur_v + leader_ht <= edge) {
                            /*659: "Output a leader box at cur_v, then advance
                             * cur_v by leader_ht + lx". "When we reach this
                             * part of the program, cur_v indicates the top of
                             * a leader box, not its baseline." */

                            if (cur_dir == RIGHT_TO_LEFT)
                                cur_h = left_edge - BOX_shift_amount(leader_box);
                            else
                                cur_h = left_edge + BOX_shift_amount(leader_box);

                            if (cur_h != dvi_h) {
                                movement(cur_h - dvi_h, RIGHT1);
                                dvi_h = cur_h;
                            }

                            save_h = dvi_h;
                            cur_v += BOX_height(leader_box);

                            if (cur_v != dvi_v) {
                                movement(cur_v - dvi_v, DOWN1);
                                dvi_v = cur_v;
                            }

                            save_v = dvi_v;
                            temp_ptr = leader_box;
                            outer_doing_leaders = doing_leaders;
                            doing_leaders = true;

                            if (NODE_type(leader_box) == VLIST_NODE)
                                vlist_out();
                            else
                                hlist_out();

                            doing_leaders = outer_doing_leaders;
                            dvi_v = save_v;
                            dvi_h = save_h;
                            cur_h = left_edge;
                            cur_v = save_v - BOX_height(leader_box) + leader_ht + lx;
                        }

                        cur_v = edge - 10;
                        goto next_p;
                    }
                }

                goto move_past;
                break;

            case KERN_NODE:
                if (upwards)
                    cur_v -= BOX_width(p);
                else
                    cur_v += BOX_width(p);
                break;

            default:
                break;
            }

            goto next_p;

        fin_rule:
            /*655: "Output a rule in a vlist, goto next_p */
            if (rule_wd == NULL_FLAG)
                rule_wd = BOX_width(this_box);

            rule_ht += rule_dp;

            if (upwards)
                cur_v -= rule_ht;
            else
                cur_v += rule_ht;

            if (rule_ht > 0 && rule_wd > 0) {
                if (cur_dir == RIGHT_TO_LEFT)
                    cur_h -= rule_wd;

                if (cur_h != dvi_h) {
                    movement(cur_h - dvi_h, RIGHT1);
                    dvi_h = cur_h;
                }

                if (cur_v != dvi_v) {
                    movement(cur_v - dvi_v, DOWN1);
                    dvi_v = cur_v;
                }

                dvi_out(PUT_RULE);
                dvi_four(rule_ht);
                dvi_four(rule_wd);
                cur_h = left_edge;
            }

            goto next_p;

        move_past:
            if (upwards)
                cur_v -= rule_ht;
            else
                cur_v += rule_ht;
        }

    next_p:
        p = LLIST_link(p);
    }

    synctex_tsilv(this_box);
    prune_movements(save_loc);

    if (cur_s > 0)
        dvi_pop(save_loc);
    cur_s--;
}


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
