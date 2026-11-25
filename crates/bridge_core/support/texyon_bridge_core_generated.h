#ifndef TEXYON_BRIDGE_CORE_GENERATED_H
#define TEXYON_BRIDGE_CORE_GENERATED_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
typedef uintptr_t rust_input_handle_t;
typedef uintptr_t rust_output_handle_t;
typedef rust_input_handle_t Option_InputId;
typedef rust_output_handle_t Option_OutputId;
#define INVALID_HANDLE ((uintptr_t)0)


/**
 * Different types of files that can be opened by TeX engines
 *
 * This enumeration is used to guess filename extensions to try when looking
 * for a file to open.
 *
 */
typedef enum {
  /**
   * An Adobe Font Metrics file.
   */
  TTBC_FILE_FORMAT_AFM = 4,
  /**
   * A BibTeX bibliography data file.
   */
  TTBC_FILE_FORMAT_BIB = 6,
  /**
   * A BibTeX style file.
   */
  TTBC_FILE_FORMAT_BST = 7,
  /**
   * A character map data file.
   */
  TTBC_FILE_FORMAT_CMAP = 45,
  /**
   * A configuration file.
   */
  TTBC_FILE_FORMAT_CNF = 8,
  /**
   * An encoding data file.
   */
  TTBC_FILE_FORMAT_ENC = 44,
  /**
   * A TeX "format" file.
   */
  TTBC_FILE_FORMAT_FORMAT = 10,
  /**
   * A font-map file.
   */
  TTBC_FILE_FORMAT_FONT_MAP = 11,
  /**
   * A miscellaneous font file.
   */
  TTBC_FILE_FORMAT_MISC_FONTS = 41,
  /**
   * An OFM font metrics file.
   */
  TTBC_FILE_FORMAT_OFM = 20,
  /**
   * An OpenType font file.
   */
  TTBC_FILE_FORMAT_OPEN_TYPE = 47,
  /**
   * An OVF file.
   */
  TTBC_FILE_FORMAT_OVF = 23,
  /**
   * An image file.
   */
  TTBC_FILE_FORMAT_PICT = 25,
  /**
   * A PK font file.
   */
  TTBC_FILE_FORMAT_PK = 1,
  /**
   * A general program data file.
   */
  TTBC_FILE_FORMAT_PROGRAM_DATA = 39,
  /**
   * An SFD file.
   */
  TTBC_FILE_FORMAT_SFD = 46,
  /**
   * The Tectonic primary input file.
   */
  TTBC_FILE_FORMAT_TEXYON_PRIMARY = 59,
  /**
   * A TeX language file.
   */
  TTBC_FILE_FORMAT_TEX = 26,
  /**
   * A TeX PostScript header file.
   */
  TTBC_FILE_FORMAT_TEX_PS_HEADER = 30,
  /**
   * A TeX Font Metrics file.
   */
  TTBC_FILE_FORMAT_TFM = 3,
  /**
   * A TrueType font file.
   */
  TTBC_FILE_FORMAT_TRUE_TYPE = 36,
  /**
   * A Type1 font file.
   */
  TTBC_FILE_FORMAT_TYPE1 = 32,
  /**
   * A Virtual Font file.
   */
  TTBC_FILE_FORMAT_VF = 33,
} ttbc_file_format;

/**
 * The CoreBridgeState structure is a handle to Rust state that can be used by
 * C/C++ engine code to perform basic I/O functions.
 *
 * Code that invokes a Tectonic C/C++ engine should pass a pointer to one of
 * these state structures into the C/C++ layer. It is essential that lifetimes
 * be properly managed across the Rust/C boundary.
 */
typedef struct ttbc_state_t ttbc_state_t;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

extern const char *_ttbc_get_error_message(void);

/**
 * Issue a warning.
 *
 * # Safety
 *
 * This function is unsafe because it accepts a raw C string.
 */
void ttbc_issue_warning(ttbc_state_t *es, const char *text);

/**
 * Open a Tectonic file for output.
 *
 * # Safety
 *
 * This function is unsafe because it accepts a raw C string.
 */
Option_OutputId ttbc_output_open(ttbc_state_t *es, const char *name, int is_gz);

/**
 * Open the general user output stream as a Tectonic output file.
 */
Option_OutputId ttbc_output_open_stdout(ttbc_state_t *es);

/**
 * Write a single character to a Tectonic output file.
 */
int ttbc_output_putc(ttbc_state_t *es, Option_OutputId handle, int c);

/**
 * Write data to a Tectonic output file.
 *
 * # Safety
 *
 * This function is unsafe because it dereferences raw C pointers.
 */
size_t ttbc_output_write(ttbc_state_t *es, Option_OutputId handle, const uint8_t *data, size_t len);

/**
 * Close a Tectonic output file.
 */
int ttbc_output_close(ttbc_state_t *es, Option_OutputId handle);

/**
 * Open a Tectonic file for input.
 *
 * # Safety
 *
 * This function is unsafe because it accepts a raw C string.
 */
Option_InputId ttbc_input_open(ttbc_state_t *es,
                               const char *name,
                               ttbc_file_format format,
                               int is_gz);

/**
 * Get the size of a Tectonic input file.
 */
size_t ttbc_input_get_size(ttbc_state_t *es, Option_InputId handle);

/**
 * Get the modification time of a Tectonic input file.
 */
int64_t ttbc_input_get_mtime(ttbc_state_t *es, Option_InputId handle);

/**
 * Seek in a Tectonic input stream.
 *
 * # Safety
 *
 * This function is unsafe because it dereferences raw pointers from C.
 */
size_t ttbc_input_seek(ttbc_state_t *es,
                       Option_InputId handle,
                       ssize_t offset,
                       int whence,
                       int *internal_error);

/**
 * Get a single character from a Tectonic input file.
 */
int ttbc_input_getc(ttbc_state_t *es, Option_InputId handle);

/**
 * Put back a character that was obtained from a `getc` call.
 */
int ttbc_input_ungetc(ttbc_state_t *es, Option_InputId handle, int ch);

/**
 * Read data from a Tectonic input handle
 *
 * This read corresponds to Rust's read_exact, i.e. it will return exactly the number of requested
 * bytes or error (-1).
 *
 * # Safety
 *
 * This function is unsafe because it dereferences raw C pointers.
 */
ssize_t ttbc_input_read(ttbc_state_t *es, Option_InputId handle, uint8_t *data, size_t len);

/**
 * Read data from a Tectonic input handle
 *
 * This read corresponds to Rust's read, i.e. it can return less bytes than requested (and does
 * when buffering)
 *
 * # Safety
 *
 * This function is unsafe because it dereferences raw C pointers.
 */
ssize_t ttbc_input_read_partial(ttbc_state_t *es, Option_InputId handle, uint8_t *data, size_t len);

/**
 * Close a Tectonic input file.
 */
int ttbc_input_close(ttbc_state_t *es, Option_InputId handle);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* TEXYON_BRIDGE_CORE_GENERATED_H */
