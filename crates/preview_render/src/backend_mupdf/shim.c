/* Linking this adapter incorporates AGPL-3.0-or-later MuPDF. */

#include <mupdf/fitz.h>
#include <stdlib.h>
#include <string.h>

struct pr_backend {
    fz_context *ctx;
    fz_buffer *buffer;
    fz_output *output;
    fz_device *device;
    fz_pixmap *pixmap;
    fz_font **fonts;
    int font_count;
    int font_capacity;
};

struct pr_stroke_style {
    float width;
    int join;
    int cap;
    float miter_limit;
    float dash[4];
    size_t dash_len;
    float dash_phase;
};

static fz_path *pr_make_path(fz_context *ctx, const unsigned char *kinds,
                             const float *coordinates, int count)
{
    fz_path *path = fz_new_path(ctx);
    int coordinate = 0;
    for (int i = 0; i < count; ++i) {
        switch (kinds[i]) {
        case 0:
            fz_moveto(ctx, path, coordinates[coordinate], coordinates[coordinate + 1]);
            coordinate += 2;
            break;
        case 1:
            fz_lineto(ctx, path, coordinates[coordinate], coordinates[coordinate + 1]);
            coordinate += 2;
            break;
        case 2:
            fz_curveto(ctx, path,
                coordinates[coordinate], coordinates[coordinate + 1],
                coordinates[coordinate + 2], coordinates[coordinate + 3],
                coordinates[coordinate + 4], coordinates[coordinate + 5]);
            coordinate += 6;
            break;
        case 3:
            fz_rectto(ctx, path,
                coordinates[coordinate], coordinates[coordinate + 1],
                coordinates[coordinate + 2], coordinates[coordinate + 3]);
            coordinate += 4;
            break;
        case 4:
            fz_closepath(ctx, path);
            break;
        default:
            fz_throw(ctx, FZ_ERROR_GENERIC, "unknown preview path command");
        }
    }
    return path;
}

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
    fz_try(backend->ctx) {
        fz_register_document_handlers(backend->ctx);
    }
    fz_catch(backend->ctx) {
        fz_drop_context(backend->ctx);
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
    if (backend->pixmap) {
        fz_drop_pixmap(backend->ctx, backend->pixmap);
        backend->pixmap = NULL;
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

int pr_backend_gid_by_name(struct pr_backend *backend, int font_id,
                           const char *name)
{
    if (font_id < 0 || font_id >= backend->font_count)
        return -1;
    int gid = -1;
    fz_try(backend->ctx) {
        gid = fz_encode_character_by_glyph_name(backend->ctx,
            backend->fonts[font_id], name);
    }
    fz_catch(backend->ctx) {
        gid = -1;
    }
    return gid;
}

int pr_backend_glyphs(struct pr_backend *backend, int font_id, float size,
                      const unsigned short *gids, const fz_matrix *transforms,
                      int count, const float *rgba, float extend, float slant,
                      float embolden, int vertical)
{
    if (font_id < 0 || font_id >= backend->font_count)
        return 0;
    fz_text *text = NULL;
    fz_stroke_state *stroke = NULL;
    int ok = 0;
    fz_var(text);
    fz_var(stroke);
    fz_try(backend->ctx) {
        text = fz_new_text(backend->ctx);
        for (int i = 0; i < count; ++i) {
            fz_matrix glyph = {
                size * extend, 0.0f, size * slant, size, 0.0f, 0.0f
            };
            fz_matrix matrix = fz_concat(glyph, transforms[i]);
            fz_show_glyph(backend->ctx, text, backend->fonts[font_id], matrix,
                gids[i], 0, vertical, 0, FZ_BIDI_LTR, FZ_LANG_UNSET);
        }
        fz_fill_text(backend->ctx, backend->device, text, fz_identity,
            fz_device_rgb(backend->ctx), rgba, rgba[3], fz_default_color_params);
        if (embolden > 0.0f) {
            stroke = fz_new_stroke_state(backend->ctx);
            stroke->linewidth = size * embolden;
            stroke->linejoin = FZ_LINEJOIN_ROUND;
            fz_stroke_text(backend->ctx, backend->device, text, stroke,
                fz_identity, fz_device_rgb(backend->ctx), rgba, rgba[3],
                fz_default_color_params);
        }
        ok = 1;
    }
    fz_always(backend->ctx) {
        if (text)
            fz_drop_text(backend->ctx, text);
        if (stroke)
            fz_drop_stroke_state(backend->ctx, stroke);
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

int pr_backend_rule(struct pr_backend *backend, float width, float height,
                    fz_matrix transform, const float *rgba)
{
    fz_path *path = NULL;
    int ok = 0;
    fz_var(path);
    fz_try(backend->ctx) {
        path = fz_new_path(backend->ctx);
        fz_rectto(backend->ctx, path, 0, 0, width, height);
        fz_fill_path(backend->ctx, backend->device, path, 0, transform,
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

int pr_backend_fill_path(struct pr_backend *backend,
                         const unsigned char *kinds, const float *coordinates,
                         int count, int even_odd, fz_matrix transform,
                         const float *rgba)
{
    fz_path *path = NULL;
    int ok = 0;
    fz_var(path);
    fz_try(backend->ctx) {
        path = pr_make_path(backend->ctx, kinds, coordinates, count);
        fz_fill_path(backend->ctx, backend->device, path, even_odd, transform,
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

int pr_backend_stroke_path(struct pr_backend *backend,
                           const unsigned char *kinds, const float *coordinates,
                           int count, struct pr_stroke_style style,
                           fz_matrix transform, const float *rgba)
{
    fz_path *path = NULL;
    fz_stroke_state *stroke = NULL;
    int ok = 0;
    fz_var(path);
    fz_var(stroke);
    fz_try(backend->ctx) {
        path = pr_make_path(backend->ctx, kinds, coordinates, count);
        stroke = fz_new_stroke_state_with_dash_len(backend->ctx,
            style.dash_len > 4 ? 4 : style.dash_len);
        stroke->linewidth = style.width;
        stroke->linejoin = style.join < 0 ? 0 : (style.join > 2 ? 2 : style.join);
        int cap = style.cap < 0 ? 0 : (style.cap > 2 ? 2 : style.cap);
        stroke->start_cap = cap;
        stroke->dash_cap = cap;
        stroke->end_cap = cap;
        stroke->miterlimit = style.miter_limit;
        stroke->dash_phase = style.dash_phase;
        stroke->dash_len = style.dash_len > 4 ? 4 : style.dash_len;
        for (int i = 0; i < stroke->dash_len; ++i)
            stroke->dash_list[i] = style.dash[i];
        fz_stroke_path(backend->ctx, backend->device, path, stroke, transform,
            fz_device_rgb(backend->ctx), rgba, rgba[3], fz_default_color_params);
        ok = 1;
    }
    fz_always(backend->ctx) {
        if (path)
            fz_drop_path(backend->ctx, path);
        if (stroke)
            fz_drop_stroke_state(backend->ctx, stroke);
    }
    fz_catch(backend->ctx) {
        ok = 0;
    }
    return ok;
}

int pr_backend_clip_path(struct pr_backend *backend,
                         const unsigned char *kinds, const float *coordinates,
                         int count, int even_odd, fz_matrix transform)
{
    fz_path *path = NULL;
    int ok = 0;
    fz_var(path);
    fz_try(backend->ctx) {
        path = pr_make_path(backend->ctx, kinds, coordinates, count);
        fz_clip_path(backend->ctx, backend->device, path, even_odd, transform,
            fz_infinite_rect);
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

int pr_backend_pop_clip(struct pr_backend *backend)
{
    int ok = 0;
    fz_try(backend->ctx) {
        fz_pop_clip(backend->ctx, backend->device);
        ok = 1;
    }
    fz_catch(backend->ctx) {
        ok = 0;
    }
    return ok;
}

int pr_backend_image(struct pr_backend *backend, const unsigned char *data,
                     size_t length, fz_matrix base, float width, float height)
{
    fz_buffer *buffer = NULL;
    fz_image *image = NULL;
    int ok = 0;
    fz_var(buffer);
    fz_var(image);
    fz_try(backend->ctx) {
        buffer = fz_new_buffer_from_copied_data(backend->ctx, data, length);
        image = fz_new_image_from_buffer(backend->ctx, buffer);
        float ratio = (float)image->w / (float)image->h;
        if (width != width && height != height) {
            width = (float)image->w;
            height = (float)image->h;
        } else if (width != width) {
            width = height * ratio;
        } else if (height != height) {
            height = width / ratio;
        }
        fz_matrix transform = fz_pre_scale(fz_pre_translate(base, 0, height),
            width, -height);
        fz_fill_image(backend->ctx, backend->device, image, transform, 1.0f,
            fz_default_color_params);
        ok = 1;
    }
    fz_always(backend->ctx) {
        if (image)
            fz_drop_image(backend->ctx, image);
        if (buffer)
            fz_drop_buffer(backend->ctx, buffer);
    }
    fz_catch(backend->ctx) {
        ok = 0;
    }
    return ok;
}

int pr_backend_pdf(struct pr_backend *backend, const unsigned char *data,
                   size_t length, int page_number, fz_matrix base)
{
    fz_stream *stream = NULL;
    fz_document *document = NULL;
    fz_page *page = NULL;
    int ok = 0;
    fz_var(stream);
    fz_var(document);
    fz_var(page);
    fz_try(backend->ctx) {
        stream = fz_open_memory(backend->ctx, data, length);
        document = fz_open_document_with_stream(backend->ctx, "application/pdf", stream);
        page = fz_load_page(backend->ctx, document, page_number);
        fz_rect box = fz_bound_page(backend->ctx, page);
        fz_matrix transform = fz_pre_translate(base, 0, box.y0 - box.y1);
        fz_run_page(backend->ctx, page, backend->device, transform, NULL);
        ok = 1;
    }
    fz_always(backend->ctx) {
        if (page)
            fz_drop_page(backend->ctx, page);
        if (document)
            fz_drop_document(backend->ctx, document);
        if (stream)
            fz_drop_stream(backend->ctx, stream);
    }
    fz_catch(backend->ctx) {
        ok = 0;
    }
    return ok;
}

/* Draws into the caller's `samples`, `width * height` tightly packed RGBA pixels. */
int pr_backend_begin_raster(struct pr_backend *backend, int width, int height,
                            float scale, unsigned char *samples)
{
    int ok = 0;
    fz_try(backend->ctx) {
        pr_close_page(backend);
        backend->pixmap = fz_new_pixmap_with_data(backend->ctx, fz_device_rgb(backend->ctx),
            width, height, NULL, 1, width * 4, samples);
        fz_clear_pixmap_with_value(backend->ctx, backend->pixmap, 255);
        backend->device = fz_new_draw_device(backend->ctx, fz_scale(scale, scale),
            backend->pixmap);
        ok = 1;
    }
    fz_catch(backend->ctx) {
        pr_close_page(backend);
        ok = 0;
    }
    return ok;
}

int pr_backend_finish_raster(struct pr_backend *backend)
{
    int ok = 0;
    fz_try(backend->ctx) {
        fz_close_device(backend->ctx, backend->device);
        ok = 1;
    }
    fz_catch(backend->ctx) {
        ok = 0;
    }
    pr_close_page(backend);
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

unsigned char *pr_backend_raster(struct pr_backend *backend,
        const unsigned char *data, size_t length, const char *mime,
        int page_number, float dpi, int *width, int *height, int *components,
        size_t *sample_length)
{
    fz_stream *stream = NULL;
    fz_document *document = NULL;
    fz_page *page = NULL;
    fz_pixmap *pixmap = NULL;
    unsigned char *copy = NULL;
    fz_var(stream);
    fz_var(document);
    fz_var(page);
    fz_var(pixmap);
    *sample_length = 0;
    fz_try(backend->ctx) {
        stream = fz_open_memory(backend->ctx, data, length);
        document = fz_open_document_with_stream(backend->ctx, mime, stream);
        page = fz_load_page(backend->ctx, document, page_number);
        fz_matrix transform = fz_scale(dpi / 72.0f, dpi / 72.0f);
        pixmap = fz_new_pixmap_from_page(backend->ctx, page, transform,
            fz_device_rgb(backend->ctx), 0);
        *width = fz_pixmap_width(backend->ctx, pixmap);
        *height = fz_pixmap_height(backend->ctx, pixmap);
        *components = fz_pixmap_components(backend->ctx, pixmap);
        int stride = fz_pixmap_stride(backend->ctx, pixmap);
        size_t row = (size_t)*width * (size_t)*components;
        *sample_length = row * (size_t)*height;
        copy = malloc(*sample_length ? *sample_length : 1);
        if (!copy)
            fz_throw(backend->ctx, FZ_ERROR_GENERIC, "raster allocation failed");
        unsigned char *samples = fz_pixmap_samples(backend->ctx, pixmap);
        for (int y = 0; y < *height; ++y)
            memcpy(copy + row * y, samples + stride * y, row);
    }
    fz_always(backend->ctx) {
        if (pixmap)
            fz_drop_pixmap(backend->ctx, pixmap);
        if (page)
            fz_drop_page(backend->ctx, page);
        if (document)
            fz_drop_document(backend->ctx, document);
        if (stream)
            fz_drop_stream(backend->ctx, stream);
    }
    fz_catch(backend->ctx) {
        free(copy);
        copy = NULL;
        *sample_length = 0;
    }
    return copy;
}

void pr_backend_free(unsigned char *data)
{
    free(data);
}
