/* tectonic/xetex-scaledmath.c: low-level math functions
   Copyright 2017 The Tectonic Project
   Licensed under the MIT License.
*/

#include "xetex-core.h"
#include "xetex-xetexd.h"

/*
 * The following functions used to live in this file and have been moved
 * to Rust (`crates/engine_xetex/src/scaled_math.rs`):
 *
 *   tex_round, half, mult_and_add, x_over_n, xn_over_d, round_xn_over_d,
 *   make_frac, take_frac, ab_vs_cd, new_randoms, init_randoms,
 *   unif_rand, norm_rand
 *
 * Their prototypes are still declared in xetex-xetexd.h so C callers
 * keep working; the linker resolves the definitions to the Rust
 * #[no_mangle] implementations. See plan/06-workflow.md §"Step 5".
 */

int32_t
m_log(int32_t x)
{
    int32_t y, z;
    int32_t k;

    if (x <= 0) { /*125: */
        error_here_with_diagnostic("Logarithm of ");
        print_scaled(x);
        print_cstr(" has been replaced by 0");
        capture_to_diagnostic(NULL);
        help_ptr = 2;
        help_line[1] = "Since I don't take logs of non-positive numbers,";
        help_line[0] = "I'm zeroing this one. Proceed, with fingers crossed.";
        error();
        return 0;
    } else {
        y = 1302456860L;
        z = 6581195L;

        while (x < 0x40000000) {
            x = x + x;
            y = y - 93032639L;
            z = z - 48782L;
        }

        y = y + (z / 65536L);
        k = 2;

        while (x > 0x40000004) { /*124: */
            z = ((x - 1) / two_to_the[k]) + 1;

            while (x < 0x40000000 + z) {
                z = (z + 1) / 2;
                k = k + 1;
            }

            y = y + spec_log[k];
            x = x - z;
        }

        return y / 8;
    }
}

