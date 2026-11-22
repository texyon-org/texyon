"""XDV pages decoded to one text line per command, for diffing."""

import struct
from pathlib import Path

PRE, POST, POST_POST = 247, 248, 249
BOP = 139
NATIVE_FONT, GLYPHS, TEXT_GLYPHS = 252, 253, 254
# Native font flags that each add a 4-byte field: colour, extend, slant, embolden.
NATIVE_FONT_FIELDS = (0x200, 0x1000, 0x2000, 0x4000)
ZERO_MOVES = {147: "w0", 152: "x0", 161: "y0", 166: "z0"}
# First opcode of each 1-to-4-byte family, with the name it prints as.
MOVES = {143: "right", 148: "w", 153: "x", 157: "down", 162: "y", 167: "z"}


class Reader:
    def __init__(self, data):
        self.data = data
        self.pos = 0

    def unsigned(self, n):
        value = int.from_bytes(self.data[self.pos:self.pos + n], "big")
        self.pos += n
        return value

    def signed(self, n):
        value = int.from_bytes(self.data[self.pos:self.pos + n], "big", signed=True)
        self.pos += n
        return value

    def take(self, n):
        chunk = self.data[self.pos:self.pos + n]
        self.pos += n
        return chunk

    def glyphs(self):
        width, count = self.signed(4), self.unsigned(2)
        positions = [self.signed(4) for _ in range(2 * count)]
        ids = [self.unsigned(2) for _ in range(count)]
        return f"{width} {positions} {ids}"


def command(r, op):
    """The line for one command, or None at the end of the pages."""
    if op <= 127:
        return f"setchar {op}"
    if op <= 131:
        return f"set{op - 127} {r.unsigned(op - 127)}"
    if op == 132:
        return f"setrule {r.signed(4)} {r.signed(4)}"
    if op <= 136:
        return f"put{op - 132} {r.unsigned(op - 132)}"
    if op == 137:
        return f"putrule {r.signed(4)} {r.signed(4)}"
    if op == 138:
        return "nop"
    if op == BOP:
        fields = r.take(44)
        return f"bop {struct.unpack('>10i', fields[:40])}" if len(fields) == 44 else None
    if op == 140:
        return "eop"
    if op == 141:
        return "push"
    if op == 142:
        return "pop"
    if op in ZERO_MOVES:
        return ZERO_MOVES[op]
    if 143 <= op <= 170:
        first = max(f for f in MOVES if f <= op)
        return f"{MOVES[first]} {r.signed(op - first + 1)}"
    if op <= 234:
        return f"fnt {op - 171}"
    if op <= 238:
        return f"fnt {r.unsigned(op - 234)}"
    if op <= 242:
        # kpathsea's paths print as ./name, TeXyoN's as name.
        return f"xxx {r.take(r.unsigned(op - 238)).replace(b'(./', b'(')!r}"
    if op <= 246:
        font = r.unsigned(op - 242)
        checksum, size, design = r.unsigned(4), r.unsigned(4), r.unsigned(4)
        area, name = r.unsigned(1), r.unsigned(1)
        return f"fntdef {font} {checksum} {size} {design} {r.take(area + name).decode('latin-1')}"
    if op == POST:
        r.take(4)
        fields = [r.unsigned(4) for _ in range(5)] + [r.unsigned(2) for _ in range(2)]
        return "post " + " ".join(map(str, fields))
    if op == NATIVE_FONT:
        font, size, flags = r.unsigned(4), r.unsigned(4), r.unsigned(2)
        # Upstream records the path kpathsea found.
        name = r.take(r.unsigned(1)).rsplit(b"/", 1)[-1]
        index = r.unsigned(4)
        extra = [r.unsigned(4) for bit in NATIVE_FONT_FIELDS if flags & bit]
        return f"nativefont {font} {size} {flags} {name!r} {index} {extra}"
    if op == GLYPHS:
        return f"glyphs {r.glyphs()}"
    if op == TEXT_GLYPHS:
        text = r.take(2 * r.unsigned(2))
        return f"textglyphs {text!r} {r.glyphs()}"
    return f"<op {op}>"


def pages(path: Path):
    """The decoded commands from the preamble to the postamble."""
    if not path.exists():
        return ["<no xdv>"]
    r = Reader(path.read_bytes())
    if r.unsigned(1) != PRE:
        return ["<bad xdv>"]
    r.unsigned(1)
    out = [f"pre {r.unsigned(4)} {r.unsigned(4)} {r.unsigned(4)}"]
    r.take(r.unsigned(1))
    while r.pos < len(r.data):
        op = r.unsigned(1)
        if op == POST_POST:
            break
        line = command(r, op)
        if line is None:
            break
        out.append(line)
        if line.startswith("<op"):
            break
    # A run killed at its file size limit leaves the XDV cut off.
    if r.pos > len(r.data):
        out.append("<truncated xdv>")
    return out
