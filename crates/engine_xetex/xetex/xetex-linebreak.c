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

int32_t passive;
scaled_t cur_active_width[7];
scaled_t background[7];
scaled_t break_width[7];
int32_t best_place[4];
int32_t best_pl_line[4];
scaled_t disc_width;
int32_t cur_p;
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
int32_t best_line;
small_number hn;
int32_t ha, hb;
bool do_last_line_fit;
small_number active_node_size;
scaled_t fill_width[3];
scaled_t best_pl_short[4];
scaled_t best_pl_glue[4];
