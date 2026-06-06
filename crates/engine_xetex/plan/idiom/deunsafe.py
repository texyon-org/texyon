#!/usr/bin/env python3
"""De-unsafe codemod for workstream 03 / overview target #4.

Attempts to remove the `unsafe` keyword from function signatures one-by-one.
Verifies via `cargo check` whether the change is valid (i.e. the function body
does not perform any unsafe operations that would require an unsafe block, and
all other module interfaces remain compatible).

Usage:
    python3 plan/idiom/deunsafe.py src/scaled_math.rs --apply
"""
import argparse
import subprocess
import sys
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src"

UNSAFE_FN_RE = re.compile(
    r"^(?P<indent>\s*)(?P<pub>(?:pub(?:\([^)]*\))?\s+)?)(?P<unsafe>unsafe\s+)(?P<extern>(?:extern\s+\"[^\"]*\"\s+)?)(?P<fn>fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*))"
)

def run_cargo_check() -> bool:
    res = subprocess.run(
        ["cargo", "check", "--lib"],
        cwd=str(ROOT),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return res.returncode == 0

def process_file(path: Path, apply: bool):
    print(f"Processing {path.relative_to(ROOT)}...")
    content = path.read_text(encoding="utf-8")
    lines = content.splitlines(keepends=True)
    
    # First locate all unsafe functions
    unsafe_indices = []
    for idx, line in enumerate(lines):
        m = UNSAFE_FN_RE.match(line)
        if m:
            # We want to skip external FFI declarations or Keep set if any
            name = m.group("name")
            # Don't try to de-unsafe if it is an extern "C" block (e.g. extern "C" fn ...)
            # which might have different rules, though we can check if it works.
            unsafe_indices.append((idx, name))
            
    if not unsafe_indices:
        print("  No unsafe functions found.")
        return
        
    print(f"  Found {len(unsafe_indices)} unsafe functions. Testing them one-by-one...")
    
    deunsafed_count = 0
    deunsafed_names = []
    
    # For each unsafe function, try to make it safe
    for idx, name in unsafe_indices:
        original_line = lines[idx]
        # Replace the first occurrence of "unsafe " with "" on that line
        # e.g., `pub unsafe fn` -> `pub fn`
        # `unsafe fn` -> `fn`
        new_line = original_line.replace("unsafe ", "", 1)
        
        # Write the modification
        lines[idx] = new_line
        path.write_text("".join(lines), encoding="utf-8")
        
        # Check if it compiles
        if run_cargo_check():
            # Success! Keep the change.
            deunsafed_count += 1
            deunsafed_names.append(name)
            print(f"  SUCCESS: Made '{name}' safe (line {idx+1})")
        else:
            # Failure. Revert the change.
            lines[idx] = original_line
            path.write_text("".join(lines), encoding="utf-8")
            print(f"  FAILED: '{name}' requires unsafe (line {idx+1})")
            
    print(f"File {path.name} summary: Made {deunsafed_count} functions safe: {', '.join(deunsafed_names)}")

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="*")
    ap.add_argument("--all", action="store_true")
    args = ap.parse_args()
    
    files = (
        sorted(SRC.glob("*.rs")) if args.all
        else [ROOT / f if not Path(f).is_absolute() else Path(f) for f in args.files]
    )
    if not files:
        ap.error("give files or --all")
        
    # Check if baseline compiles first
    print("Checking baseline compilation...")
    if not run_cargo_check():
        print("Error: baseline does not compile!")
        sys.exit(1)
        
    for f in files:
        process_file(f, True)

if __name__ == "__main__":
    main()
