/* Copyright 2016-2018 The Tectonic Project
 * Licensed under the MIT License.
 */

/* Customizations for Tectonic:
 *
 * In semantic pagination mode, we don't actually linebreak paragraphs. We
 * just set them as one big line at its natural width no matter what. In the
 * context of this algorithm that means that try_break is always a no-op
 * unless we're trying to break at the very end of the paragraph.
 */

#include "xetex-core.h"
#include "xetex-xetexd.h"
#include "tectonic_bridge_core.h"

#define AWFUL_BAD 0x3FFFFFFF

#define VERY_LOOSE_FIT 0
#define LOOSE_FIT 1
#define DECENT_FIT 2
#define TIGHT_FIT 3

int32_t passive;
scaled_t cur_active_width[7];
scaled_t background[7];
scaled_t break_width[7];
int32_t best_place[4];
int32_t best_pl_line[4];
scaled_t disc_width;
bool no_shrink_error_yet;
int32_t cur_p;
static bool second_pass;
bool final_pass;
int32_t threshold;
int32_t minimal_demerits[4];
int32_t minimum_demerits;
int32_t easy_line;
int32_t last_special_line;
scaled_t first_width;
scaled_t second_width;
scaled_t first_indent;
scaled_t second_indent;
int32_t best_bet;
static int32_t fewest_demerits;
int32_t best_line;
static int32_t actual_looseness;
static int32_t line_diff;
small_number hn;
int32_t ha, hb;
int32_t hyf_char;
static unsigned char init_cur_lang;
int32_t l_hyf, r_hyf, init_l_hyf, init_r_hyf;
int32_t hyf_bchar;
static int32_t last_line_fill;
bool do_last_line_fit;
small_number active_node_size;
scaled_t fill_width[3];
scaled_t best_pl_short[4];
scaled_t best_pl_glue[4];


void post_line_break(bool d);
void try_break(int32_t pi, small_number break_type);
void hyphenate(void);
int32_t finite_shrink(int32_t p);
small_number reconstitute(small_number j, small_number n, int32_t bchar, int32_t hchar);
scaled_t total_pw(int32_t q, int32_t p);
int32_t find_protchar_left(int32_t l, bool d);
int32_t find_protchar_right(int32_t l, int32_t r);
void push_node(int32_t p);
int32_t pop_node(void);
UnicodeScalar get_native_usv(int32_t p, int32_t i);

/* "The active list ends where it begins" */
#define LAST_ACTIVE ACTIVE_LIST

/* Break a paragraph into lines (XTTP:843).
 *
 * d: true if we are breaking a partial paragraph preceding display math mode
 *
 * Should only be called in horizontal mode. Will leave horizontal and place
 * the output in the enclosing vertical list.
 *
 * `cur_list.head` is the non-empty hlist to be broken. `prev_graf` tells the
 * starting line number (0 unless we're continuing after display math). After
 * completion, `just_box` will point to the final box created.
 */
#if 0
void
line_break(bool d)
{
    bool auto_breaking;
    int32_t prev_p;
    int32_t q, r, s, prev_s;
    internal_font_number f;
    small_number j;
    UnicodeScalar c;
    int32_t l;
    int32_t i;
    int32_t for_end_1;

    pack_begin_line = cur_list.mode_line; /* "this is for over/underfull box messages" */

    LLIST_link(TEMP_HEAD) = LLIST_link(cur_list.head);

    /* Remove trailing space or glue if present; add infinite penalty then par_fill_skip */

    if (is_char_node(cur_list.tail)) { /* is_char_node */
        cur_list.tail = LLIST_link(cur_list.tail) = new_penalty(INF_PENALTY);
    } else if (NODE_type(cur_list.tail) != GLUE_NODE) {
        cur_list.tail = LLIST_link(cur_list.tail) = new_penalty(INF_PENALTY);
    } else {
        NODE_type(cur_list.tail) = PENALTY_NODE;
        delete_glue_ref(GLUE_NODE_glue_ptr(cur_list.tail));
        flush_node_list(GLUE_NODE_leader_ptr(cur_list.tail));
        PENALTY_NODE_penalty(cur_list.tail) = INF_PENALTY;
    }

    last_line_fill = LLIST_link(cur_list.tail) = new_param_glue(GLUE_PAR__par_fill_skip);

    /* Yet more initialization of various kinds */

    init_cur_lang = cur_list.prev_graf % 65536L;
    init_l_hyf = cur_list.prev_graf / 0x0400000;
    init_r_hyf = (cur_list.prev_graf / 65536L) % 64;

    pop_nest();

    no_shrink_error_yet = true;

    if (GLUE_SPEC_shrink_order(GLUEPAR(left_skip)) != NORMAL && GLUE_SPEC_shrink(GLUEPAR(left_skip)) != 0)
        GLUEPAR(left_skip) = finite_shrink(GLUEPAR(left_skip));

    if (GLUE_SPEC_shrink_order(GLUEPAR(right_skip)) != NORMAL && GLUE_SPEC_shrink(GLUEPAR(right_skip)) != 0)
        GLUEPAR(right_skip) = finite_shrink(GLUEPAR(right_skip));

    q = GLUEPAR(left_skip);
    r = GLUEPAR(right_skip);

    background[1] = BOX_width(q) + BOX_width(r);
    background[2] = 0;
    background[3] = 0;
    background[4] = 0;
    background[5] = 0;
    background[2 + GLUE_SPEC_stretch_order(q)] = GLUE_SPEC_stretch(q);
    background[2 + GLUE_SPEC_stretch_order(r)] += GLUE_SPEC_stretch(r);
    background[6] = GLUE_SPEC_shrink(q) + GLUE_SPEC_shrink(r);

    /* 1631: "check for special treatment of last line of paragraph" (\lastlinefit > 0) */

    do_last_line_fit = false;
    active_node_size = ACTIVE_NODE_SIZE_NORMAL;

    if (INTPAR(last_line_fit) > 0) {
        q = GLUE_NODE_glue_ptr(last_line_fill);

        if (GLUE_SPEC_stretch(q) > 0 && GLUE_SPEC_stretch_order(q) > NORMAL) {
            if (background[3] == 0 && background[4] == 0 && background[5] == 0) {
                do_last_line_fit = true;
                active_node_size = ACTIVE_NODE_SIZE_EXTENDED;
                fill_width[0] = 0;
                fill_width[1] = 0;
                fill_width[2] = 0;
                fill_width[GLUE_SPEC_stretch_order(q) - 1] = GLUE_SPEC_stretch(q);
            }
        }
    }

    minimum_demerits = AWFUL_BAD; /*863:*/
    minimal_demerits[TIGHT_FIT] = AWFUL_BAD;
    minimal_demerits[DECENT_FIT] = AWFUL_BAD;
    minimal_demerits[LOOSE_FIT] = AWFUL_BAD;
    minimal_demerits[VERY_LOOSE_FIT] = AWFUL_BAD;

    /* Prep relating to par_shape (877) */

    if (LOCAL(par_shape) == TEX_NULL) {
        if (DIMENPAR(hang_indent) == 0) {
            last_special_line = 0;
            second_width = DIMENPAR(hsize);
            second_indent = 0;
        } else { /*878:*/
            last_special_line = abs(INTPAR(hang_after));

            if (INTPAR(hang_after) < 0) {
                first_width = DIMENPAR(hsize) - abs(DIMENPAR(hang_indent));
                if (DIMENPAR(hang_indent) >= 0)
                    first_indent = DIMENPAR(hang_indent);
                else
                    first_indent = 0;
                second_width = DIMENPAR(hsize);
                second_indent = 0;
            } else {
                first_width = DIMENPAR(hsize);
                first_indent = 0;
                second_width = DIMENPAR(hsize) - abs(DIMENPAR(hang_indent));
                if (DIMENPAR(hang_indent) >= 0)
                    second_indent = DIMENPAR(hang_indent);
                else
                    second_indent = 0;
            }
        }
    } else {
        last_special_line = LLIST_info(LOCAL(par_shape)) - 1;
        /* These direct `mem` accesses are in the original WEB code */
        second_width = mem[LOCAL(par_shape) + 2 * (last_special_line + 1)].b32.s1;
        second_indent = mem[LOCAL(par_shape) + 2 * last_special_line + 1].b32.s1;
    }

    if (INTPAR(looseness) == 0)
        easy_line = last_special_line;
    else
        easy_line = MAX_HALFWORD; /*:877*/

    /* Start finding optimal breakpoints (892) */

    threshold = INTPAR(pretolerance);

    if (threshold >= 0) {
        second_pass = false;
        final_pass = false;
    } else {
        threshold = INTPAR(tolerance);
        second_pass = true;
        final_pass = (DIMENPAR(emergency_stretch) <= 0);
    }

    while (true) {
        if (threshold > INF_BAD)
            threshold = INF_BAD;

        if (second_pass) { /*920:*/
            if (trie_not_ready)
                init_trie();

            cur_lang = init_cur_lang;
            l_hyf = init_l_hyf;
            r_hyf = init_r_hyf;

            if (trie_trc[hyph_start + cur_lang] != cur_lang)
                hyph_index = 0;
            else
                hyph_index = trie_trl[hyph_start + cur_lang];
        }

        q = get_node(active_node_size); /*893:*/
        NODE_type(q) = UNHYPHENATED;
        ACTIVE_NODE_fitness(q) = DECENT_FIT;
        LLIST_link(q) = LAST_ACTIVE;
        ACTIVE_NODE_break_node(q) = TEX_NULL;
        ACTIVE_NODE_line_number(q) = cur_list.prev_graf + 1;
        ACTIVE_NODE_total_demerits(q) = 0;
        LLIST_link(ACTIVE_LIST) = q;

        if (do_last_line_fit) { /*1633:*/
            ACTIVE_NODE_shortfall(q) = 0;
            ACTIVE_NODE_glue(q) = 0;
        }

        active_width[1] = background[1];
        active_width[2] = background[2];
        active_width[3] = background[3];
        active_width[4] = background[4];
        active_width[5] = background[5];
        active_width[6] = background[6];
        passive = TEX_NULL;
        font_in_short_display = 0; /*:893*/
        cur_p = LLIST_link(TEMP_HEAD);
        auto_breaking = true;

        prev_p = global_prev_p = cur_p;
        first_p = cur_p;

        while (cur_p != TEX_NULL && LLIST_link(ACTIVE_LIST) != LAST_ACTIVE) {
            /*895: "Call try_break if cur_p is a legal breakpoint; on the
             * second pass, also try to hyphenate the next word, if cur_p is a
             * glue node; then advance cur_p to the next node of the paragraph
             * that could possibly be a legal breakpoint." */
            if (is_char_node(cur_p)) { /*896:*/
                prev_p = global_prev_p = cur_p;

                do {
                    int32_t eff_char;

                    f = CHAR_NODE_font(cur_p);
                    eff_char = effective_char(true, f, CHAR_NODE_character(cur_p));
                    active_width[1] += FONT_CHARACTER_WIDTH(f, eff_char);
                    cur_p = LLIST_link(cur_p);
                } while (is_char_node(cur_p));
            }

            switch (NODE_type(cur_p)) {
            case HLIST_NODE:
            case VLIST_NODE:
            case RULE_NODE:
                active_width[1] += BOX_width(cur_p);
                break;

            case WHATSIT_NODE:
                if (NODE_subtype(cur_p) == LANGUAGE_NODE) {
                    cur_lang = LANGUAGE_NODE_what_lang(cur_p);
                    l_hyf = LANGUAGE_NODE_what_lhm(cur_p);
                    r_hyf = LANGUAGE_NODE_what_rhm(cur_p);

                    if (trie_trc[hyph_start + cur_lang] != cur_lang)
                        hyph_index = 0;
                    else
                        hyph_index = trie_trl[hyph_start + cur_lang];
                } else if (NODE_subtype(cur_p) == NATIVE_WORD_NODE
                           || NODE_subtype(cur_p) == NATIVE_WORD_NODE_AT
                           || NODE_subtype(cur_p) == GLYPH_NODE
                           || NODE_subtype(cur_p) == PIC_NODE
                           || NODE_subtype(cur_p) == PDF_NODE) {
                    active_width[1] += BOX_width(cur_p);
                }
                break;

            case GLUE_NODE:
                if (auto_breaking) {
                    if (is_char_node(prev_p))
                        try_break(0, UNHYPHENATED);
                    else if (is_non_discardable_node(prev_p))
                        try_break(0, UNHYPHENATED);
                    else if (NODE_type(prev_p) == KERN_NODE && NODE_subtype(prev_p) != EXPLICIT)
                        try_break(0, UNHYPHENATED);
                }

                q = GLUE_NODE_glue_ptr(cur_p);

                if (GLUE_SPEC_shrink_order(q) != NORMAL && GLUE_SPEC_shrink(q) != 0)
                    q = GLUE_NODE_glue_ptr(cur_p) = finite_shrink(q);

                active_width[1] += BOX_width(q);
                active_width[2 + GLUE_SPEC_stretch_order(q)] += GLUE_SPEC_stretch(q);
                active_width[6] += GLUE_SPEC_shrink(q); /*:897*/

                if (second_pass && auto_breaking) {
                    /*924: "Try to hyphenate the following word." */
                    prev_s = cur_p;
                    s = LLIST_link(prev_s);

                    if (s != TEX_NULL) {
                        /*930: skip to node ha, or goto done1 if no hyphenation should be attempted */
                        while (true) {
                            if (is_char_node(s)) {
                                c = CHAR_NODE_character(s);
                                hf = CHAR_NODE_font(s);
                            } else if (NODE_type(s) == LIGATURE_NODE) {
                                if (LIGATURE_NODE_lig_ptr(s) == TEX_NULL)
                                    goto _continue;

                                q = LIGATURE_NODE_lig_ptr(s);
                                c = CHAR_NODE_character(q);
                                hf = CHAR_NODE_font(q);
                            } else if (NODE_type(s) == KERN_NODE && NODE_subtype(s) == NORMAL) {
                                goto _continue;
                            } else if (NODE_type(s) == MATH_NODE && NODE_subtype(s) >= L_CODE) {
                                goto _continue;
                            } else if (NODE_type(s) == WHATSIT_NODE) {
                                if (NODE_subtype(s) == NATIVE_WORD_NODE || NODE_subtype(s) == NATIVE_WORD_NODE_AT) {
                                    for (l = 0; l < NATIVE_NODE_length(s); l++) {
                                        c = get_native_usv(s, l);
                                        if (LC_CODE(c) != 0) {
                                            hf = NATIVE_NODE_font(s);
                                            prev_s = s;
                                            if (LC_CODE(c) == c || INTPAR(uc_hyph) > 0)
                                                goto done2;
                                            else
                                                goto done1;
                                        }

                                        if (c >= 65536L)
                                            l++;
                                    }
                                }

                                if (NODE_subtype(s) == LANGUAGE_NODE) {
                                    cur_lang = LANGUAGE_NODE_what_lang(s);
                                    l_hyf = LANGUAGE_NODE_what_lhm(s);
                                    r_hyf = LANGUAGE_NODE_what_rhm(s);

                                    if (trie_trc[hyph_start + cur_lang] != cur_lang)
                                        hyph_index = 0;
                                    else
                                        hyph_index = trie_trl[hyph_start + cur_lang];
                                }

                                goto _continue;
                            } else {
                                goto done1;
                            }

                            if (hyph_index == 0 || c > 255)
                                hc[0] = LC_CODE(c);
                            else if (trie_trc[hyph_index + c] != c)
                                hc[0] = 0;
                            else
                                hc[0] = trie_tro[hyph_index + c];

                            if (hc[0] != 0) {
                                if (hc[0] == c || INTPAR(uc_hyph) > 0)
                                    goto done2;
                                else
                                    goto done1;
                            }

                        _continue:
                            prev_s = s;
                            s = LLIST_link(prev_s);
                        }

                    done2:
                        hyf_char = hyphen_char[hf];
                        if (hyf_char < 0)
                            goto done1;
                        if (hyf_char > BIGGEST_CHAR)
                            goto done1;

                        ha = prev_s; /*:930*/

                        if (l_hyf + r_hyf > max_hyphenatable_length())
                            goto done1;

                        if (ha != TEX_NULL && ha < hi_mem_min && NODE_type(ha) == WHATSIT_NODE &&
                            (NODE_subtype(ha) == NATIVE_WORD_NODE || NODE_subtype(ha) == NATIVE_WORD_NODE_AT))
                        {
                            /*926: check that nodes after native_word permit hyphenation; if not, goto done1 */
                            s = LLIST_link(ha);

                            while (true) {
                                if (!is_char_node(s)) {
                                    switch (NODE_type(s)) {
                                    case LIGATURE_NODE:
                                        break;

                                    case KERN_NODE:
                                        if (NODE_subtype(s) != NORMAL)
                                            goto done6;
                                        break;

                                    case WHATSIT_NODE:
                                    case GLUE_NODE:
                                    case PENALTY_NODE:
                                    case INS_NODE:
                                    case ADJUST_NODE:
                                    case MARK_NODE:
                                        goto done6;
                                        break;

                                    default:
                                        goto done1;
                                        break;
                                    }
                                }

                                s = LLIST_link(s);
                            }

                        done6:
                            /*927: prepare a native_word_node for hyphenation.
                             * "Note that if there are chars with lccode = 0,
                             * we split them out into separate native_word
                             * nodes." */
                            hn = 0;

                        restart:
                            /* 'ha' can change in the loop, so for safety: */
                            for_end_1 = NATIVE_NODE_length(ha);

                            for (l = 0; l < for_end_1; l++) {
                                c = get_native_usv(ha, l);

                                if (hyph_index == 0 || c > 255)
                                    hc[0] = LC_CODE(c);
                                else if (trie_trc[hyph_index + c] != c)
                                    hc[0] = 0;
                                else
                                    hc[0] = trie_tro[hyph_index + c];

                                if (hc[0] == 0) {
                                    if (hn > 0) {
                                        q = new_native_word_node(hf, NATIVE_NODE_length(ha) - l);
                                        NODE_subtype(q) = NODE_subtype(ha);

                                        for (i = l; i < NATIVE_NODE_length(ha); i++)
                                            NATIVE_NODE_text(q)[i - l] = NATIVE_NODE_text(ha)[i];

                                        set_native_metrics(q, (INTPAR(xetex_use_glyph_metrics) > 0));
                                        LLIST_link(q) = LLIST_link(ha);
                                        LLIST_link(ha) = q;
                                        NATIVE_NODE_length(ha) = l;
                                        set_native_metrics(ha, (INTPAR(xetex_use_glyph_metrics) > 0));
                                        goto done3;
                                    }
                                } else if (hn == 0 && l > 0) {
                                    q = new_native_word_node(hf, NATIVE_NODE_length(ha) - l);
                                    NODE_subtype(q) = NODE_subtype(ha);

                                    for (i = l; i < NATIVE_NODE_length(ha); i++)
                                        NATIVE_NODE_text(q)[i - l] = NATIVE_NODE_text(ha)[i];

                                    set_native_metrics(q, (INTPAR(xetex_use_glyph_metrics) > 0));
                                    LLIST_link(q) = LLIST_link(ha);
                                    LLIST_link(ha) = q;
                                    NATIVE_NODE_length(ha) = l;
                                    set_native_metrics(ha, (INTPAR(xetex_use_glyph_metrics) > 0));
                                    ha = LLIST_link(ha);
                                    goto restart;
                                } else if (hn == max_hyphenatable_length()) {
                                    goto done3;
                                } else {
                                    hn++;

                                    if (c < 65536L) {
                                        hu[hn] = c;
                                        hc[hn] = hc[0];
                                    } else {
                                        hu[hn] = (c - 65536L) / 1024 + 0xD800;
                                        hc[hn] = (hc[0] - 65536L) / 1024 + 0xD800;
                                        hn++;
                                        hu[hn] = c % 1024 + 0xDC00;
                                        hc[hn] = hc[0] % 1024 + 0xDC00;
                                        l++;
                                    }

                                    hyf_bchar = TOO_BIG_CHAR;
                                }
                            }
                        } else {
                            /*931: skip to node hb, putting letters into hu and hc */
                            hn = 0;

                            while (true) {
                                if (is_char_node(s)) {
                                    if (CHAR_NODE_font(s) != hf)
                                        goto done3;

                                    hyf_bchar = CHAR_NODE_character(s);
                                    c = hyf_bchar;

                                    if (hyph_index == 0 || c > 255)
                                        hc[0] = LC_CODE(c);
                                    else if (trie_trc[hyph_index + c] != c)
                                        hc[0] = 0;
                                    else
                                        hc[0] = trie_tro[hyph_index + c];

                                    if (hc[0] == 0)
                                        goto done3;
                                    if (hc[0] > max_hyph_char)
                                        goto done3;
                                    if (hn == max_hyphenatable_length())
                                        goto done3;

                                    hb = s;
                                    hn++;
                                    hu[hn] = c;
                                    hc[hn] = hc[0];
                                    hyf_bchar = TOO_BIG_CHAR;
                                } else if (NODE_type(s) == LIGATURE_NODE) {
                                    /*932: move the characters of a ligature node to hu and hc; but goto done3
                                     * if they are not all letters. */
                                    if (LIGATURE_NODE_lig_font(s) != hf)
                                        goto done3;

                                    j = hn;
                                    q = LIGATURE_NODE_lig_ptr(s);

                                    if (q > TEX_NULL)
                                        hyf_bchar = CHAR_NODE_character(q);

                                    while (q > TEX_NULL) {
                                        c = CHAR_NODE_character(q);

                                        if (hyph_index == 0 || c > 255)
                                            hc[0] = LC_CODE(c);
                                        else if (trie_trc[hyph_index + c] != c)
                                            hc[0] = 0;
                                        else
                                            hc[0] = trie_tro[hyph_index + c];

                                        if (hc[0] == 0)
                                            goto done3;
                                        if (hc[0] > max_hyph_char)
                                            goto done3;
                                        if (j == max_hyphenatable_length())
                                            goto done3;

                                        j++;
                                        hu[j] = c;
                                        hc[j] = hc[0];
                                        q = LLIST_link(q);
                                    }

                                    hb = s;
                                    hn = j;

                                    if (odd(NODE_subtype(s)))
                                        hyf_bchar = font_bchar[hf];
                                    else
                                        hyf_bchar = TOO_BIG_CHAR; /*:932*/
                                } else if (NODE_type(s) == KERN_NODE && NODE_subtype(s) == NORMAL) {
                                    hb = s;
                                    hyf_bchar = font_bchar[hf];
                                } else {
                                    goto done3;
                                }

                                s = LLIST_link(s);
                            }

                        done3:
                            ;
                        }

                        /*933: check that the nodes following hb permit
                         * hyphenation and that at least l_hyf + r_hyf letters
                         * have been found, otherwise goto done1 */

                        if (hn < l_hyf + r_hyf)
                            goto done1;

                        while (true) {
                            if (!is_char_node(s)) {
                                switch (NODE_type(s)) {
                                case LIGATURE_NODE:
                                    break;

                                case KERN_NODE:
                                    if (NODE_subtype(s) != NORMAL)
                                        goto done4;
                                    break;

                                case WHATSIT_NODE:
                                case GLUE_NODE:
                                case PENALTY_NODE:
                                case INS_NODE:
                                case ADJUST_NODE:
                                case MARK_NODE:
                                    goto done4;
                                    break;

                                case MATH_NODE:
                                    if (NODE_subtype(s) >= L_CODE)
                                        goto done4;
                                    else
                                        goto done1;
                                    break;

                                default:
                                    goto done1;
                                    break;
                                }
                            }

                            s = LLIST_link(s);
                        }

                    done4: /*:933*/
                        hyphenate();
                    }

                done1: /*:924*/
                    ;
                }
                break; /* that was a long-ass GLUE_NODE case */

            /* ... resuming 895 ... */
            case KERN_NODE:
                if (NODE_subtype(cur_p) == EXPLICIT) {
                    if (!is_char_node(LLIST_link(cur_p)) && auto_breaking) {
                        if (NODE_type(LLIST_link(cur_p)) == GLUE_NODE)
                            try_break(0, UNHYPHENATED);
                    }
                    active_width[1] += BOX_width(cur_p);
                } else
                    active_width[1] += BOX_width(cur_p);
                break;

            case LIGATURE_NODE:
                f = LIGATURE_NODE_lig_font(cur_p);
                xtx_ligature_present = true;
                active_width[1] += FONT_CHARACTER_WIDTH(f, effective_char(true, f, LIGATURE_NODE_lig_char(cur_p)));
                break;

            case DISC_NODE:
                /*898: try to break after a discretionary fragment, then goto done5 */
                s = DISCRETIONARY_NODE_pre_break(cur_p);
                disc_width = 0;

                if (s == TEX_NULL) {
                    try_break(INTPAR(ex_hyphen_penalty), HYPHENATED);
                } else {
                    do {
                        /*899:*/
                        if (is_char_node(s)) {
                            int32_t eff_char;

                            f = CHAR_NODE_font(s);
                            eff_char = effective_char(true, f, CHAR_NODE_character(s));
                            disc_width += FONT_CHARACTER_WIDTH(f, eff_char);
                        } else {
                            switch (NODE_type(s)) {
                            case LIGATURE_NODE:
                            {
                                int32_t eff_char;

                                f = LIGATURE_NODE_lig_font(s);
                                xtx_ligature_present = true;
                                eff_char = effective_char(true, f, LIGATURE_NODE_lig_char(s));
                                disc_width += FONT_CHARACTER_WIDTH(f, eff_char);
                                break;
                            }

                            case HLIST_NODE:
                            case VLIST_NODE:
                            case RULE_NODE:
                            case KERN_NODE:
                                disc_width += BOX_width(s);
                                break;

                            case WHATSIT_NODE:
                                if (NODE_subtype(s) == NATIVE_WORD_NODE ||
                                    NODE_subtype(s) == NATIVE_WORD_NODE_AT ||
                                    NODE_subtype(s) == GLYPH_NODE ||
                                    NODE_subtype(s) == PIC_NODE ||
                                    NODE_subtype(s) == PDF_NODE)
                                    disc_width += BOX_width(s);
                                else
                                    confusion("disc3a");
                                break;

                            default:
                                confusion("disc3");
                                break;
                            }
                        }

                        s = LLIST_link(s);
                    } while (s != TEX_NULL);

                    active_width[1] += disc_width;
                    try_break(INTPAR(hyphen_penalty), HYPHENATED);
                    active_width[1] -= disc_width;
                }

                r = DISCRETIONARY_NODE_replace_count(cur_p);
                s = LLIST_link(cur_p);

                while (r > 0) {
                    if (is_char_node(s)) {
                        int32_t eff_char;

                        f = CHAR_NODE_font(s);
                        eff_char = effective_char(true, f, CHAR_NODE_character(s));
                        active_width[1] += FONT_CHARACTER_WIDTH(f, eff_char);
                    } else {
                        switch (NODE_type(s)) {
                        case LIGATURE_NODE:
                        {
                            int32_t eff_char;

                            f = LIGATURE_NODE_lig_font(s);
                            xtx_ligature_present = true;
                            eff_char = effective_char(true, f, LIGATURE_NODE_lig_char(s));
                            active_width[1] += FONT_CHARACTER_WIDTH(f, eff_char);
                            break;
                        }

                        case HLIST_NODE:
                        case VLIST_NODE:
                        case RULE_NODE:
                        case KERN_NODE:
                            active_width[1] += BOX_width(s);
                            break;

                        case WHATSIT_NODE:
                            if (NODE_subtype(s) == NATIVE_WORD_NODE ||
                                NODE_subtype(s) == NATIVE_WORD_NODE_AT ||
                                NODE_subtype(s) == GLYPH_NODE ||
                                NODE_subtype(s) == PIC_NODE ||
                                NODE_subtype(s) == PDF_NODE)
                                active_width[1] += BOX_width(s);
                            else
                                confusion("disc4a");
                            break;

                        default:
                            confusion("disc4");
                            break;
                        }
                    }

                    r--;
                    s = LLIST_link(s);
                }

                prev_p = global_prev_p = cur_p;
                cur_p = s;
                goto done5;
                break; /*:898 big DISC_NODE case */

            case MATH_NODE:
                if (NODE_subtype(cur_p) < L_CODE)
                    auto_breaking = odd(NODE_subtype(cur_p));

                if (!is_char_node(LLIST_link(cur_p)) && auto_breaking) {
                    if (NODE_type(LLIST_link(cur_p)) == GLUE_NODE)
                        try_break(0, UNHYPHENATED);
                }

                active_width[1] += BOX_width(cur_p);
                break;

            case PENALTY_NODE:
                try_break(PENALTY_NODE_penalty(cur_p), UNHYPHENATED);
                break;

            case MARK_NODE:
            case INS_NODE:
            case ADJUST_NODE:
                break;

            default:
                confusion("paragraph");
                break;
            }

            prev_p = global_prev_p = cur_p;
            cur_p = LLIST_link(cur_p);
        done5:
            ; /*:895*/
        }

        if (cur_p == TEX_NULL) {
            /*902: "Try the final line break at the end of the paragraph, and
             * goto done if the desired breakpoints have been found." */
            try_break(EJECT_PENALTY, HYPHENATED);

            if (LLIST_link(ACTIVE_LIST) != LAST_ACTIVE) { /*903:*/
                r = LLIST_link(ACTIVE_LIST);
                fewest_demerits = MAX_HALFWORD;

                do {
                    if (NODE_type(r) != DELTA_NODE) {
                        if (ACTIVE_NODE_total_demerits(r) < fewest_demerits) {
                            fewest_demerits = ACTIVE_NODE_total_demerits(r);
                            best_bet = r;
                        }
                    }

                    r = LLIST_link(r);
                } while (r != LAST_ACTIVE);

                best_line = ACTIVE_NODE_line_number(best_bet); /*:903*/

                if (INTPAR(looseness) == 0)
                    goto done;

                r = LLIST_link(ACTIVE_LIST); /*904:*/
                actual_looseness = 0;

                do {
                    if (NODE_type(r) != DELTA_NODE) {
                        line_diff = ACTIVE_NODE_line_number(r) - best_line;

                        if ((line_diff < actual_looseness && INTPAR(looseness) <= line_diff)
                            || (line_diff > actual_looseness && INTPAR(looseness) >= line_diff)) {
                            best_bet = r;
                            actual_looseness = line_diff;
                            fewest_demerits = ACTIVE_NODE_total_demerits(r);
                        } else if (line_diff == actual_looseness && ACTIVE_NODE_total_demerits(r) < fewest_demerits) {
                            best_bet = r;
                            fewest_demerits = ACTIVE_NODE_total_demerits(r);
                        }
                    }

                    r = LLIST_link(r);
                } while (r != LAST_ACTIVE);

                best_line = ACTIVE_NODE_line_number(best_bet); /*:904*/

                if (actual_looseness == INTPAR(looseness) || final_pass)
                    goto done;
            } /*:902*/
        }

        /*894: clean up the memory by removing the break nodes */

        q = LLIST_link(ACTIVE_LIST);

        while (q != LAST_ACTIVE) {
            cur_p = LLIST_link(q);

            if (NODE_type(q) == DELTA_NODE)
                free_node(q, DELTA_NODE_SIZE);
            else
                free_node(q, active_node_size);

            q = cur_p;
        }

        q = passive;

        while (q != TEX_NULL) {
            cur_p = LLIST_link(q);
            free_node(q, PASSIVE_NODE_SIZE);
            q = cur_p;
        }

        /* ... resuming 892 ... */

        if (!second_pass) {
            threshold = INTPAR(tolerance);
            second_pass = true;
            final_pass = (DIMENPAR(emergency_stretch) <= 0);
        } else {
            background[2] = background[2] + DIMENPAR(emergency_stretch);
            final_pass = true;
        }
    }

done:
    if (do_last_line_fit) { /*1641:*/
        if (ACTIVE_NODE_shortfall(best_bet) == 0) {
            do_last_line_fit = false;
        } else {
            q = new_spec(GLUE_NODE_glue_ptr(last_line_fill));
            delete_glue_ref(GLUE_NODE_glue_ptr(last_line_fill));
            BOX_width(q) += ACTIVE_NODE_shortfall(best_bet) - ACTIVE_NODE_glue(best_bet);
            GLUE_SPEC_stretch(q) = 0;
            GLUE_NODE_glue_ptr(last_line_fill) = q;
        }
    }

    post_line_break(d);

    /* Clean up by removing break nodes (894, again) */

    q = LLIST_link(ACTIVE_LIST);

    while (q != ACTIVE_LIST) {
        int32_t next = LLIST_link(q);

        if (NODE_type(q) == DELTA_NODE)
            free_node(q, DELTA_NODE_SIZE);
        else
            free_node(q, active_node_size);

        q = next;
    }

    q = passive;

    while (q != TEX_NULL) {
        int32_t next = LLIST_link(q);
        free_node(q, PASSIVE_NODE_SIZE);
        q = next;
    }

    /* All done */
    pack_begin_line = 0;
}

#endif


#if 0
/*858: "The heart of the line-breaking procedure is try_break, a subroutine
 * that tests if the current breakpoint cur_p is feasible, by running through
 * the active list to see what lines of text can be made from active nodes to
 * cur_p. If feasible breaks are possible, new break nodes are created. If
 * cur_p is too far from an active node, that node is deactivated. The
 * parameter pi to try_break is the penalty associated with a break at cur_p;
 * we have pi = eject_penalty if the break is forced, and pi = inf_penalty if
 * the break is illegal. The other parameter, break_type, is set to HYPHENATED
 * or UNHYPHENATED, depending on whether or not the current break is at a
 * disc_node. The end of a paragraph is also regarded as hyphenated; this case
 * is distinguishable by the condition cur_p = null." */
void
try_break(int32_t pi, small_number break_type)
{
    int32_t r;
    int32_t prev_r;
    int32_t old_l;
    bool no_break_yet;
    int32_t prev_prev_r = TEX_NULL;
    int32_t s;
    int32_t q;
    int32_t v;
    int32_t t;
    internal_font_number f;
    int32_t l;
    bool node_r_stays_active;
    scaled_t line_width = 0;
    unsigned char /*tight_fit */ fit_class;
    int32_t b;
    int32_t d;
    bool artificial_demerits;
    scaled_t shortfall;
    scaled_t g = 0;

    /* Tectonic: no-op except at the end of the paragraph. We know we're at
     * the very end of the paragraph when cur_p is TEX_NULL. */
    if (semantic_pagination_enabled && cur_p != TEX_NULL)
        return;

    if (abs(pi) >= INF_PENALTY) {
        if (pi > 0)
            return;

        pi = EJECT_PENALTY;
    }

    no_break_yet = true;
    prev_r = ACTIVE_LIST;
    old_l = 0;
    cur_active_width[1] = active_width[1];
    cur_active_width[2] = active_width[2];
    cur_active_width[3] = active_width[3];
    cur_active_width[4] = active_width[4];
    cur_active_width[5] = active_width[5];
    cur_active_width[6] = active_width[6];

    while (true) {
        r = LLIST_link(prev_r);

        /*861: "If node r is of type delta_node, update cur_active_width, set
         * prev_r and prev_prev_r, then goto continue" */

        if (NODE_type(r) == DELTA_NODE) {
            cur_active_width[1] += DELTA_NODE_dwidth(r);
            cur_active_width[2] += DELTA_NODE_dstretch0(r);
            cur_active_width[3] += DELTA_NODE_dstretch1(r);
            cur_active_width[4] += DELTA_NODE_dstretch2(r);
            cur_active_width[5] += DELTA_NODE_dstretch3(r);
            cur_active_width[6] += DELTA_NODE_dshrink(r);
            prev_prev_r = prev_r;
            prev_r = r;
            continue;
        }

        /*864: "If a line number class has ended, create new active nodes for
         * the best feasible breaks in that class; then return if r =
         * last_active, otherwise compute the new line_width." */

        l = ACTIVE_NODE_line_number(r);

        if (l > old_l) { /* "now we are no longer in the inner loop" */
            if (minimum_demerits < AWFUL_BAD && (old_l != easy_line || r == LAST_ACTIVE)) {
                /*865: "Create new active nodes for the best feasible breaks
                 * just found." */
                if (no_break_yet) {
                    /*866: "Compute the values of break_width". */
                    no_break_yet = false;
                    break_width[1] = background[1];
                    break_width[2] = background[2];
                    break_width[3] = background[3];
                    break_width[4] = background[4];
                    break_width[5] = background[5];
                    break_width[6] = background[6];
                    s = cur_p;

                    if (break_type > UNHYPHENATED) {
                        /*869: "Compute the discretionary break_width values" */
                        if (cur_p != TEX_NULL) {
                            t = DISCRETIONARY_NODE_replace_count(cur_p);
                            v = cur_p;
                            s = DISCRETIONARY_NODE_post_break(cur_p);

                            while (t > 0) {
                                t--;
                                v = LLIST_link(v);

                                /*870: "subtract the width of node v from break_width" */
                                if (is_char_node(v)) {
                                    int32_t eff_char;

                                    f = CHAR_NODE_font(v);
                                    eff_char = effective_char(true, f, CHAR_NODE_character(v));
                                    break_width[1] -= FONT_CHARACTER_WIDTH(f, eff_char);
                                } else
                                    switch (NODE_type(v)) {
                                    case LIGATURE_NODE:
                                    {
                                        int32_t eff_char;

                                        f = LIGATURE_NODE_lig_font(v);
                                        xtx_ligature_present = true;
                                        eff_char = effective_char(true, f, LIGATURE_NODE_lig_char(v));
                                        break_width[1] -= FONT_CHARACTER_WIDTH(f, eff_char);
                                        break;
                                    }

                                    case HLIST_NODE:
                                    case VLIST_NODE:
                                    case RULE_NODE:
                                    case KERN_NODE:
                                        break_width[1] -= BOX_width(v);
                                        break;

                                    case WHATSIT_NODE:
                                        if (NODE_subtype(v) == NATIVE_WORD_NODE
                                            || NODE_subtype(v) == NATIVE_WORD_NODE_AT
                                            || NODE_subtype(v) == GLYPH_NODE
                                            || NODE_subtype(v) == PIC_NODE
                                            || NODE_subtype(v) == PDF_NODE)
                                            break_width[1] -= BOX_width(v);
                                        else
                                            confusion("disc1a");
                                        break;

                                    default:
                                        confusion("disc1");
                                        break;
                                    }
                            }

                            /*871: "add the width of node s to break_width" */
                            while (s != TEX_NULL) {
                                if (is_char_node(s)) {
                                    int32_t eff_char;

                                    f = CHAR_NODE_font(s);
                                    eff_char = effective_char(true, f, CHAR_NODE_character(s));
                                    break_width[1] += FONT_CHARACTER_WIDTH(f, eff_char);;
                                } else
                                    switch (mem[s].b16.s1) {
                                    case LIGATURE_NODE:
                                    {
                                        int32_t eff_char;

                                        f = LIGATURE_NODE_lig_font(s);
                                        xtx_ligature_present = true;
                                        eff_char = effective_char(true, f, LIGATURE_NODE_lig_char(s));
                                        break_width[1] += FONT_CHARACTER_WIDTH(f, eff_char);
                                        break;
                                    }

                                    case HLIST_NODE:
                                    case VLIST_NODE:
                                    case RULE_NODE:
                                    case KERN_NODE:
                                        break_width[1] += BOX_width(s);
                                        break;

                                    case WHATSIT_NODE:
                                        if (NODE_subtype(s) == NATIVE_WORD_NODE
                                            || NODE_subtype(s) == NATIVE_WORD_NODE_AT
                                            || NODE_subtype(s) == GLYPH_NODE
                                            || NODE_subtype(s) == PIC_NODE
                                            || NODE_subtype(s) == PDF_NODE)
                                            break_width[1] += BOX_width(s);
                                        else
                                            confusion("disc2a");
                                        break;

                                    default:
                                        confusion("disc2");
                                        break;
                                    }

                                s = LLIST_link(s);
                            }

                            break_width[1] += disc_width;
                            if (DISCRETIONARY_NODE_post_break(cur_p) == TEX_NULL)
                                s = LLIST_link(v);
                        }
                    }

                    while (s != TEX_NULL) {
                        if (is_char_node(s))
                            goto done;

                        switch (NODE_type(s)) {
                        case GLUE_NODE:
                            v = GLUE_NODE_glue_ptr(s);
                            break_width[1] -= BOX_width(v);
                            break_width[2 + GLUE_SPEC_stretch_order(v)] -= GLUE_SPEC_stretch(v);
                            break_width[6] -= GLUE_SPEC_shrink(v);
                            break;

                        case PENALTY_NODE:
                            break;

                        case MATH_NODE:
                            break_width[1] -= BOX_width(s);
                            break;

                        case KERN_NODE:
                            if (NODE_subtype(s) != EXPLICIT)
                                goto done;
                            break_width[1] -= BOX_width(s);
                            break;

                        default:
                            goto done;
                            break;
                        }

                        s = LLIST_link(s);
                    }

                done:
                    ;
                }

                /*872: "Insert a delta node to prepare for breaks at cur_p" */
                if (NODE_type(prev_r) == DELTA_NODE) {
                    DELTA_NODE_dwidth(prev_r) += -cur_active_width[1] + break_width[1];
                    DELTA_NODE_dstretch0(prev_r) += -cur_active_width[2] + break_width[2];
                    DELTA_NODE_dstretch1(prev_r) += -cur_active_width[3] + break_width[3];
                    DELTA_NODE_dstretch2(prev_r) += -cur_active_width[4] + break_width[4];
                    DELTA_NODE_dstretch3(prev_r) += -cur_active_width[5] + break_width[5];
                    DELTA_NODE_dshrink(prev_r) += -cur_active_width[6] + break_width[6];
                } else if (prev_r == ACTIVE_LIST) {
                    active_width[1] = break_width[1];
                    active_width[2] = break_width[2];
                    active_width[3] = break_width[3];
                    active_width[4] = break_width[4];
                    active_width[5] = break_width[5];
                    active_width[6] = break_width[6];
                } else {
                    q = get_node(DELTA_NODE_SIZE);
                    LLIST_link(q) = r;
                    NODE_type(q) = DELTA_NODE;
                    NODE_subtype(q) = 0; /* this is unused */
                    DELTA_NODE_dwidth(q) = break_width[1] - cur_active_width[1];
                    DELTA_NODE_dstretch0(q) = break_width[2] - cur_active_width[2];
                    DELTA_NODE_dstretch1(q) = break_width[3] - cur_active_width[3];
                    DELTA_NODE_dstretch2(q) = break_width[4] - cur_active_width[4];
                    DELTA_NODE_dstretch3(q) = break_width[5] - cur_active_width[5];
                    DELTA_NODE_dshrink(q) = break_width[6] - cur_active_width[6];
                    LLIST_link(prev_r) = q;
                    prev_prev_r = prev_r;
                    prev_r = q;
                }

                /* ... resuming 865 ... */
                if (abs(INTPAR(adj_demerits)) >= MAX_HALFWORD - minimum_demerits)
                    minimum_demerits = AWFUL_BAD - 1;
                else
                    minimum_demerits = minimum_demerits + abs(INTPAR(adj_demerits));

                for (fit_class = VERY_LOOSE_FIT; fit_class <= TIGHT_FIT; fit_class++) {
                    if (minimal_demerits[fit_class] <= minimum_demerits) {
                        /*874: "Insert a new active node from best_place[fit_class] to cur_p" */
                        q = get_node(PASSIVE_NODE_SIZE);
                        LLIST_link(q) = passive;
                        passive = q;
                        PASSIVE_NODE_cur_break(q) = cur_p;
                        PASSIVE_NODE_prev_break(q) = best_place[fit_class];

                        q = get_node(active_node_size);
                        ACTIVE_NODE_break_node(q) = passive;
                        ACTIVE_NODE_line_number(q) = best_pl_line[fit_class] + 1;
                        ACTIVE_NODE_fitness(q) = fit_class;
                        NODE_type(q) = break_type;
                        ACTIVE_NODE_total_demerits(q) = minimal_demerits[fit_class];

                        if (do_last_line_fit) {     /*1639: */
                            ACTIVE_NODE_shortfall(q) = best_pl_short[fit_class];
                            ACTIVE_NODE_glue(q) = best_pl_glue[fit_class];
                        }

                        LLIST_link(q) = r;
                        LLIST_link(prev_r) = q;
                        prev_r = q;
                    }

                    minimal_demerits[fit_class] = MAX_HALFWORD;
                }

                minimum_demerits = MAX_HALFWORD;

                /*873: "Insert a delta node to prepare for the next active node" */
                if (r != LAST_ACTIVE) {
                    q = get_node(DELTA_NODE_SIZE);
                    LLIST_link(q) = r;
                    NODE_type(q) = DELTA_NODE;
                    NODE_subtype(q) = 0; /* subtype is not used */
                    DELTA_NODE_dwidth(q) = cur_active_width[1] - break_width[1];
                    DELTA_NODE_dstretch0(q) = cur_active_width[2] - break_width[2];
                    DELTA_NODE_dstretch1(q) = cur_active_width[3] - break_width[3];
                    DELTA_NODE_dstretch2(q) = cur_active_width[4] - break_width[4];
                    DELTA_NODE_dstretch3(q) = cur_active_width[5] - break_width[5];
                    DELTA_NODE_dshrink(q) = cur_active_width[6] - break_width[6];
                    LLIST_link(prev_r) = q;
                    prev_prev_r = prev_r;
                    prev_r = q;
                }
            }

            /* ... resuming 864 ... */
            if (r == LAST_ACTIVE)
                return;

            /*879: "Compute the new line width" */
            if (l > easy_line) {
                line_width = second_width;
                old_l = MAX_HALFWORD - 1;
            } else {
                old_l = l;

                if (l > last_special_line)
                    line_width = second_width;
                else if (LOCAL(par_shape) == TEX_NULL)
                    line_width = first_width;
                else
                    line_width = mem[LOCAL(par_shape) + 2 * l].b32.s1; /* this mem access is in the WEB */
            }
        }

        /*880: "Consider the demerits for a line from r to cur_p; deactivate
         * node r if it should no longer be active; then goto continue if a
         * line from r to cur_p is infeasible; otherwise record a new feasible
         * break" */

        /* Tectonic: if we got here, we must be "considering" a linebreak
         * at the very end of the paragraph. How amazing, it's a perfect fit!
         */

        if (semantic_pagination_enabled) {
            line_width = cur_active_width[1];
            artificial_demerits = true;
            shortfall = 0;
        } else {
            artificial_demerits = false;
            shortfall = line_width - cur_active_width[1];

            if (INTPAR(xetex_protrude_chars) > 1)
                shortfall = shortfall + total_pw(r, cur_p);
        }

        if (shortfall > 0) {
            /*881: "Set the value of b to the badness for stretching the line,
             * and compute the corresponding fit_class" */
            if (cur_active_width[3] != 0 || cur_active_width[4] != 0 || cur_active_width[5] != 0) {
                if (do_last_line_fit) {
                    if (cur_p == TEX_NULL) {
                        /*1634: "Perform computations for the last line and goto found" */
                        if (ACTIVE_NODE_shortfall(r) == 0 || ACTIVE_NODE_glue(r) <= 0)
                            goto not_found;

                        if (cur_active_width[3] != fill_width[0] || cur_active_width[4] != fill_width[1]
                            || cur_active_width[5] != fill_width[2])
                            goto not_found;

                        if (ACTIVE_NODE_shortfall(r) > 0)
                            g = cur_active_width[2];
                        else
                            g = cur_active_width[6];

                        if (g <= 0)
                            goto not_found;

                        arith_error = false;
                        g = fract(g, ACTIVE_NODE_shortfall(r), ACTIVE_NODE_glue(r), MAX_HALFWORD);
                        if (INTPAR(last_line_fit) < 1000)
                            g = fract(g, INTPAR(last_line_fit), 1000, MAX_HALFWORD);

                        if (arith_error) {
                            if (ACTIVE_NODE_shortfall(r) > 0)
                                g = MAX_HALFWORD;
                            else
                                g = -MAX_HALFWORD;
                        }

                        if (g > 0) {
                            /*1635: "Set the value of b to the badness of the
                            * last line for stretching, compute the
                            * corresponding fit_class, and goto found" */
                            if (g > shortfall)
                                g = shortfall;

                            if (g > 7230584L) { /* XXX: magic number in original WEB code */
                                if (cur_active_width[2] < 1663497L) { /* XXX: magic number in original WEB code */
                                    b = INF_BAD;
                                    fit_class = VERY_LOOSE_FIT;
                                    goto found;
                                }
                            }

                            b = badness(g, cur_active_width[2]);

                            if (b > 12) {
                                if (b > 99)
                                    fit_class = VERY_LOOSE_FIT;
                                else
                                    fit_class = LOOSE_FIT;
                            } else {
                                fit_class = DECENT_FIT;
                            }

                            goto found;
                        } else if (g < 0) {
                            /*1636: "Set the value of b to the badness of the
                            * last line for shrinking, compute the
                            * corresponding fit_class, and goto found" */
                            if (-g > cur_active_width[6])
                                g = -cur_active_width[6];

                            b = badness(-g, cur_active_width[6]);
                            if (b > 12) /* XXX hardcoded in WEB */
                                fit_class = TIGHT_FIT;
                            else
                                fit_class = DECENT_FIT;

                            goto found;
                        }

                    not_found:
                        ;
                    }

                    shortfall = 0;
                }

                b = 0;
                fit_class = DECENT_FIT;
            } else {
                if (shortfall > 7230584L) { /* XXX: magic number in original WEB code */
                    if (cur_active_width[2] < 1663497L) { /* XXX: magic number in original WEB code */
                        b = INF_BAD;
                        fit_class = VERY_LOOSE_FIT;
                        goto done1;
                    }
                }

                b = badness(shortfall, cur_active_width[2]);
                if (b > 12) {
                    if (b > 99)
                        fit_class = VERY_LOOSE_FIT;
                    else
                        fit_class = LOOSE_FIT;
                } else {
                    fit_class = DECENT_FIT;
                }

            done1:
                ;
            }
        } else {
            /*882: "Set the value of b to the badness for shrinking the line,
            * and compute the corresponding fit_class" */
            if (-shortfall > cur_active_width[6])
                b = (INF_BAD + 1);
            else
                b = badness(-shortfall, cur_active_width[6]);

            if (b > 12)
                fit_class = TIGHT_FIT;
            else
                fit_class = DECENT_FIT;
        }

        if (do_last_line_fit) {
            /*1637: "Adjust the additional data for last line" */
            if (cur_p == TEX_NULL)
                shortfall = 0;

            if (shortfall > 0)
                g = cur_active_width[2];
            else if (shortfall < 0)
                g = cur_active_width[6];
            else
                g = 0;
        }

    found:
        if (b > INF_BAD || pi == EJECT_PENALTY) {
            /*883: "Prepare to deactivate node r, and goto deactivate unless
             * there is a reason to consider lines of text from r to cur_p" */
            if (final_pass && minimum_demerits == AWFUL_BAD && LLIST_link(r) == LAST_ACTIVE && prev_r == ACTIVE_LIST)
                artificial_demerits = true;
            else if (b > threshold)
                goto deactivate;

            node_r_stays_active = false;
        } else {
            prev_r = r;
            if (b > threshold)
                continue;
            node_r_stays_active = true;
        }

        if (artificial_demerits) {
            d = 0;
        } else {
            /*888: "Compute the demerits, d, from r to cur_p" */
            d = INTPAR(line_penalty) + b;
            if (abs(d) >= 10000)
                d = 100000000L; /* algorithmic constant */
            else
                d = d * d;

            if (pi != 0) {
                if (pi > 0)
                    d = d + pi * pi;
                else if (pi > EJECT_PENALTY)
                    d = d - pi * pi;
            }

            if (break_type == HYPHENATED && NODE_type(r) == HYPHENATED) {
                if (cur_p != TEX_NULL)
                    d = d + INTPAR(double_hyphen_demerits);
                else
                    d = d + INTPAR(final_hyphen_demerits);
            }

            if (abs(fit_class - ACTIVE_NODE_fitness(r)) > 1)
                d = d + INTPAR(adj_demerits);
        }

        /* resuming 884: */
        d = d + ACTIVE_NODE_total_demerits(r);

        if (d <= minimal_demerits[fit_class]) {
            minimal_demerits[fit_class] = d;
            best_place[fit_class] = ACTIVE_NODE_break_node(r);
            best_pl_line[fit_class] = l;

            if (do_last_line_fit) { /*1638:*/
                best_pl_short[fit_class] = shortfall;
                best_pl_glue[fit_class] = g;
            }

            if (d < minimum_demerits)
                minimum_demerits = d;
        }

        if (node_r_stays_active)
            continue;

    deactivate:
        /*889: "Deactivate node r" */
        LLIST_link(prev_r) = LLIST_link(r);
        free_node(r, active_node_size);

        if (prev_r == ACTIVE_LIST) {
            /*890: "Update the active widths, since the first active node has been deleted" */
            r = LLIST_link(ACTIVE_LIST);

            if (NODE_type(r) == DELTA_NODE) {
                active_width[1] += DELTA_NODE_dwidth(r);
                active_width[2] += DELTA_NODE_dstretch0(r);
                active_width[3] += DELTA_NODE_dstretch1(r);
                active_width[4] += DELTA_NODE_dstretch2(r);
                active_width[5] += DELTA_NODE_dstretch3(r);
                active_width[6] += DELTA_NODE_dshrink(r);
                cur_active_width[1] = active_width[1];
                cur_active_width[2] = active_width[2];
                cur_active_width[3] = active_width[3];
                cur_active_width[4] = active_width[4];
                cur_active_width[5] = active_width[5];
                cur_active_width[6] = active_width[6];
                LLIST_link(ACTIVE_LIST) = LLIST_link(r);
                free_node(r, DELTA_NODE_SIZE);
            }
        } else if (NODE_type(prev_r) == DELTA_NODE) {
            r = LLIST_link(prev_r);

            if (r == LAST_ACTIVE) {
                cur_active_width[1] -= DELTA_NODE_dwidth(prev_r);
                cur_active_width[2] -= DELTA_NODE_dstretch0(prev_r);
                cur_active_width[3] -= DELTA_NODE_dstretch1(prev_r);
                cur_active_width[4] -= DELTA_NODE_dstretch2(prev_r);
                cur_active_width[5] -= DELTA_NODE_dstretch3(prev_r);
                cur_active_width[6] -= DELTA_NODE_dshrink(prev_r);
                LLIST_link(prev_prev_r) = LAST_ACTIVE;
                free_node(prev_r, DELTA_NODE_SIZE);
                prev_r = prev_prev_r;
            } else if (NODE_type(r) == DELTA_NODE) {
                cur_active_width[1] += DELTA_NODE_dwidth(r);
                cur_active_width[2] += DELTA_NODE_dstretch0(r);
                cur_active_width[3] += DELTA_NODE_dstretch1(r);
                cur_active_width[4] += DELTA_NODE_dstretch2(r);
                cur_active_width[5] += DELTA_NODE_dstretch3(r);
                cur_active_width[6] += DELTA_NODE_dshrink(r);
                DELTA_NODE_dwidth(prev_r) += DELTA_NODE_dwidth(r);
                DELTA_NODE_dstretch0(prev_r) += DELTA_NODE_dstretch0(r);
                DELTA_NODE_dstretch1(prev_r) += DELTA_NODE_dstretch1(r);
                DELTA_NODE_dstretch2(prev_r) += DELTA_NODE_dstretch2(r);
                DELTA_NODE_dstretch3(prev_r) += DELTA_NODE_dstretch3(r);
                DELTA_NODE_dshrink(prev_r) += DELTA_NODE_dshrink(r);
                LLIST_link(prev_r) = LLIST_link(r);
                free_node(r, DELTA_NODE_SIZE);
            }
        }
    }
}
#endif
