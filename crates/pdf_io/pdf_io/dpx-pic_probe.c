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

int
tt_pic_probe_pdf_box(const char *ident, rust_input_handle_t handle,
                     int page_num, int dpx_options,
                     pdf_rect *bbox, pdf_tmatrix *matrix)
{
    jmp_buf *jb = ttbc_global_engine_get_jump_buffer();
    jmp_buf saved;
    void *saved_files = pdf_files_suspend();
    volatile int rc = PIC_PROBE_FAIL;
    volatile int files_inited = 0;

    memcpy(&saved, jb, sizeof(jmp_buf));
    if (setjmp(*jb)) {
        if (files_inited)
            pdf_files_close();
        pdf_files_restore(saved_files);
        memcpy(jb, &saved, sizeof(jmp_buf));
        return PIC_PROBE_ABORTED;
    }

    pdf_files_init();
    files_inited = 1;
    {
        pdf_file *pf = pdf_open(ident, handle);
        if (pf) {
            int pages = pdf_doc_get_page_count(pf);
            int pno = page_num;
            pdf_obj *page;

            if (pno > pages)
                pno = pages;
            if (pno < 0)
                pno += pages + 1;
            if (pno < 1)
                pno = 1;

            page = pdf_doc_get_page(pf, pno,
                                    (enum pdf_page_boundary) dpx_options,
                                    bbox, matrix, NULL);
            pdf_close(pf);
            if (page) {
                pdf_release_obj(page);
                rc = PIC_PROBE_OK;
            }
        }
    }

    pdf_files_close();
    files_inited = 0;
    pdf_files_restore(saved_files);
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
    void *saved_files = pdf_files_suspend();
    volatile int rc = PIC_PROBE_FAIL;
    volatile int files_inited = 0;

    memcpy(&saved, jb, sizeof(jmp_buf));
    if (setjmp(*jb)) {
        if (files_inited)
            pdf_files_close();
        pdf_files_restore(saved_files);
        memcpy(jb, &saved, sizeof(jmp_buf));
        return PIC_PROBE_ABORTED;
    }

    pdf_files_init();
    files_inited = 1;
    {
        pdf_file *pf = pdf_open(ident, handle);
        if (pf) {
            *pages_out = pdf_doc_get_page_count(pf);
            pdf_close(pf);
            rc = PIC_PROBE_OK;
        }
    }

    pdf_files_close();
    files_inited = 0;
    pdf_files_restore(saved_files);
    memcpy(jb, &saved, sizeof(jmp_buf));
    return rc;
}
