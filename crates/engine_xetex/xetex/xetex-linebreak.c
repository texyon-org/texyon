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

/* The line-breaker scratch globals formerly defined here (passive,
   cur_active_width, background, break_width, best_place, best_pl_line,
   disc_width, cur_p, final_pass, threshold, minimal_demerits,
   minimum_demerits, easy_line, last_special_line, first_width, second_width,
   first_indent, second_indent, best_bet, best_line, hn, ha, hb,
   do_last_line_fit, active_node_size, fill_width, best_pl_short, best_pl_glue)
   moved to EngineState.linebreak (LinebreakState). */
