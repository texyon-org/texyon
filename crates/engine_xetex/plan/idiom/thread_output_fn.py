#!/usr/bin/env python3
"""Thread an output.rs print/show function over &mut EngineState (session 183).

For a named `pub unsafe fn NAME(args) { BODY }` it:
  - splits it into a thin `NAME` wrapper (with_state_mut → NAME_in) plus a
    `pub(crate) fn NAME_in(st: &mut EngineState, args) { BODY' }` core,
  - rewrites BODY: print wrappers -> their `_in` variants threading `st`;
    `with_state`-bearing globals -> direct `st.<cluster>.<field>` access.
RAW accessors (cur_chr/cur_cmd/line/cur_input/set_cur_input) are left alone:
they read `raw::` and are safe under a live borrow. Compiler-driven cleanup
(closure sigs, dead imports) is reported, not auto-fixed.
"""
import re
import sys

# print wrappers: NAME(args) -> NAME_in(st, args)
PRINT_FNS = {
    "print", "print_char", "print_cs", "print_cstr", "print_esc", "print_esc_cstr",
    "print_group", "print_hex", "print_int", "print_ln", "print_mode", "print_nl",
    "print_nl_cstr", "print_raw_char", "print_sa_num", "print_scaled", "print_size",
    "print_skip_param", "print_style", "print_totals", "print_cmd_chr", "print_meaning",
    "print_param", "print_length_param", "show_box", "show_token_list", "token_show",
    "show_save_groups", "show_activities", "show_context",
    "print_mark", "print_spec", "print_write_whatsit", "print_native_word",
    "print_fam_and_char", "print_glue", "print_delimiter", "print_rule_dimen",
    "print_font_and_char", "print_file_name", "sprint_cs", "print_subsidiary_data",
    "print_in_mode", "short_display", "show_node_list",
}

# scalar getters NAME() -> expr
GETTERS = {
    "tally": "st.errors.tally", "error_line": "st.errors.error_line",
    "half_error_line": "st.errors.half_error_line", "first_count": "st.errors.first_count",
    "trick_count": "st.errors.trick_count", "selector": "st.shipout.selector",
    "output_active": "st.shipout.output_active", "page_contents": "st.shipout.page_contents",
    "page_tail": "st.shipout.page_tail", "save_ptr": "st.save_stack.save_ptr",
    "cur_level": "st.save_stack.cur_level", "cur_group": "st.save_stack.cur_group",
    "cur_boundary": "st.save_stack.cur_boundary", "nest_ptr": "st.input.nest_ptr",
    "base_ptr": "st.input.base_ptr", "input_ptr": "st.input.input_ptr",
    "in_open": "st.input.in_open", "old_setting": "st.shipout.old_setting",
    "history": "st.errors.history",
}

# clean prefix-call replace NAME( -> NEWPREFIX(
PREFIX = {
    "mem_b16s0": "st.mem.b16s0", "mem_b16s1": "st.mem.b16s1",
    "mem_b32s0": "st.mem.b32s0", "mem_b32s1": "st.mem.b32s1",
    "eqtb_b32s1": "st.eqtb.b32s1", "save_stack_b32s1": "st.save_stack.b32s1",
}

# indexed accessor NAME(arg) -> prefix[arg suffix]
INDEXED = {
    "font_size_entry": ("st.fonts.font_size", " as usize"),
    "font_name_entry": ("st.fonts.font_name", " as usize"),
    "font_area_entry": ("st.fonts.font_area", " as usize"),
    "font_dsize_entry": ("st.fonts.font_dsize", " as usize"),
    "str_start": ("st.strings.str_start", " as usize"),
    "str_pool_entry": ("st.strings.str_pool", " as usize"),
    "buffer_entry": ("st.input.buffer", " as usize"),
    "input_stack": ("st.input.input_stack", " as usize"),
    "line_stack": ("st.input.line_stack", " as usize"),
    "full_source_filename_stack": ("st.input.full_source_filename_stack", " as usize"),
    "trick_buf": ("st.errors.trick_buf", ""),
    "page_so_far": ("st.shipout.page_so_far", ""),
    "cur_mark": ("st.scanner.cur_mark", ""),
}

# nest field NAME(arg) -> st.input.nest[arg as usize].field  (aux ones wrap unsafe)
NESTF = {
    "nest_head": "head", "nest_mode": "mode", "nest_mode_line": "mode_line",
    "nest_prev_graf": "prev_graf", "nest_e_tex_aux": "e_tex_aux",
}
NEST_AUX = {"nest_aux_b32s0": "s0", "nest_aux_b32s1": "s1"}

# scalar setters set_NAME(expr) -> lvalue = expr
SETTERS = {
    "set_save_ptr": "st.save_stack.save_ptr", "set_cur_level": "st.save_stack.cur_level",
    "set_cur_group": "st.save_stack.cur_group", "set_base_ptr": "st.input.base_ptr",
    "set_first_count": "st.errors.first_count", "set_tally": "st.errors.tally",
    "set_selector": "st.shipout.selector", "set_trick_count": "st.errors.trick_count",
    "set_old_setting": "st.shipout.old_setting", "set_history": "st.errors.history",
}


def skip_literal(s, j):
    """If s[j] begins a string/char literal or comment, return index past it;
    else return j unchanged. Handles // /* */ "..." and '...' (incl. '{' '}')."""
    c = s[j]
    if c == "/" and j + 1 < len(s) and s[j + 1] == "/":
        k = s.find("\n", j)
        return len(s) if k < 0 else k
    if c == "/" and j + 1 < len(s) and s[j + 1] == "*":
        k = s.find("*/", j + 2)
        return len(s) if k < 0 else k + 2
    if c == '"':
        k = j + 1
        while k < len(s):
            if s[k] == "\\":
                k += 2
                continue
            if s[k] == '"':
                return k + 1
            k += 1
        return len(s)
    if c == "'":
        # char literal 'x' / '\n' / '{' / '}' — but not a loop label ('foo:)
        m = re.match(r"'(\\.|[^'\\])'", s[j:])
        if m:
            return j + m.end()
    return j


def match_paren(s, i):
    """s[i] == '('; return (inner, end) where end is index after matching ')'."""
    assert s[i] == "("
    depth = 0
    j = i
    while j < len(s):
        nj = skip_literal(s, j)
        if nj != j:
            j = nj
            continue
        c = s[j]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return s[i + 1:j], j + 1
        j += 1
    raise ValueError("unbalanced")


def split_top(args):
    out, depth, cur = [], 0, ""
    for c in args:
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        if c == "," and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += c
    if cur.strip():
        out.append(cur)
    return out


def apply_call(body, name, builder):
    """Replace every \\bname(...) (balanced) using builder(args)->str."""
    out = ""
    i = 0
    pat = re.compile(r"\b" + re.escape(name) + r"\(")
    while True:
        m = pat.search(body, i)
        if not m:
            out += body[i:]
            break
        out += body[i:m.start()]
        args, end = match_paren(body, m.end() - 1)
        out += builder(args)
        i = end
    return body[:0] + out if False else out


def transform_body(body):
    # 1. print wrappers: insert "st, " after the opening paren
    for name in sorted(PRINT_FNS, key=len, reverse=True):
        body = apply_call(body, name, lambda a, n=name: f"{n}_in(st, {a})")
    body = re.sub(r"\(st, \)", "(st)", body)
    # 1b. the print_cmd_chr local closure: takes st explicitly, name unchanged
    if "print_char_or_wide(" in body:
        body = apply_call(body, "print_char_or_wide", lambda a: f"print_char_or_wide(st, {a})")
    # 2. clean prefix replacements
    for name, rep in PREFIX.items():
        body = re.sub(r"\b" + re.escape(name) + r"\(", rep + "(", body)
    # 3. scalar setters (before getters; names disjoint via \b)
    for name, lval in SETTERS.items():
        body = apply_call(body, name, lambda a, l=lval: f"{l} = {a.strip()}")
    if "set_input_stack" in body:
        body = apply_call(body, "set_input_stack",
                          lambda a: (lambda p: f"st.input.input_stack[{p[0].strip()} as usize] = {p[1].strip()}")(split_top(a)))
    body = body.replace("crate::globals::save_cur_list_to_nest", "save_cur_list_to_nest")
    if "save_cur_list_to_nest" in body:
        body = apply_call(body, "save_cur_list_to_nest",
                          lambda a: f"st.input.nest[({a.strip()}) as usize] = crate::globals::cur_list()")
    if "save_stack_b16s0" in body:
        body = apply_call(body, "save_stack_b16s0",
                          lambda a: f"unsafe {{ st.save_stack.data[({a.strip()}) as usize].b16.s0 }}")
    # 4. indexed accessors (parenthesize arg before a non-empty `as usize` suffix)
    for name, (pre, suf) in INDEXED.items():
        body = apply_call(
            body, name,
            lambda a, p=pre, s=suf: f"{p}[({a.strip()}){s}]" if s else f"{p}[{a.strip()}]",
        )
    # 5. nest field accessors (parenthesize the index expr before `as usize`)
    for name, fld in NESTF.items():
        body = apply_call(body, name, lambda a, f=fld: f"st.input.nest[({a.strip()}) as usize].{f}")
    for name, sx in NEST_AUX.items():
        body = apply_call(body, name, lambda a, x=sx: f"unsafe {{ st.input.nest[({a.strip()}) as usize].aux.b32.{x} }}")
    # 6. scalar getters NAME()
    for name, rep in GETTERS.items():
        body = re.sub(r"\b" + re.escape(name) + r"\(\s*\)", rep, body)
    return body


def find_fn(src, name):
    m = re.search(r"pub (unsafe )?fn " + re.escape(name) + r"\(", src)
    if not m:
        raise SystemExit(f"fn {name} not found")
    is_unsafe = m.group(1) is not None
    # back up to include the doc-comment block
    start = m.start()
    # signature end: first ') {' at depth 0 after name
    psig = src.index("(", m.start())
    args, after = match_paren(src, psig)
    assert src[after:after + 2] == " {", repr(src[after:after + 5])
    bodystart = after + 2
    # brace match the body
    depth = 1
    j = bodystart
    while depth:
        nj = skip_literal(src, j)
        if nj != j:
            j = nj
            continue
        c = src[j]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        j += 1
    bodyend = j  # index after closing }
    body = src[bodystart:bodyend - 1]
    return start, bodyend, args, body, is_unsafe


def arg_names(args):
    names = []
    for a in split_top(args):
        a = a.strip()
        if not a:
            continue
        names.append(a.split(":")[0].strip().lstrip("mut ").strip())
    return names


def main():
    path = sys.argv[1]
    names = sys.argv[2:]
    src = open(path).read()
    for name in names:
        start, end, args, body, is_unsafe = find_fn(src, name)
        nbody = transform_body(body)
        anames = arg_names(args)
        callargs = ", ".join(["st"] + anames)
        sig_args = ("st: &mut EngineState, " + args) if args.strip() else "st: &mut EngineState"
        kw = "pub unsafe fn" if is_unsafe else "pub fn"
        wrapper = (
            f"{kw} {name}({args}) {{\n"
            f"    crate::state::with_state_mut(|st| {name}_in({callargs}));\n"
            f"}}\n\n"
            f"/// `&mut EngineState` core of [`{name}`].\n"
            f"pub(crate) fn {name}_in({sig_args}) {{{nbody}}}"
        )
        src = src[:start] + wrapper + src[end:]
        # report leftovers
        leftovers = sorted(set(re.findall(r"\bwith_state\w*\(", nbody)))
        print(f"[{name}] threaded. leftover with_state: {leftovers}")
    open(path, "w").write(src)


if __name__ == "__main__":
    main()
