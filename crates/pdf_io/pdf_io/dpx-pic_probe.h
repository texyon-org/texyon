#ifndef _DPX_PIC_PROBE_H_
#define _DPX_PIC_PROBE_H_

#include "texyon_bridge_core.h"
#include "dpx-pdfdev.h"

#define PIC_PROBE_OK 0
#define PIC_PROBE_FAIL 1
#define PIC_PROBE_ABORTED 2

int tt_pic_probe_pdf_box(const char *ident, rust_input_handle_t handle,
                         int page_num, int dpx_options,
                         pdf_rect *bbox, int *rotate);
int tt_pic_probe_image_box(rust_input_handle_t handle,
                           unsigned int *width, unsigned int *height,
                           double *xdensity, double *ydensity);
int tt_pic_probe_pdf_page_count(const char *ident, rust_input_handle_t handle,
                                int *pages_out);
/* Drops the PDFs parsed by earlier probes, whose files may have changed since. */
void tt_pic_probe_forget(void);

#endif
