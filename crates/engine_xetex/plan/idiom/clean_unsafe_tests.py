#!/usr/bin/env python3
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

def clean_file(path: Path):
    content = path.read_text(encoding="utf-8")
    
    # We want to find patterns like:
    #     #[test]
    #     fn test_name() {
    #         unsafe {
    #             body...
    #         }
    #     }
    # And transform them to:
    #     #[test]
    #     fn test_name() {
    #         body...
    #     }
    
    # We can do this with a stateful parser line-by-line
    lines = content.splitlines(keepends=True)
    out = []
    i = 0
    in_test = False
    in_unsafe_block = False
    unsafe_indent = ""
    
    while i < len(lines):
        line = lines[i]
        
        # Detect start of test
        if "#[test]" in line:
            in_test = True
            
        if in_test and "fn " in line and "{" in line:
            # We are inside the test fn
            pass
            
        if in_test and "unsafe {" in line:
            # We found the unsafe block inside the test
            in_unsafe_block = True
            # Skip this line (the unsafe { line)
            i += 1
            continue
            
        if in_unsafe_block and line.strip() == "}":
            # We found the closing brace of the unsafe block
            in_unsafe_block = False
            in_test = False
            # Skip this line (the } line)
            i += 1
            continue
            
        if in_unsafe_block:
            # Unindent by 4 spaces
            if line.startswith("            "):
                line = line[4:]
            elif line.startswith("\t\t\t"):
                line = line[1:]
                
        out.append(line)
        i += 1
        
    path.write_text("".join(out), encoding="utf-8")

def main():
    clean_file(ROOT / "src/helpers.rs")
    clean_file(ROOT / "src/scaled_math.rs")
    print("Cleaned unsafe blocks from tests in helpers.rs and scaled_math.rs.")

if __name__ == "__main__":
    main()
