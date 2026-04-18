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

void
set_input_file_encoding(UFILE* f, int32_t mode, int32_t encodingData)
{
    if ((f->encodingMode == ICUMAPPING) && (f->conversionData != NULL))
        ucnv_close((UConverter*)(f->conversionData));
    f->conversionData = 0;

    switch (mode) {
        case UTF8:
        case UTF16BE:
        case UTF16LE:
        case RAW:
            f->encodingMode = mode;
            break;

        case ICUMAPPING:
            {
                char* name = gettexstring(encodingData);
                UErrorCode err = U_ZERO_ERROR;
                UConverter* cnv = ucnv_open(name, &err);
                if (cnv == NULL) {
                    begin_diagnostic();
                    print_nl('E');
                    print_c_string("rror ");
                    print_int(err);
                    print_c_string(" creating Unicode converter for `");
                    print_c_string(name);
                    print_c_string("'; reading as raw bytes");
                    end_diagnostic(1);
                    f->encodingMode = RAW;
                } else {
                    f->encodingMode = ICUMAPPING;
                    f->conversionData = cnv;
                }
                free(name);
            }
            break;
    }
}


/* u_open_in translated to Rust (src/io.rs). */


static void
buffer_overflow(void)
{
    _tt_abort("unable to read an entire line (buf_size=%u)", (unsigned) buf_size);
}


static void
conversion_error(int errcode)
{
    begin_diagnostic();
    print_nl('U');
    print_c_string("nicode conversion failed (ICU error code = ");
    print_int(errcode);
    print_c_string(") discarding any remaining text");
    end_diagnostic(1);
}


static void
apply_normalization(uint32_t* buf, int len, int norm)
{
    static TECkit_Converter normalizers[2] = { NULL, NULL };

    TECkit_Status status;
    UInt32 inUsed, outUsed;
    TECkit_Converter *normPtr = &normalizers[norm - 1];
    if (*normPtr == NULL) {
        status = TECkit_CreateConverter(NULL, 0, 1,
            NATIVE_UTF32, NATIVE_UTF32 | (norm == 1 ? kForm_NFC : kForm_NFD),
            &*normPtr);
        if (status != kStatus_NoError)
            _tt_abort ("failed to create normalizer: error code = %d", (int)status);
    }

    status = TECkit_ConvertBuffer(*normPtr, (Byte*)buf, len * sizeof(UInt32), &inUsed,
                (Byte*)&buffer[first], sizeof(*buffer) * (buf_size - first), &outUsed, 1);
    TECkit_ResetConverter(*normPtr);
    if (status != kStatus_NoError)
        buffer_overflow();
    last = first + outUsed / sizeof(*buffer);
}


int
input_line(UFILE* f)
{
    static char* byteBuffer = NULL;
    static uint32_t *utf32Buf = NULL;
    int i, tmpLen;
    int norm = get_input_normalization_state();

    if (f->handle == INVALID_HANDLE)
        /* NULL 'handle' means this: */
        _tt_abort ("reads from synthetic \"terminal\" file #0 should never happen");

    last = first;

    if (f->encodingMode == ICUMAPPING) {
        uint32_t bytesRead = 0;
        UConverter* cnv;
        int outLen;
        UErrorCode errorCode = U_ZERO_ERROR;

        if (byteBuffer == NULL)
            byteBuffer = xmalloc(buf_size + 1);

        /* Recognize either LF or CR as a line terminator; skip initial LF if prev line ended with CR.  */
        i = ttstub_input_getc (f->handle);
        if (f->skipNextLF) {
            f->skipNextLF = 0;
            if (i == '\n')
                i = ttstub_input_getc (f->handle);
        }

        if (i != EOF && i != '\n' && i != '\r')
            byteBuffer[bytesRead++] = i;
        if (i != EOF && i != '\n' && i != '\r')
            while (bytesRead < buf_size && (i = ttstub_input_getc(f->handle)) != EOF && i != '\n' && i != '\r')
                byteBuffer[bytesRead++] = i;

        if (i == EOF && errno != EINTR && bytesRead == 0)
            return false;

        if (i != EOF && i != '\n' && i != '\r')
            buffer_overflow();

        /* now apply the mapping to turn external bytes into Unicode characters in buffer */
        cnv = (UConverter*)(f->conversionData);
        switch (norm) {
            case 1: // NFC
            case 2: // NFD
                if (utf32Buf == NULL)
                    utf32Buf = xcalloc(buf_size, sizeof(uint32_t));
                tmpLen = ucnv_toAlgorithmic(UCNV_UTF32_NativeEndian, cnv,
                                            (char*)utf32Buf, buf_size * sizeof(*utf32Buf),
                                            byteBuffer, bytesRead, &errorCode);
                if (errorCode != 0) {
                    conversion_error((int)errorCode);
                    return false;
                }
                apply_normalization(utf32Buf, tmpLen / sizeof(*utf32Buf), norm); // sets 'last' correctly
                break;

            default: // none
                outLen = ucnv_toAlgorithmic(UCNV_UTF32_NativeEndian, cnv,
                                            (char*)&buffer[first], sizeof(*buffer) * (buf_size - first),
                                            byteBuffer, bytesRead, &errorCode);
                if (errorCode != 0) {
                    conversion_error((int)errorCode);
                    return false;
                }
                outLen /= sizeof(*buffer);
                last = first + outLen;
                break;
        }
    } else {
        /* Recognize either LF or CR as a line terminator; skip initial LF if prev line ended with CR.  */
        i = get_uni_c(f);
        if (f->skipNextLF) {
            f->skipNextLF = 0;
            if (i == '\n')
                i = get_uni_c(f);
        }

        switch (norm) {
            case 1: // NFC
            case 2: // NFD
                // read Unicode chars into utf32Buf as UTF32
                if (utf32Buf == NULL)
                    utf32Buf = xcalloc(buf_size, sizeof(uint32_t));
                tmpLen = 0;
                if (i != EOF && i != '\n' && i != '\r')
                    utf32Buf[tmpLen++] = i;
                if (i != EOF && i != '\n' && i != '\r')
                    while (tmpLen < buf_size && (i = get_uni_c(f)) != EOF && i != '\n' && i != '\r')
                        utf32Buf[tmpLen++] = i;

                if (i == EOF && errno != EINTR && tmpLen == 0)
                    return false;

                /* We didn't get the whole line because our buffer was too small.  */
                if (i != EOF && i != '\n' && i != '\r')
                    buffer_overflow();
                apply_normalization(utf32Buf, tmpLen, norm);
                break;

            default: // none
                if (last < buf_size && i != EOF && i != '\n' && i != '\r')
                    buffer[last++] = i;
                if (i != EOF && i != '\n' && i != '\r')
                    while (last < buf_size && (i = get_uni_c(f)) != EOF && i != '\n' && i != '\r')
                        buffer[last++] = i;

                if (i == EOF && errno != EINTR && last == first)
                    return false;

                /* We didn't get the whole line because our buffer was too small.  */
                if (i != EOF && i != '\n' && i != '\r')
                    buffer_overflow();
                break;
        }
    }

    /* If line ended with CR, remember to skip following LF. */
    if (i == '\r')
        f->skipNextLF = 1;

    buffer[last] = ' ';
    if (last >= max_buf_stack)
        max_buf_stack = last;

    /* Trim trailing space or EOL characters.  */
#define IS_SPC_OR_EOL(c) ((c) == ' ' || (c) == '\r' || (c) == '\n')
    while (last > first && IS_SPC_OR_EOL(buffer[last - 1]))
        --last;

    return true;
}


void
u_close(UFILE* f)
{
    if (f == NULL || f->handle == INVALID_HANDLE)
        /* NULL handle is stdin/terminal file. Shouldn't happen but meh. */
        return;

    ttstub_input_close (f->handle);

    if (f->encodingMode == ICUMAPPING && f->conversionData != NULL)
        ucnv_close ((UConverter*) f->conversionData);

    free (f);
}


/* get_uni_c translated to Rust (src/io.rs). */


/* make_utf16_name translated to Rust (src/io.rs). */


/* open_or_close_in translated to Rust (src/io.rs). */
