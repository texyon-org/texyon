/* tectonic/output.c -- functions related to outputting messages
 * Copyright 2016 the Tectonic Project
 * Licensed under the MIT License.
*/

#include "xetex-core.h"
#include "xetex-xetexd.h"
#include "xetex-synctex.h"
#include "tectonic_bridge_core.h"

/* Storage for current_diagnostic — accessed from Rust via extern "C". */
ttbc_diagnostic_t *current_diagnostic = 0;

/* All functions in this file have been translated to Rust (src/output.rs).
 * Only the current_diagnostic variable definition remains here. */
