/* Linking this adapter incorporates AGPL-3.0-or-later MuPDF. */

#include <mupdf/fitz.h>
#include <stdlib.h>
#include <string.h>

struct pr_backend {
    fz_context *ctx;
    fz_buffer *buffer;
    fz_output *output;
    fz_device *device;
    fz_font **fonts;
    int font_count;
    int font_capacity;
};

struct pr_backend *pr_backend_new(void)
{
    struct pr_backend *backend = calloc(1, sizeof(*backend));
    if (!backend)
        return NULL;
    backend->ctx = fz_new_context(NULL, NULL, FZ_STORE_UNLIMITED);
    if (!backend->ctx) {
        free(backend);
        return NULL;
    }
    return backend;
}

static void pr_close_page(struct pr_backend *backend)
{
    if (backend->device) {
        fz_close_device(backend->ctx, backend->device);
        fz_drop_device(backend->ctx, backend->device);
        backend->device = NULL;
    }
    if (backend->output) {
        fz_close_output(backend->ctx, backend->output);
        fz_drop_output(backend->ctx, backend->output);
        backend->output = NULL;
    }
    if (backend->buffer) {
        fz_drop_buffer(backend->ctx, backend->buffer);
        backend->buffer = NULL;
    }
}

void pr_backend_drop(struct pr_backend *backend)
{
    if (!backend)
        return;
    pr_close_page(backend);
    for (int i = 0; i < backend->font_count; ++i)
        fz_drop_font(backend->ctx, backend->fonts[i]);
    free(backend->fonts);
    fz_drop_context(backend->ctx);
    free(backend);
}

int pr_backend_font(struct pr_backend *backend, const unsigned char *data,
                    size_t length, int face_index)
{
    fz_buffer *buffer = NULL;
    fz_font *font = NULL;
    int result = -1;
    fz_var(buffer);
    fz_var(font);
    fz_try(backend->ctx) {
        buffer = fz_new_buffer_from_copied_data(backend->ctx, data, length);
        font = fz_new_font_from_buffer(backend->ctx, NULL, buffer, face_index, 0);
        if (backend->font_count == backend->font_capacity) {
            int capacity = backend->font_capacity ? backend->font_capacity * 2 : 8;
            fz_font **fonts = realloc(backend->fonts, capacity * sizeof(*fonts));
            if (!fonts)
                fz_throw(backend->ctx, FZ_ERROR_GENERIC, "font registry allocation failed");
            backend->fonts = fonts;
            backend->font_capacity = capacity;
        }
        result = backend->font_count;
        backend->fonts[backend->font_count++] = font;
        font = NULL;
    }
    fz_always(backend->ctx) {
        if (font)
            fz_drop_font(backend->ctx, font);
        if (buffer)
            fz_drop_buffer(backend->ctx, buffer);
    }
    fz_catch(backend->ctx) {
        result = -1;
    }
    return result;
}

int pr_backend_glyphs(struct pr_backend *backend, int font_id, float size,
                      const unsigned short *gids, const float *positions,
                      int count, const float *rgba, float extend, float slant,
                      int vertical)
{
    if (font_id < 0 || font_id >= backend->font_count)
        return 0;
    fz_text *text = NULL;
    int ok = 0;
    fz_var(text);
    fz_try(backend->ctx) {
        text = fz_new_text(backend->ctx);
        for (int i = 0; i < count; ++i) {
            fz_matrix matrix = {
                size * extend, 0.0f, size * slant, -size,
                positions[i * 2], positions[i * 2 + 1]
            };
            fz_show_glyph(backend->ctx, text, backend->fonts[font_id], matrix,
                gids[i], 0, vertical, 0, FZ_BIDI_LTR, FZ_LANG_UNSET);
        }
        fz_fill_text(backend->ctx, backend->device, text, fz_identity,
            fz_device_rgb(backend->ctx), rgba, rgba[3], fz_default_color_params);
        ok = 1;
    }
    fz_always(backend->ctx) {
        if (text)
            fz_drop_text(backend->ctx, text);
    }
    fz_catch(backend->ctx) {
        ok = 0;
    }
    return ok;
}

int pr_backend_begin(struct pr_backend *backend, float width, float height)
{
    int ok = 0;
    fz_try(backend->ctx) {
        pr_close_page(backend);
        backend->buffer = fz_new_buffer(backend->ctx, 0);
        backend->output = fz_new_output_with_buffer(backend->ctx, backend->buffer);
        backend->device = fz_new_svg_device(backend->ctx, backend->output,
            width, height, FZ_SVG_TEXT_AS_PATH, 0);
        ok = 1;
    }
    fz_catch(backend->ctx) {
        pr_close_page(backend);
        ok = 0;
    }
    return ok;
}

int pr_backend_rule(struct pr_backend *backend, float x, float y,
                    float width, float height, const float *rgba)
{
    fz_path *path = NULL;
    int ok = 0;
    fz_var(path);
    fz_try(backend->ctx) {
        path = fz_new_path(backend->ctx);
        fz_rectto(backend->ctx, path, x, y - height, x + width, y);
        fz_fill_path(backend->ctx, backend->device, path, 0, fz_identity,
            fz_device_rgb(backend->ctx), rgba, rgba[3], fz_default_color_params);
        ok = 1;
    }
    fz_always(backend->ctx) {
        if (path)
            fz_drop_path(backend->ctx, path);
    }
    fz_catch(backend->ctx) {
        ok = 0;
    }
    return ok;
}

unsigned char *pr_backend_finish(struct pr_backend *backend, size_t *length)
{
    unsigned char *copy = NULL;
    unsigned char *data = NULL;
    size_t size = 0;
    *length = 0;
    fz_try(backend->ctx) {
        fz_close_device(backend->ctx, backend->device);
        fz_drop_device(backend->ctx, backend->device);
        backend->device = NULL;
        fz_close_output(backend->ctx, backend->output);
        size = fz_buffer_storage(backend->ctx, backend->buffer, &data);
        copy = malloc(size ? size : 1);
        if (!copy)
            fz_throw(backend->ctx, FZ_ERROR_GENERIC, "SVG allocation failed");
        memcpy(copy, data, size);
        *length = size;
    }
    fz_catch(backend->ctx) {
        free(copy);
        copy = NULL;
        *length = 0;
    }
    if (backend->output) {
        fz_drop_output(backend->ctx, backend->output);
        backend->output = NULL;
    }
    if (backend->buffer) {
        fz_drop_buffer(backend->ctx, backend->buffer);
        backend->buffer = NULL;
    }
    return copy;
}

void pr_backend_free(unsigned char *data)
{
    free(data);
}
