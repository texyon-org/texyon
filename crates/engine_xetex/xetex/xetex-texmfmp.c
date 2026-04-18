/* texmfmp.c: Hand-coded routines for TeX or Metafont in C.  Originally
   written by Tim Morgan, drawing from other Unix ports of TeX.  This is
   a collection of miscellany, everything that's easier (or only
   possible) to do in C.

   This file is public domain.  */

#include "xetex-core.h"
#include "xetex-xetexd.h"
#include "xetex-ext.h"

#include <stdio.h> /* For s(n)printf */
#include <time.h> /* For `struct tm'.  Moved here for Visual Studio 2005.  */
#ifndef _MSC_VER
#include <sys/time.h>
#define HAVE_GETTIMEOFDAY
#endif

/* last_source_name and last_lineno moved to Rust statics in src/texmfmp.rs. */

#define check_nprintf(size_get, size_want) \
    if ((unsigned)(size_get) >= (unsigned)(size_want)) \
        _tt_abort ("snprintf failed: file %s, line %d", __FILE__, __LINE__);

/* makepdftime, init_start_time, getcreationdate, getfilemoddate translated to Rust
   (src/texmfmp.rs). start_time_str moved to Rust static START_TIME_STR. */

/* get_date_and_time and get_seconds_and_micros translated to Rust (src/texmfmp.rs). */

/* getfilesize and getfiledump translated to Rust (src/texmfmp.rs). */


/* checkpool_pointer and maketexstring translated to Rust (src/texmfmp.rs). */


/* gettexstring translated to Rust (src/texmfmp.rs). */


/* compare_paths, is_new_source, remember_source_info translated to Rust (src/texmfmp.rs).
   last_source_name and last_lineno are now Rust statics (LAST_SOURCE_NAME, LAST_LINENO). */


/* make_src_special translated to Rust (src/texmfmp.rs). */

/* convertStringToHexString and getmd5sum translated to Rust (src/texmfmp.rs). */
