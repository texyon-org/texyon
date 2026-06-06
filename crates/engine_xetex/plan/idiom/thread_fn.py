#!/usr/bin/env python3
"""Phase-C threading assistant for workstream 01 (`01b-phase-c-recipe.md`).

This automates the *mechanical, zero-judgment* half of threading a function off
the thread-local bridge onto explicit narrow `&mut MemState`/`&EqtbState`/…
borrows, and — just as importantly — automates the **FFI-hazard grep the
session-135/136 revert made mandatory**: before threading any function you must
prove its body does not interleave a `with_state`-bearing re-entry (the C++
OT-math measurer, `new_native_character`, print/diagnostic, scanner) with mem
allocation, because a single borrow held across that re-entry nest-panics
(`RefCell already borrowed`).

It deliberately does only the parts that are safe to automate (01b §4a buckets
A/B): the regular *indexed* accessor token rewrites whose call shape is
preserved and whose correctness the compiler then verifies. Everything that
needs human judgment — the signature's return type and cold-path sentinel
(§5), scalar getter/setter rewrites, `*_global` → threaded-core swaps, and
anything flagged HAZARD — is reported, never rewritten. C/D work stays manual.

What it does:
  1. Extracts the target fn body (column-0 `fn` … column-0 `}`, the move_fns.py
     heuristic).
  2. HAZARD report: scans the body for `with_state`-bearing re-entries. If any
     fire, the function is likely NOT single-borrow threadable — stop and use
     scoped islands or thread the callee first (the §135/136 lesson).
  3. Auto-rewrites the regular indexed accessors for the requested borrows
     (`mem_b32s1(x)`→`mem.b32s1(x)`, `set_mem_b32s1(x,v)`→`mem.set_b32s1(x,v)`,
     same for `eqtb_*`). Pure token renames; the call args are untouched.
  4. REVIEW report: lines needing a human (scalars, `*_global` calls, cold
     paths, sentinels).
  5. Prints the caller list (bridge-down sites) and a `_global` wrapper skeleton.

Usage:
    # dry run (default): report + print the rewritten body, touch nothing
    python3 plan/idiom/thread_fn.py --module src/packaging.rs --fn just_copy \
        --borrows mem

    # multiple borrows (disjoint &mut fields)
    python3 plan/idiom/thread_fn.py --module src/packaging.rs --fn append_to_vlist \
        --borrows mem,eqtb

    # apply the accessor rewrites in place (does NOT touch the signature or
    # add the wrapper — you wire those by hand after reviewing the report)
    python3 plan/idiom/thread_fn.py --module src/packaging.rs --fn just_copy \
        --borrows mem --apply

Always: dry-run first, read the HAZARD/REVIEW report, then --apply, then wire
the signature + sentinel + wrapper + callers by hand, then `cargo check` +
the four suites + (for math/fonts/native paths) `bench_phase_c.py --scope
tier1`/`tier1-real` before committing. The tool is an accelerator for the
keystrokes, not a substitute for the §4/§5 judgment.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src"

# Regular indexed-accessor suffixes whose global accessor maps 1:1 onto an
# inherent method of the same name on the cluster sub-struct (verified against
# `impl MemState`/`impl EqtbState` in src/state.rs). These are pure token
# renames: `<cluster>_<suf>(args)` -> `<var>.<suf>(args)` and
# `set_<cluster>_<suf>(args)` -> `<var>.set_<suf>(args)`.
INDEXED_SUFFIXES = ["b32s0", "b32s1", "b16s0", "b16s1", "b16s2", "b16s3", "word", "ptr", "gr"]

# Clusters this tool can auto-rewrite, mapped to their default borrow-var name.
# (fonts/scanner/save_stack accessors are less regular — name mismatches and
# scalar-heavy — so they are reported for manual rewrite, not auto-rewritten.)
AUTO_CLUSTERS = {"mem": "mem", "eqtb": "eqtb"}

# Calls that re-enter Rust through `with_state` (directly or transitively). A
# live outer borrow across any of these nest-panics (01b §4). If the body has
# one interleaved with mem allocation, the function is NOT single-borrow
# threadable — this is the automated session-135/136 grep.
HAZARD_RE = re.compile(
    r"\b("
    r"with_state(?:_mut)?|"
    r"measure_native_\w*|get_native_\w*|get_ot_math_\w*|get_native_mathsy_param|"
    r"new_native_character|"
    r"print_\w*|print\b|print_cstr|print_nl_cstr|print_esc\w*|print_int|print_scaled|print_ln|"
    r"diagnostic\w*|begin_diagnostic|end_diagnostic|error_here_with_diagnostic|"
    r"capture_to_diagnostic|\berror\s*\(|confusion|overflow|runaway|"
    r"back_input|get_next|get_token|get_x_token|scan_\w+|group_warning|"
    # session-138: re-entrant giants/helpers the earlier regex missed (they
    # call `with_state` transitively, so a body that interleaves any of these
    # with mem allocation nest-panics — clean_box/ins_the_toks/app_space all
    # looked HAZARD-free but call one of these). The packaging packers, the
    # math list builder, the token-list builders, and `xn_over_d` (which sets
    # the `tex_remainder`/`arith_error` globals via `with_state`, the §129
    # lesson) are NOT single-borrow threadable.
    r"mlist_to_hlist|hpack|vpackage|vsplit|vert_break|"
    r"the_toks|begin_token_list|macro_call|conv_toks|expand\b|"
    r"xn_over_d|x_over_n|unsave\b|build_page|push_nest|pop_nest|insert_src_special"
    r")\b"
)

# Cold-path / sentinel calls: not necessarily a hazard, but they need the §5
# sentinel-return treatment rather than a bare call under the borrow.
SENTINEL_RE = re.compile(r"\b(get_node_global|get_avail_global)\b")

# Any remaining `*_global` bridge call — candidate to swap for a threaded core
# (pass the borrow through) once that core exists.
GLOBAL_CALL_RE = re.compile(r"\b([a-z][a-z0-9_]*_global)\s*\(")

FN_START_RE_TMPL = (
    r"^(?P<prefix>(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?(?:extern\s+\"C\"\s+)?)"
    r"fn\s+{name}\s*[\(<]"
)


def extract_fn(lines: list[str], name: str) -> tuple[int, int]:
    """Return (start_idx, end_idx_exclusive) of the fn item body, column-0
    `fn` through the matching column-0 `}`. Mirrors move_fns.py's heuristic."""
    start_re = re.compile(FN_START_RE_TMPL.format(name=re.escape(name)))
    start = None
    for i, ln in enumerate(lines):
        if start_re.match(ln):
            start = i
            break
    if start is None:
        raise SystemExit(f"fn {name} not found (column-0 item) in module")
    for j in range(start + 1, len(lines)):
        if lines[j].startswith("}"):
            return start, j + 1
    raise SystemExit(f"fn {name}: no column-0 closing brace found")


def rewrite_accessors(body: str, borrows: dict[str, str]) -> tuple[str, int]:
    """Apply the regular indexed-accessor token renames for the requested
    clusters. Returns (new_body, num_substitutions)."""
    count = 0
    for cluster, var in borrows.items():
        if cluster not in AUTO_CLUSTERS:
            continue
        for suf in INDEXED_SUFFIXES:
            # set_<cluster>_<suf>  ->  <var>.set_<suf>   (do the set_ form first)
            set_pat = re.compile(rf"\bset_{cluster}_{suf}\b")
            body, n = set_pat.subn(f"{var}.set_{suf}", body)
            count += n
            # <cluster>_<suf>  ->  <var>.<suf>
            # safe: inside "set_<cluster>_..." there is no \b before <cluster>
            # (underscore-letter is intra-word), so this never double-fires.
            get_pat = re.compile(rf"\b{cluster}_{suf}\b")
            body, n = get_pat.subn(f"{var}.{suf}", body)
            count += n
    return body, count


def report(body: str) -> None:
    lines = body.splitlines()
    hazards, sentinels, globals_, reviews = [], [], [], []
    for k, ln in enumerate(lines, 1):
        if HAZARD_RE.search(ln):
            hazards.append((k, ln.strip()))
        if SENTINEL_RE.search(ln):
            sentinels.append((k, ln.strip()))
        for m in GLOBAL_CALL_RE.finditer(ln):
            if not SENTINEL_RE.search(m.group(0)):
                globals_.append((k, m.group(1)))
        # scalar getters/setters that are not indexed accessors but read a field
        if re.search(r"\b(hi_mem_min|mem_end|set_avail|set_rover|set_temp_ptr|"
                     r"set_hi_mem_min|set_mem_end|set_lo_mem_max)\b", ln):
            reviews.append((k, ln.strip()))

    def dump(title: str, items, fmt):
        print(f"\n--- {title} ({len(items)}) ---")
        for it in items:
            print(fmt(it))

    print("=" * 72)
    if hazards:
        print("HAZARD: with_state-bearing re-entries found in this body.")
        print("If any of these is INTERLEAVED with mem allocation, this function")
        print("CANNOT be param-threaded with a single borrow (01b §4, the")
        print("session-135/136 nest-panic). Use scoped islands or thread the")
        print("callee first. Review each:")
        dump("hazard lines", hazards, lambda it: f"  L{it[0]:>4}: {it[1]}")
    else:
        print("HAZARD: none found — body appears FFI/print/scanner-free, so a")
        print("single disjoint borrow across the whole body is safe (still")
        print("eyeball the report below).")
    if sentinels:
        dump("cold-path allocators (use cnl_get_node!/cnl_get_avail! + §5 sentinel)",
             sentinels, lambda it: f"  L{it[0]:>4}: {it[1]}")
    if globals_:
        dump("*_global bridge calls (swap for threaded core, pass borrow through, if it exists)",
             sorted(set(globals_)), lambda it: f"  L{it[0]:>4}: {it[1]}")
    if reviews:
        dump("scalar get/set (rewrite to field access/assignment by hand)",
             reviews, lambda it: f"  L{it[0]:>4}: {it[1]}")
    print("=" * 72)


def caller_sites(name: str, self_module: Path) -> None:
    print(f"\n--- caller sites of `{name}` (each becomes a bridge-down wrap or pass-through) ---")
    call_re = re.compile(rf"\b{re.escape(name)}\s*\(")
    def_re = re.compile(rf"\bfn\s+{re.escape(name)}\b")
    total = 0
    for f in sorted(SRC.glob("*.rs")):
        txt = f.read_text()
        hits = [
            (i, ln.strip())
            for i, ln in enumerate(txt.splitlines(), 1)
            if call_re.search(ln) and not def_re.search(ln)
        ]
        if hits:
            print(f"  {f.name}: {len(hits)}")
            total += len(hits)
    print(f"  TOTAL: {total}")


def wrapper_skeleton(name: str, borrows: dict[str, str]) -> None:
    args = ", ".join(f"&mut s.{c}" if c in ("mem",) else f"&s.{c}" for c in borrows)
    binds = "\n        let s = &mut *st;" if len(borrows) > 1 else ""
    call = f"{name}(" + ", ".join(
        (f"&mut st.{c}" if len(borrows) == 1 and c == 'mem' else f"&mut s.{c}" if c == 'mem' else f"&s.{c}")
        for c in borrows
    )
    print(f"\n--- `{name}_global` wrapper skeleton (wire the §5 sentinel to your return type) ---")
    print(f"""pub unsafe fn {name}_global(/* same value args as {name} minus the borrows */) {{
    {('let __r = ' if False else '')}crate::state::with_state_mut(|st| {{{binds}
        {call}, /* value args */)
    }});
    // if {name} returns a sentinel/Result: run the cold abort here, after the
    // borrow drops (e.g. `node_recycle_abort(e)` / `overflow(...)`).
}}""")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--module", required=True, help="e.g. src/packaging.rs")
    ap.add_argument("--fn", required=True, dest="fn", help="function name")
    ap.add_argument("--borrows", default="mem",
                    help="comma list of clusters, e.g. mem,eqtb (var name = cluster name)")
    ap.add_argument("--apply", action="store_true",
                    help="write the accessor rewrites back (signature/wrapper still manual)")
    args = ap.parse_args()

    mod = (ROOT / args.module) if not Path(args.module).is_absolute() else Path(args.module)
    if not mod.exists():
        mod = SRC / Path(args.module).name
    lines = mod.read_text().splitlines(keepends=True)
    start, end = extract_fn(lines, args.fn)
    body = "".join(lines[start:end])

    borrows = {c.strip(): c.strip() for c in args.borrows.split(",") if c.strip()}

    print(f"# {args.fn}  in {mod.relative_to(ROOT)}  (lines {start+1}..{end})")
    print(f"# borrows: {', '.join(f'&mut {v}' for v in borrows.values())}")
    report(body)
    caller_sites(args.fn, mod)
    wrapper_skeleton(args.fn, borrows)

    new_body, n = rewrite_accessors(body, borrows)
    print(f"\n--- accessor rewrites: {n} substitution(s) for borrows {list(borrows)} ---")
    if args.apply:
        lines[start:end] = [new_body]
        mod.write_text("".join(lines))
        print(f"APPLIED to {mod.relative_to(ROOT)} (signature/wrapper/callers still need manual wiring).")
    else:
        print("(dry run — rewritten body below; re-run with --apply to write)\n")
        print(new_body)


if __name__ == "__main__":
    main()
