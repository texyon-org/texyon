/* tectonic/xetex-scaledmath.c: low-level math functions
   Copyright 2017 The Tectonic Project
   Licensed under the MIT License.
*/

#include "xetex-core.h"
#include "xetex-xetexd.h"

/*
 * All functions in this file have been moved to Rust
 * (`crates/engine_xetex/src/scaled_math.rs`):
 *
 *   tex_round, half, mult_and_add, x_over_n, xn_over_d, round_xn_over_d,
 *   make_frac, take_frac, ab_vs_cd, new_randoms, init_randoms,
 *   unif_rand, norm_rand, m_log
 *
 * Their prototypes are still declared in xetex-xetexd.h so C callers
 * keep working; the linker resolves the definitions to the Rust
 * #[no_mangle] implementations. See plan/06-workflow.md §"Step 5".
 */
