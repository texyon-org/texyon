/* Guard abort-prone picture probes without longjmp crossing a Rust frame. */

#include <setjmp.h>
#include <stdint.h>
#include <string.h>

#include "texyon_bridge_core.h"

#include "dpx-bmpimage.h"
#include "dpx-jpegimage.h"
#include "dpx-pdfdoc.h"
#include "dpx-pdfobj.h"
#include "dpx-pic_probe.h"
#include "dpx-pngimage.h"

/* Parsed PDFs kept between probes, so probing many pages of one file parses it once. */
static void *probed_files = NULL;

static void *
enter_probed_files (void)
{
    void *outer = pdf_files_suspend();

    if (probed_files)
        pdf_files_restore(probed_files);
    else
        pdf_files_init();
    probed_files = NULL;
    return outer;
}

static void
leave_probed_files (void *outer, int keep)
{
    if (keep)
        probed_files = pdf_files_suspend();
    else
        pdf_files_close();
    pdf_files_restore(outer);
}

void
tt_pic_probe_forget (void)
{
    void *outer;

    if (!probed_files)
        return;
    outer = pdf_files_suspend();
    pdf_files_restore(probed_files);
    probed_files = NULL;
    pdf_files_close();
    pdf_files_restore(outer);
}

int
tt_pic_probe_pdf_box(const char *ident, rust_input_handle_t handle,
                     int page_num, int dpx_options,
                     pdf_rect *bbox, int *rotate)
{
    jmp_buf *jb = ttbc_global_engine_get_jump_buffer();
    jmp_buf saved;
    void *outer = enter_probed_files();
    volatile int rc = PIC_PROBE_FAIL;

    memcpy(&saved, jb, sizeof(jmp_buf));
    if (setjmp(*jb)) {
        /* An aborted parse may have left a file half read. */
        leave_probed_files(outer, 0);
        memcpy(jb, &saved, sizeof(jmp_buf));
        return PIC_PROBE_ABORTED;
    }

    {
        pdf_file *pf = pdf_open(ident, handle);
        if (pf) {
            int pages = pdf_doc_get_page_count(pf);
            int pno = page_num;

            if (pno > pages)
                pno = pages;
            if (pno < 0)
                pno += pages + 1;
            if (pno < 1)
                pno = 1;

            if (pdf_doc_get_page_rect(pf, pno, (enum pdf_page_boundary) dpx_options,
                                      bbox, rotate) == 0)
                rc = PIC_PROBE_OK;
            pdf_close(pf);
        }
    }

    leave_probed_files(outer, 1);
    memcpy(jb, &saved, sizeof(jmp_buf));
    return rc;
}

int
tt_pic_probe_image_box(rust_input_handle_t handle,
                       unsigned int *width, unsigned int *height,
                       double *xdensity, double *ydensity)
{
    jmp_buf *jb = ttbc_global_engine_get_jump_buffer();
    jmp_buf saved;
    int rc;

    memcpy(&saved, jb, sizeof(jmp_buf));
    if (setjmp(*jb)) {
        memcpy(jb, &saved, sizeof(jmp_buf));
        return PIC_PROBE_ABORTED;
    }

    rc = PIC_PROBE_FAIL;
    if (check_for_jpeg(handle)) {
        rc = jpeg_get_bbox(handle, width, height, xdensity, ydensity);
    } else if (check_for_bmp(handle)) {
        rc = bmp_get_bbox(handle, width, height, xdensity, ydensity);
    } else if (check_for_png(handle)) {
        uint32_t w = 0, h = 0;
        rc = png_get_bbox(handle, &w, &h, xdensity, ydensity);
        *width = w;
        *height = h;
    }

    memcpy(jb, &saved, sizeof(jmp_buf));
    return rc;
}

int
tt_pic_probe_pdf_page_count(const char *ident, rust_input_handle_t handle,
                            int *pages_out)
{
    jmp_buf *jb = ttbc_global_engine_get_jump_buffer();
    jmp_buf saved;
    void *outer = enter_probed_files();
    volatile int rc = PIC_PROBE_FAIL;

    memcpy(&saved, jb, sizeof(jmp_buf));
    if (setjmp(*jb)) {
        /* An aborted parse may have left a file half read. */
        leave_probed_files(outer, 0);
        memcpy(jb, &saved, sizeof(jmp_buf));
        return PIC_PROBE_ABORTED;
    }

    {
        pdf_file *pf = pdf_open(ident, handle);
        if (pf) {
            *pages_out = pdf_doc_get_page_count(pf);
            pdf_close(pf);
            rc = PIC_PROBE_OK;
        }
    }

    leave_probed_files(outer, 1);
    memcpy(jb, &saved, sizeof(jmp_buf));
    return rc;
}
