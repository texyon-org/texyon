/* tectonic/xetex-io.c: low-level input/output functions tied to the XeTeX engine
   Copyright 2016-2019 The Tectonic Project
   Licensed under the MIT License.
*/

#include "xetex-core.h"
#include "teckit-c-Engine.h"
#include "xetex-swap.h"
#include "xetex-xetexd.h"

#include <stdio.h>
#include <unicode/ucnv.h>

/* name_of_input_file, abspath_of_input_file, tt_xetex_open_input
   translated to Rust (src/io.rs). */

/* tables/values used in UTF-8 interpretation -
   code is based on ConvertUTF.[ch] sample code
   published by the Unicode consortium */
const uint32_t
offsetsFromUTF8[6] = {
    0x00000000UL,
    0x00003080UL,
    0x000E2080UL,
    0x03C82080UL,
    0xFA082080UL,
    0x82082080UL
};

const uint8_t
bytesFromUTF8[256] = {
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
    2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2, 3,3,3,3,3,3,3,3,4,4,4,4,5,5,5,5
};

const uint8_t
firstByteMark[7] = {
    0x00, 0x00, 0xC0, 0xE0, 0xF0, 0xF8, 0xFC
};

/* set_input_file_encoding: translated to Rust (src/io.rs) */


/* u_open_in translated to Rust (src/io.rs). */


/* buffer_overflow, conversion_error, apply_normalization, input_line:
   translated to Rust (src/io.rs). */


/* u_close: translated to Rust in src/io.rs */

/* Thin wrapper so Rust can finalize an ICU UConverter without directly
   linking against libicuuc (ICU is already linked via the C objects). */
void
xetex_ucnv_close(void* cnv)
{
    ucnv_close((UConverter*)cnv);
}

/* Thin wrapper so Rust can open an ICU UConverter by name. */
void*
xetex_ucnv_open(const char* name, int* out_err)
{
    UErrorCode err = U_ZERO_ERROR;
    UConverter* cnv = ucnv_open(name, &err);
    if (out_err) *out_err = (int)err;
    return (void*)cnv;
}

/* Thin wrapper: ucnv_toAlgorithmic with UCNV_UTF32_NativeEndian.
   Used by the Rust translation of input_line to avoid exposing
   UConverterType and UErrorCode to Rust directly. */
int32_t
xetex_ucnv_to_utf32(void* cnv,
    char* target, int32_t targetCapacity,
    const char* source, int32_t sourceLength,
    int* out_err)
{
    UErrorCode err = U_ZERO_ERROR;
    int32_t result = ucnv_toAlgorithmic(UCNV_UTF32_NativeEndian, (UConverter*)cnv,
                                        target, targetCapacity,
                                        source, sourceLength, &err);
    if (out_err) *out_err = (int)err;
    return result;
}


/* get_uni_c translated to Rust (src/io.rs). */


/* make_utf16_name translated to Rust (src/io.rs). */


/* open_or_close_in translated to Rust (src/io.rs). */
