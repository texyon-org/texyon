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


void
measure_native_node(void* pNode, int use_glyph_metrics)
{
    memory_word* node = (memory_word*)pNode;
    int txtLen = native_length(node);
    uint16_t* txtPtr = (uint16_t*)(node + NATIVE_NODE_SIZE);

    unsigned int f = native_font(node);

#ifdef XETEX_MAC
    if (font_area[f] == AAT_FONT_FLAG) {
        /* we're using this font in AAT mode, so font_layout_engine[f] is actually a CFDictionaryRef */
        DoAATLayout(node, 0);
    } else
#endif
    if (font_area[f] == OTGR_FONT_FLAG) {
        /* using this font in OT Layout mode, so font_layout_engine[f] is actually a XeTeXLayoutEngine */

        XeTeXLayoutEngine engine = (XeTeXLayoutEngine)(font_layout_engine[f]);

        FixedPoint* locations = NULL;
        uint16_t* glyphIDs;
        Fixed* glyphAdvances = NULL;
        int totalGlyphCount = 0;

        /* need to find direction runs within the text, and call layoutChars separately for each */

        UBiDiDirection dir;
        void* glyph_info = 0;
        static FloatPoint* positions = 0;
        static float* advances = 0;
        static uint32_t* glyphs = 0;

        UBiDi* pBiDi = ubidi_open();

        UErrorCode errorCode = U_ZERO_ERROR;
        ubidi_setPara(pBiDi, (const UChar*) txtPtr, txtLen, getDefaultDirection(engine), NULL, &errorCode);

        dir = ubidi_getDirection(pBiDi);
        if (dir == UBIDI_MIXED) {
            /* we actually do the layout twice here, once to count glyphs and then again to get them;
               which is inefficient, but i figure that MIXED is a relatively rare occurrence, so i can't be
               bothered to deal with the memory reallocation headache of doing it differently
            */
            int nRuns = ubidi_countRuns(pBiDi, &errorCode);
            double width = 0;
            int i, runIndex;
            int32_t logicalStart, length;
            for (runIndex = 0; runIndex < nRuns; ++runIndex) {
                dir = ubidi_getVisualRun(pBiDi, runIndex, &logicalStart, &length);
                totalGlyphCount += layoutChars(engine, txtPtr, logicalStart, length, txtLen, (dir == UBIDI_RTL));
            }

            if (totalGlyphCount > 0) {
                double x, y;
                glyph_info = xcalloc(totalGlyphCount, native_glyph_info_size);
                locations = (FixedPoint*)glyph_info;
                glyphIDs = (uint16_t*)(locations + totalGlyphCount);
                glyphAdvances = xcalloc(totalGlyphCount, sizeof(Fixed));
                totalGlyphCount = 0;

                x = y = 0.0;
                for (runIndex = 0; runIndex < nRuns; ++runIndex) {
                    int nGlyphs;
                    dir = ubidi_getVisualRun(pBiDi, runIndex, &logicalStart, &length);
                    nGlyphs = layoutChars(engine, txtPtr, logicalStart, length, txtLen,
                                            (dir == UBIDI_RTL));

                    glyphs = xcalloc(nGlyphs, sizeof(uint32_t));
                    positions = xcalloc(nGlyphs + 1, sizeof(FloatPoint));
                    advances = xcalloc(nGlyphs, sizeof(float));

                    getGlyphs(engine, glyphs);
                    getGlyphAdvances(engine, advances);
                    getGlyphPositions(engine, positions);

                    for (i = 0; i < nGlyphs; ++i) {
                        glyphIDs[totalGlyphCount] = glyphs[i];
                        locations[totalGlyphCount].x = D2Fix(positions[i].x + x);
                        locations[totalGlyphCount].y = D2Fix(positions[i].y + y);
                        glyphAdvances[totalGlyphCount] = D2Fix(advances[i]);
                        ++totalGlyphCount;
                    }
                    x += positions[nGlyphs].x;
                    y += positions[nGlyphs].y;

                    free(glyphs);
                    free(positions);
                    free(advances);
                }
                width = x;
            }

            node_width(node) = D2Fix(width);
            native_glyph_count(node) = totalGlyphCount;
            native_glyph_info_ptr(node) = glyph_info;
        } else {
            double width = 0;
            totalGlyphCount = layoutChars(engine, txtPtr, 0, txtLen, txtLen, (dir == UBIDI_RTL));

            glyphs = xcalloc(totalGlyphCount, sizeof(uint32_t));
            positions = xcalloc(totalGlyphCount + 1, sizeof(FloatPoint));
            advances = xcalloc(totalGlyphCount, sizeof(float));

            getGlyphs(engine, glyphs);
            getGlyphAdvances(engine, advances);
            getGlyphPositions(engine, positions);

            if (totalGlyphCount > 0) {
                int i;
                glyph_info = xcalloc(totalGlyphCount, native_glyph_info_size);
                locations = (FixedPoint*)glyph_info;
                glyphIDs = (uint16_t*)(locations + totalGlyphCount);
                glyphAdvances = xcalloc(totalGlyphCount, sizeof(Fixed));
                for (i = 0; i < totalGlyphCount; ++i) {
                    glyphIDs[i] = glyphs[i];
                    glyphAdvances[i] = D2Fix(advances[i]);
                    locations[i].x = D2Fix(positions[i].x);
                    locations[i].y = D2Fix(positions[i].y);
                }
                width = positions[totalGlyphCount].x;
            }

            node_width(node) = D2Fix(width);
            native_glyph_count(node) = totalGlyphCount;
            native_glyph_info_ptr(node) = glyph_info;

            free(glyphs);
            free(positions);
            free(advances);
        }

        ubidi_close(pBiDi);


        if (font_letter_space[f] != 0) {
            Fixed lsDelta = 0;
            Fixed lsUnit = font_letter_space[f];
            int i;
            for (i = 0; i < totalGlyphCount; ++i) {
                if (glyphAdvances[i] == 0 && lsDelta != 0)
                    lsDelta -= lsUnit;
                locations[i].x += lsDelta;
                lsDelta += lsUnit;
            }
            if (lsDelta != 0) {
                lsDelta -= lsUnit;
                node_width(node) += lsDelta;
            }
        }
        free(glyphAdvances);
    } else {
        _tt_abort("bad native font flag in `measure_native_node`");
    }

    if (use_glyph_metrics == 0 || native_glyph_count(node) == 0) {
        /* for efficiency, height and depth are the font's ascent/descent,
            not true values based on the actual content of the word,
            unless use_glyph_metrics is non-zero */
        node_height(node) = height_base[f];
        node_depth(node) = depth_base[f];
    } else {
        /* this iterates over the glyph data whether it comes from AAT or OT layout */
        FixedPoint* locations = (FixedPoint*)native_glyph_info_ptr(node);
        uint16_t* glyphIDs = (uint16_t*)(locations + native_glyph_count(node));
        float yMin = 65536.0;
        float yMax = -65536.0;
        int i;
        for (i = 0; i < native_glyph_count(node); ++i) {
            float ht, dp;
            float y = Fix2D(-locations[i].y); /* NB negative is upwards in locations[].y! */

            GlyphBBox bbox;
            if (getCachedGlyphBBox(f, glyphIDs[i], &bbox) == 0) {
#ifdef XETEX_MAC
                if (font_area[f] == AAT_FONT_FLAG)
                    GetGlyphBBox_AAT((CFDictionaryRef)(font_layout_engine[f]), glyphIDs[i], &bbox);
                else
#endif
                if (font_area[f] == OTGR_FONT_FLAG)
                    getGlyphBounds((XeTeXLayoutEngine)(font_layout_engine[f]), glyphIDs[i], &bbox);

                cacheGlyphBBox(f, glyphIDs[i], &bbox);
            }

            ht = bbox.yMax;
            dp = -bbox.yMin;

            if (y + ht > yMax)
                yMax = y + ht;
            if (y - dp < yMin)
                yMin = y - dp;
        }
        node_height(node) = D2Fix(yMax);
        node_depth(node) = -D2Fix(yMin);
    }
}

/* these are here, not XeTeX_mac.c, because we need stubs on other platforms */

