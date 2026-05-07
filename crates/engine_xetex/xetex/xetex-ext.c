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

static void*
loadOTfont(RawPlatformFontRef fontRef, XeTeXFont font, Fixed scaled_size, char* cp1)
{
    XeTeXLayoutEngine engine = NULL;
    hb_tag_t script = HB_TAG_NONE;
    char * language = NULL;
    hb_feature_t* features = NULL;
    char** shapers = NULL; /* NULL-terminated array */
    int nFeatures = 0;
    int nShapers = 0;

    char* cp2;
    const char* cp3;

    hb_tag_t tag;

    uint32_t rgbValue = 0x000000FF;

    float extend = 1.0;
    float slant = 0.0;
    float embolden = 0.0;
    float letterspace = 0.0;

    int i;

    char reqEngine = getReqEngine();

    if (reqEngine == 'O' || reqEngine == 'G') {
        shapers = (char**) xrealloc(shapers, (nShapers + 1) * sizeof(char *));
        if (reqEngine == 'O') {
            static char ot_const[] = "ot";
            shapers[nShapers] = ot_const;
        } else if (reqEngine == 'G') {
            static char graphite2_const[] = "graphite2";
            shapers[nShapers] = graphite2_const;
        }
        nShapers++;
    }

    if (reqEngine == 'G') {
    	static const char* graphiteShaper[] = {"graphite2", NULL};
        /* create a default engine so we can query the font for Graphite features;
         * because of font caching, it's cheap to discard this and create the real one later */
        engine = createLayoutEngineBorrowed(font, script, NULL,
                features, nFeatures, graphiteShaper, rgbValue, extend, slant, embolden);

        if (engine == NULL)
            return NULL;
    }

    /* scan the feature string (if any) */
    if (cp1 != NULL) {
        while (*cp1) {
            if ((*cp1 == ':') || (*cp1 == ';') || (*cp1 == ','))
                ++cp1;
            while ((*cp1 == ' ') || (*cp1 == '\t')) /* skip leading whitespace */
                ++cp1;
            if (*cp1 == 0) /* break if end of string */
                break;

            cp2 = cp1;
            while (*cp2 && (*cp2 != ':') && (*cp2 != ';') && (*cp2 != ','))
                ++cp2;

            cp3 = strstartswith(cp1, "script");
            if (cp3) {
                if (*cp3 != '=')
                    goto bad_option;
                ++cp3;
                script = hb_tag_from_string(cp3, cp2 - cp3);
                goto next_option;
            }

            cp3 = strstartswith(cp1, "language");
            if (cp3) {
                if (*cp3 != '=')
                    goto bad_option;
                ++cp3;
                language = xmalloc(cp2 - cp3 + 1);
                language[cp2 - cp3] = '\0';
                memcpy(language, cp3, cp2 - cp3);
                goto next_option;
            }

            cp3 = strstartswith(cp1, "shaper");
            if (cp3) {
                if (*cp3 != '=')
                    goto bad_option;
                ++cp3;
                shapers = (char**) xrealloc(shapers, (nShapers + 1) * sizeof(char *));
                /* some dumb systems have no strndup() */
                shapers[nShapers] = strdup(cp3);
                shapers[nShapers][cp2 - cp3] = '\0';
                nShapers++;
                goto next_option;
            }

            i = readCommonFeatures(cp1, cp2, &extend, &slant, &embolden, &letterspace, &rgbValue);
            if (i == 1)
                goto next_option;
            else if (i == -1)
                goto bad_option;

            if (reqEngine == 'G') {
                int value = 0;
                if (readFeatureNumber(cp1, cp2, &tag, &value)
                 || findGraphiteFeature(engine, cp1, cp2, &tag, &value)) {
                    features = (hb_feature_t*) xrealloc(features, (nFeatures + 1) * sizeof(hb_feature_t));
                    features[nFeatures].tag = tag;
                    features[nFeatures].value = value;
                    features[nFeatures].start = 0;
                    features[nFeatures].end = (unsigned int) -1;
                    nFeatures++;
                    goto next_option;
                }
            }

            if (*cp1 == '+') {
                int param = 0;
                tag = read_tag_with_param(cp1 + 1, &param);
                features = (hb_feature_t*) xrealloc(features, (nFeatures + 1) * sizeof(hb_feature_t));
                features[nFeatures].tag = tag;
                features[nFeatures].start = 0;
                features[nFeatures].end = (unsigned int) -1;
                // for backward compatibility with pre-0.9999 where feature
                // indices started from 0
                if (param >= 0)
                    param++;
                features[nFeatures].value = param;
                nFeatures++;
                goto next_option;
            }

            if (*cp1 == '-') {
                ++cp1;
                tag = hb_tag_from_string(cp1, cp2 - cp1);
                features = (hb_feature_t*) xrealloc(features, (nFeatures + 1) * sizeof(hb_feature_t));
                features[nFeatures].tag = tag;
                features[nFeatures].start = 0;
                features[nFeatures].end = (unsigned int) -1;
                features[nFeatures].value = 0;
                nFeatures++;
                goto next_option;
            }

            if (strstartswith(cp1, "vertical")) {
                cp3 = cp2;
                if (*cp3 == ';' || *cp3 == ':' || *cp3 == ',')
                    --cp3;
                while (*cp3 == '\0' || *cp3 == ' ' || *cp3 == '\t')
                    --cp3;
                if (*cp3)
                    ++cp3;
                if (cp3 == cp1 + 8) {
                    loaded_font_flags |= FONT_FLAGS_VERTICAL;
                    goto next_option;
                }
            }

        bad_option:
            font_feature_warning((void*) cp1, cp2 - cp1, 0, 0);

        next_option:
            cp1 = cp2;
        }
    }

    if (shapers != NULL) {
        shapers = (char**) xrealloc(shapers, (nShapers + 1) * sizeof(char *));
        shapers[nShapers] = NULL;
    }

	if (engine) {
		deleteLayoutEngine(engine);
	}

    if (embolden != 0.0)
        embolden = embolden * Fix2D(scaled_size) / 100.0;

    if (letterspace != 0.0)
        loaded_font_letter_space = (letterspace / 100.0) * scaled_size;

    if ((loaded_font_flags & FONT_FLAGS_COLORED) == 0)
        rgbValue = 0x000000FF;

    if ((loaded_font_flags & FONT_FLAGS_VERTICAL) != 0)
        setFontLayoutDir(font, 1);

    engine = createLayoutEngine(font, script, language,
                    features, nFeatures, (const char**) shapers, rgbValue, extend, slant, embolden);

    // The layout engine clones all these - this is temporary to avoid using
    // different alloc/dealloc
    if (features) {
        free(features);
    }
    if (shapers) {
        free(shapers);
    }
    if (language) {
        free(language);
    }
    if (engine) {
        native_font_type_flag = OTGR_FONT_FLAG;
    }

    return engine;
}

void splitFontName(char* name, char** var, char** feat, char** end, int* index);

void*
find_native_font(char* uname, int32_t scaled_size)
    /* scaled_size here is in TeX points, or is a negative integer for 'scaled_t' */
{
    void* rval = NULL;
    char* nameString;
    char* var;
    char* feat;
    char* end;
    char* name = (char*)uname;
    char* varString = NULL;
    char* featString = NULL;
    RawPlatformFontRef fontRef;
    XeTeXFont font = NULL;
    int index = 0;

    loaded_font_mapping = NULL;
    loaded_font_flags = 0;
    loaded_font_letter_space = 0;

    splitFontName(name, &var, &feat, &end, &index);
    nameString = xmalloc(var - name + 1);
    strncpy(nameString, name, var - name);
    nameString[var - name] = 0;

    if (feat > var) {
        varString = xmalloc(feat - var);
        strncpy(varString, var + 1, feat - var - 1);
        varString[feat - var - 1] = 0;
    }

    if (end > feat) {
        featString = xmalloc(end - feat);
        strncpy(featString, feat + 1, end - feat - 1);
        featString[end - feat - 1] = 0;
    }

    // check for "[filename]" form, don't search maps in this case
    if (nameString[0] == '[') {
        if (scaled_size < 0) {
            font = createFontFromFile(nameString + 1, index, 655360L);
            if (font != NULL) {
                Fixed dsize = D2Fix(getDesignSize(font));
                if (scaled_size == -1000)
                    scaled_size = dsize;
                else
                    scaled_size = xn_over_d(dsize, -scaled_size, 1000);
                deleteFont(font);
            }
        }
        font = createFontFromFile(nameString + 1, index, scaled_size);
        if (font != NULL) {
            set_loaded_font_design_size(D2Fix(getDesignSize(font)));

            /* This is duplicated in XeTeXFontMgr::findFont! */
            setReqEngine(0);
            if (varString) {
                if (strstartswith(varString, "/AAT"))
                    setReqEngine('A');
                else if ((strstartswith(varString, "/OT")) || (strstartswith(varString, "/ICU")))
                    setReqEngine('O');
                else if (strstartswith(varString, "/GR"))
                    setReqEngine('G');
            }

            rval = loadOTfont(0, font, scaled_size, featString);
            if (rval == NULL)
                deleteFont(font);
            if (rval != NULL && get_tracing_fonts_state() > 0) {
                begin_diagnostic();
                print_nl(' ');
                print_c_string("-> ");
                print_c_string(nameString + 1);
                end_diagnostic(0);
            }
        }
    } else {
        fontRef = findFontByName(nameString, varString, Fix2D(scaled_size));

        /* Tectonic: this used to live in XeTeXFontMgr::findFont(), but we needed to
         * move it here to preserve encapsulation.
         */

        if (get_tracing_fonts_state() > 0) {
            begin_diagnostic();
            print_nl(' ');
            print_c_string("-> ");
            print_c_string(ttxl_platfont_get_desc(fontRef));
            end_diagnostic(0);
        }

        if (fontRef) {
            /* update name_of_file to the full name of the font, for error messages during font loading */
            const char* fullName = getFullName(fontRef);
            name_length = strlen(fullName);
            if (featString != NULL)
                name_length += strlen(featString) + 1;
            if (varString != NULL)
                name_length += strlen(varString) + 1;
            free(name_of_file);
            name_of_file = xmalloc(name_length + 1);
            strcpy(name_of_file, fullName);

            if (scaled_size < 0) {
                font = createFont(fontRef, scaled_size);
                if (font != NULL) {
                    Fixed dsize = D2Fix(getDesignSize(font));
                    if (scaled_size == -1000)
                        scaled_size = dsize;
                    else
                        scaled_size = xn_over_d(dsize, -scaled_size, 1000);
                    deleteFont(font);
                }
            }

            font = createFont(fontRef, scaled_size);
            if (font != NULL) {
#ifdef XETEX_MAC
                /* decide whether to use AAT or OpenType rendering with this font */
                if (getReqEngine() == 'A') {
                    rval = loadAATfont(fontRef, scaled_size, featString);
                    if (rval == NULL)
                        deleteFont(font);
                } else {
                    if (getReqEngine() == 'O' || getReqEngine() == 'G' ||
                            hasFontTable(font, kGSUB) || hasFontTable(font, kGPOS))
                        rval = loadOTfont(fontRef, font, scaled_size, featString);

                    /* loadOTfont failed or the above check was false */
                    if (rval == NULL)
                        rval = loadAATfont(fontRef, scaled_size, featString);

                    if (rval == NULL)
                        deleteFont(font);
                }
#else
                rval = loadOTfont(fontRef, font, scaled_size, featString);
                if (rval == NULL)
                    deleteFont(font);
#endif
            }

            /* append the style and feature strings, so that \show\fontID will give a full result */
            if (varString != NULL && *varString != 0) {
                strcat(name_of_file, "/");
                strcat(name_of_file, varString);
            }
            if (featString != NULL && *featString != 0) {
                strcat(name_of_file, ":");
                strcat(name_of_file, featString);
            }
            name_length = strlen(name_of_file);
        }
    }

    free(varString);

    free(featString);

    free(nameString);

    return rval;
}

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


