/****************************************************************************\
 Part of the XeTeX typesetting system
 Copyright (c) 1994-2008 by SIL International
 Copyright (c) 2009, 2011 by Jonathan Kew
 Copyright (c) 2012-2015 by Khaled Hosny
 Copyright (c) 2012, 2013 by Jiang Jiang

 SIL Author(s): Jonathan Kew

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE COPYRIGHT HOLDERS BE LIABLE
FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the name of the copyright holders
shall not be used in advertising or otherwise to promote the sale,
use or other dealings in this Software without prior written
authorization from the copyright holders.
\****************************************************************************/

/* XeTeX_ext.c
 * additional plain C extensions for XeTeX - mostly platform-neutral
 */

#include "xetex-core.h"
#include "xetex-ext.h"
#include "teckit-c-Engine.h"
#include "xetex-swap.h"

#include <assert.h>
#include <locale.h>
#include <math.h> /* for fabs() */
#include <signal.h>
#include <time.h>

#ifndef _MSC_VER
#include <sys/time.h>
#endif

#include <unicode/ubidi.h>
#include <unicode/ubrk.h>
#include <unicode/ucnv.h>

#include <graphite2/Font.h>

#include "xetex-xetexd.h"


/* OT-related constants we need */
#define kGSUB HB_TAG('G','S','U','B')
#define kGPOS HB_TAG('G','P','O','S')


hb_tag_t read_tag_with_param(const char* cp, int* param);
bool readFeatureNumber(const char* s, const char* e, hb_tag_t* f, int* v);

void splitFontName(char* name, char** var, char** feat, char** end, int* index);

#define XDV_FLAG_VERTICAL       0x0100
#define XDV_FLAG_COLORED        0x0200
#define XDV_FLAG_EXTEND         0x1000
#define XDV_FLAG_SLANT          0x2000
#define XDV_FLAG_EMBOLDEN       0x4000

#ifdef XETEX_MAC
static UInt32
cgColorToRGBA32(CGColorRef color)
{
    const CGFloat *components = CGColorGetComponents(color);

    UInt32 rval = (UInt8)(components[0] * 255.0 + 0.5);
    rval <<= 8;
    rval += (UInt8)(components[1] * 255.0 + 0.5);
    rval <<= 8;
    rval += (UInt8)(components[2] * 255.0 + 0.5);
    rval <<= 8;
    rval += (UInt8)(components[3] * 255.0 + 0.5);
    return rval;
}
#endif


/* these are here, not XeTeX_mac.c, because we need stubs on other platforms */
