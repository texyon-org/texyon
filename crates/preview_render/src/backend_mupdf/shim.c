/* Linking this adapter incorporates AGPL-3.0-or-later MuPDF. */

#include <mupdf/fitz.h>
#include <stdlib.h>
#include <string.h>

struct pr_backend {
    fz_context *ctx;
    fz_buffer *buffer;
    fz_output *output;
    fz_device *device;
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
    fz_drop_context(backend->ctx);
    free(backend);
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
