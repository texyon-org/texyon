/* tectonic/output.c -- functions related to outputting messages
 * Copyright 2016 the Tectonic Project
 * Licensed under the MIT License.
*/

#include "xetex-core.h"
#include "xetex-xetexd.h"
#include "xetex-synctex.h"
#include "tectonic_bridge_core.h"

ttbc_diagnostic_t *current_diagnostic = 0;

void
capture_to_diagnostic(ttbc_diagnostic_t *diagnostic)
{
    if (current_diagnostic) {
        ttstub_diag_finish(current_diagnostic);
    }

    current_diagnostic = diagnostic;
}

static void
diagnostic_print_file_line(ttbc_diagnostic_t *diagnostic)
{
    // Add file/line number information
    // This duplicates logic from print_file_line

    int32_t level = in_open;
    while (level > 0 && full_source_filename_stack[level] == 0)
        level--;

    if (level == 0) {
        ttbc_diag_append(diagnostic, "!");
    } else {
        int32_t source_line = line;
        if (level != in_open) {
            source_line = line_stack[level + 1];
        }

        char* filename = gettexstring(full_source_filename_stack[level]);
        ttstub_diag_printf(diagnostic, "%s:%d: ", filename, source_line);
        free(filename);
    }
}

ttbc_diagnostic_t *
diagnostic_begin_capture_warning_here(void)
{
    ttbc_diagnostic_t *warning = ttbc_diag_begin_warning();
    diagnostic_print_file_line(warning);
    capture_to_diagnostic(warning);
    return warning;
}

// This replaces the "print file+line number" block at the start of errors
ttbc_diagnostic_t *
error_here_with_diagnostic(const char* message)
{
    ttbc_diagnostic_t *error = ttbc_diag_begin_error();
    diagnostic_print_file_line(error);
    ttstub_diag_printf(error, "%s", message);

    if (file_line_error_style_p)
        print_file_line();
    else
        print_nl_cstr("! ");
    print_cstr(message);

    capture_to_diagnostic(error);

    return error;
}

void
print_cs(int32_t p)
{
    if (p < HASH_BASE) {
        if (p >= SINGLE_BASE) {
            if (p == NULL_CS) {
                print_esc_cstr("csname");
                print_esc_cstr("endcsname");
                print_char(' ');
            } else {
                print_esc(p - SINGLE_BASE);
                if (CAT_CODE(p - SINGLE_BASE) == LETTER)
                    print_char(' ');
            }
        } else if (p < ACTIVE_BASE)
            print_esc_cstr("IMPOSSIBLE.");
        else
            print_char(p - 1);
    } else if (((p >= UNDEFINED_CONTROL_SEQUENCE) && (p <= EQTB_SIZE)) || (p > eqtb_top)) {
        print_esc_cstr("IMPOSSIBLE.");
    } else if (hash[p].s1 >= str_ptr) {
        print_esc_cstr("NONEXISTENT.");
    } else {
        if (p >= PRIM_EQTB_BASE && p < FROZEN_NULL_FONT)
            print_esc(prim[p - PRIM_EQTB_BASE].s1 - 1);
        else
            print_esc(hash[p].s1);
        print_char(' ');
    }
}


void
sprint_cs(int32_t p)
{
    if (p < HASH_BASE) {
        if (p < SINGLE_BASE)
            print_char(p - 1);
        else if (p < NULL_CS)
            print_esc(p - SINGLE_BASE);
        else {
            print_esc_cstr("csname");
            print_esc_cstr("endcsname");
        }
    } else if (p >= PRIM_EQTB_BASE && p < FROZEN_NULL_FONT) {
        print_esc(prim[p - PRIM_EQTB_BASE].s1 - 1);
    } else {
        print_esc(hash[p].s1);
    }
}


void
print_file_name(int32_t n, int32_t a, int32_t e)
{
    bool must_quote = false;
    int32_t quote_char = 0;
    pool_pointer j;

    if (a != 0) {
        j = str_start[(a) - 0x10000];
        while (((!must_quote) || (quote_char == 0)) && (j < str_start[(a + 1) - 0x10000])) {
            if (str_pool[j] == ' ' )
                must_quote = true;
            else if ((str_pool[j] == '"' ) || (str_pool[j] == '\'' )) {
                must_quote = true;
                quote_char = 73 /*""" 39 */  - str_pool[j];
            }
            j++;
        }
    }

    if (n != 0) {
        j = str_start[(n) - 0x10000];
        while (((!must_quote) || (quote_char == 0)) && (j < str_start[(n + 1) - 0x10000])) {
            if (str_pool[j] == ' ' )
                must_quote = true;
            else if ((str_pool[j] == '"' ) || (str_pool[j] == '\'' )) {
                must_quote = true;
                quote_char = 73 /*""" 39 */  - str_pool[j];
            }
            j++;
        }
    }

    if (e != 0) {
        j = str_start[(e) - 0x10000];
        while (((!must_quote) || (quote_char == 0)) && (j < str_start[(e + 1) - 0x10000])) {
            if (str_pool[j] == ' ' )
                must_quote = true;
            else if ((str_pool[j] == '"' ) || (str_pool[j] == '\'' )) {
                must_quote = true;
                quote_char = 73 /*""" 39 */  - str_pool[j];
            }
            j++;
        }
    }

    if (must_quote) {
        if (quote_char == 0)
            quote_char = '"' ;
        print_char(quote_char);
    }

    if (a != 0) {
        register int32_t for_end;
        j = str_start[(a) - 0x10000];
        for_end = str_start[(a + 1) - 0x10000] - 1;
        if (j <= for_end)
            do {
                if (str_pool[j] == quote_char) {
                    print(quote_char);
                    quote_char = 73 /*""" 39 */  - quote_char;
                    print(quote_char);
                }
                print(str_pool[j]);
            }
            while (j++ < for_end);
    }

    if (n != 0) {
        register int32_t for_end;
        j = str_start[(n) - 0x10000];
        for_end = str_start[(n + 1) - 0x10000] - 1;
        if (j <= for_end)
            do {
                if (str_pool[j] == quote_char) {
                    print(quote_char);
                    quote_char = 73 /*""" 39 */  - quote_char;
                    print(quote_char);
                }
                print(str_pool[j]);
            }
            while (j++ < for_end);
    }

    if (e != 0) {
        register int32_t for_end;
        j = str_start[(e) - 0x10000];
        for_end = str_start[(e + 1) - 0x10000] - 1;
        if (j <= for_end)
            do {
                if (str_pool[j] == quote_char) {
                    print(quote_char);
                    quote_char = 73 /*""" 39 */  - quote_char;
                    print(quote_char);
                }
                print(str_pool[j]);
            }
            while (j++ < for_end);
    }

    if (quote_char != 0)
        print_char(quote_char);
}


void
print_size(int32_t s)
{
    if (s == TEXT_SIZE)
        print_esc_cstr("textfont");
    else if (s == SCRIPT_SIZE)
        print_esc_cstr("scriptfont");
    else
        print_esc_cstr("scriptscriptfont");
}


void
print_write_whatsit(const char* s, int32_t p)
{

    print_esc_cstr(s);

    if (mem[p + 1].b32.s0 < 16)
        print_int(mem[p + 1].b32.s0);
    else if (mem[p + 1].b32.s0 == 16)
        print_char('*');
    else
        print_char('-');
}


void
print_native_word(int32_t p)
{
    int32_t i, c, cc;
    int32_t for_end = mem[p + 4].b16.s1 - 1;

    for (i = 0; i <= for_end; i++) {
        c = NATIVE_NODE_text(p)[i];
        if ((c >= 0xD800) && (c < 0xDC00)) {
            if (i < mem[p + 4].b16.s1 - 1) {
                cc = NATIVE_NODE_text(p)[i + 1];
                if ((cc >= 0xDC00) && (cc < 0xE000)) {
                    c = 0x10000 + (c - 0xD800) * 1024 + (cc - 0xDC00);
                    print_char(c);
                    i++;
                } else
                    print('.');
            } else
                print('.');
        } else
            print_char(c);
    }
}


void
print_sa_num(int32_t q)
{
    int32_t n;

    if (mem[q].b16.s1 < DIMEN_VAL_LIMIT)
        n = mem[q + 1].b32.s1;
    else {
        n = mem[q].b16.s1 % 64;
        q = LLIST_link(q);
        n = n + 64 * mem[q].b16.s1;
        q = LLIST_link(q);
        n = n + 64 * 64 * (mem[q].b16.s1 + 64 * mem[mem[q].b32.s1].b16.s1);
    }

    print_int(n);
}


void
print_file_line(void)
{
    int32_t level = in_open;

    while ((level > 0) && (full_source_filename_stack[level] == 0))
        level--;

    if (level == 0)
        print_nl_cstr("! ");
    else {
        print_nl_cstr("");
        print(full_source_filename_stack[level]);
        print(':');
        if (level == in_open)
            print_int(line);
        else
            print_int(line_stack[level + 1]);
        print_cstr(": ");
    }
}
/*:1660*/


void
print_current_string(void)
{
    pool_pointer j = str_start[str_ptr - 0x10000];

    while (j < pool_ptr) {
        print_char(str_pool[j]);
        j++;
    }
}



