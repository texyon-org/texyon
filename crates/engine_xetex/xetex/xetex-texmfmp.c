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

/* minimum size for time_str is 24: "D:YYYYmmddHHMMSS+HH'MM'" */
#define TIME_STR_SIZE 30
static char start_time_str[TIME_STR_SIZE];

static void
makepdftime(time_t t, char *time_str, bool utc)
{
    struct tm lt, gmt;
    size_t size;
    int i, off, off_hours, off_mins;

    /* get the time */
    if (utc) {
        lt = *gmtime(&t);
    }
    else {
        lt = *localtime(&t);
    }
    size = strftime(time_str, TIME_STR_SIZE, "D:%Y%m%d%H%M%S", &lt);
    /* expected format: "YYYYmmddHHMMSS" */
    if (size == 0) {
        /* unexpected, contents of time_str is undefined */
        time_str[0] = '\0';
        return;
    }

    /* correction for seconds: %S can be in range 00..61,
       the PDF reference expects 00..59,
       therefore we map "60" and "61" to "59" */
    if (time_str[14] == '6') {
        time_str[14] = '5';
        time_str[15] = '9';
        time_str[16] = '\0';    /* for safety */
    }

    /* get the time zone offset */
    gmt = *gmtime(&t);

    /* this calculation method was found in exim's tod.c */
    off = 60 * (lt.tm_hour - gmt.tm_hour) + lt.tm_min - gmt.tm_min;
    if (lt.tm_year != gmt.tm_year) {
        off += (lt.tm_year > gmt.tm_year) ? 1440 : -1440;
    } else if (lt.tm_yday != gmt.tm_yday) {
        off += (lt.tm_yday > gmt.tm_yday) ? 1440 : -1440;
    }

    if (off == 0) {
        time_str[size++] = 'Z';
        time_str[size] = 0;
    } else {
        off_hours = off / 60;
        off_mins = abs(off - off_hours * 60);
        i = snprintf(&time_str[size], 9, "%+03d'%02d'", off_hours, off_mins);
        check_nprintf(i, 9);
    }
}


void
init_start_time(time_t source_date_epoch)
{
  makepdftime(source_date_epoch, start_time_str, /* utc= */true);
}


void
getcreationdate(void)
{
    size_t len;
    int i;

    /* init_start_time(); -- Tectonic: not needed*/
    /* put creation date on top of string pool and update pool_ptr */
    len = strlen(start_time_str);

    /* In e-pTeX, "init len => call init_start_time()" (as pdftexdir/utils.c)
       yields  unintentional output. */

    if ((unsigned) (pool_ptr + len) >= (unsigned) (pool_size)) {
        pool_ptr = pool_size;
        /* error by str_toks that calls str_room(1) */
        return;
    }

    for (i = 0; i < len; i++)
        str_pool[pool_ptr++] = (uint16_t)start_time_str[i];
}


/* get_date_and_time and get_seconds_and_micros translated to Rust (src/texmfmp.rs). */


/* Given a file name stored in the string pool, insert into the string pool text
 * giving its modification date in PDF-style format. */
void
getfilemoddate(str_number s)
{
  char *name;
  time_t mtime;
  size_t text_len;
  rust_input_handle_t handle;
  char buf[20];

  name = gettexstring(s);
  handle = ttstub_input_open(name, TTBC_FILE_FORMAT_TEX, 0);
  free(name);

  if (handle == INVALID_HANDLE)
    return; /* => evaluate to the empty string; intentional */

  mtime = ttstub_input_get_mtime(handle);
  ttstub_input_close(handle);

  makepdftime(mtime, buf, /* utc= */true);
  text_len = strlen(buf);

  if ((unsigned) (pool_ptr + text_len) >= (unsigned) pool_size) {
    pool_ptr = pool_size;
    /* error by str_toks that calls str_room(1) */
  } else {
    int i;

    for (i = 0; i < text_len; i++)
      str_pool[pool_ptr++] = (uint16_t) buf[i];
  }
}

/* getfilesize and getfiledump translated to Rust (src/texmfmp.rs). */


/* checkpool_pointer and maketexstring translated to Rust (src/texmfmp.rs). */


/* gettexstring translated to Rust (src/texmfmp.rs). */


/* compare_paths, is_new_source, remember_source_info translated to Rust (src/texmfmp.rs).
   last_source_name and last_lineno are now Rust statics (LAST_SOURCE_NAME, LAST_LINENO). */


/* make_src_special translated to Rust (src/texmfmp.rs). */

/* convertStringToHexString and getmd5sum translated to Rust (src/texmfmp.rs). */
