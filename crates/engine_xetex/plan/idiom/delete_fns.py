#!/usr/bin/env python3
"""Delete named free functions (and their leading doc/attr lines) from a file.
Brace/literal-aware so `'{'` etc. inside bodies don't fool the matcher."""
import re
import sys


def skip_literal(s, j):
    c = s[j]
    if c == "/" and s[j:j + 2] == "//":
        k = s.find("\n", j)
        return len(s) if k < 0 else k
    if c == "/" and s[j:j + 2] == "/*":
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
        m = re.match(r"'(\\.|[^'\\])'", s[j:])
        if m:
            return j + m.end()
    return j


def delete_fn(src, name):
    m = re.search(r"pub(\(crate\))? (unsafe )?fn " + re.escape(name) + r"\(", src)
    if not m:
        raise SystemExit(f"fn {name} not found")
    start = m.start()
    # absorb preceding doc-comment / attribute lines
    ls = src.rfind("\n", 0, start)
    while ls != -1:
        pls = src.rfind("\n", 0, ls)
        line = src[pls + 1:ls + 1]
        if line.lstrip().startswith(("///", "//", "#[", "#!")):
            ls = pls
            start = pls + 1
        else:
            break
    # brace match the body
    bp = src.index("{", m.end())
    depth, j = 1, bp + 1
    while depth:
        nj = skip_literal(src, j)
        if nj != j:
            j = nj
            continue
        if src[j] == "{":
            depth += 1
        elif src[j] == "}":
            depth -= 1
        j += 1
    end = j
    while end < len(src) and src[end] in " \t":
        end += 1
    if src[end:end + 1] == "\n":
        end += 1
    # drop one trailing blank line if it leaves a double blank
    if src[end:end + 1] == "\n" and src[start - 2:start] == "\n\n":
        end += 1
    return src[:start] + src[end:]


def main():
    path = sys.argv[1]
    src = open(path).read()
    for name in sys.argv[2:]:
        src = delete_fn(src, name)
        print(f"deleted {name}")
    open(path, "w").write(src)


if __name__ == "__main__":
    main()
