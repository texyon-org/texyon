# Third-party notices

TeXyoN's own code is MIT licensed (see `LICENSE`). It grew out of
[Tectonic](https://tectonic-typesetting.github.io/) and contains code from
several other projects, listed below with their licenses. The full license
texts referred to here are in `licenses/`.

## What this means for builds

The source tree is MIT apart from the directories named below. Built programs
link some of that code, so they carry its terms:

| Build | Links | Distributed under |
| --- | --- | --- |
| `texyon` without MuPDF | dvipdfmx | GPL-2.0-or-later |
| `texyon` with `preview-mupdf`, the VS Code extension's binary, the wasm engine | dvipdfmx, MuPDF | AGPL-3.0-or-later |

In both cases the complete source is this repository.

## Tectonic

Most of this repository started as Tectonic, and the Rust engine is a port of
Tectonic's C translation of XeTeX.

Copyright (c) 2016-2026 the Tectonic Project. MIT License.

## XeTeX, e-TeX and TeX

The engine follows `xetex.web` from TeX Live, which is built on e-TeX and on
Donald E. Knuth's TeX. TeXyoN is not TeX. That name is reserved for Knuth's
program. TeX is a trademark of the American Mathematical Society.

XeTeX is distributed under this license:

```
Copyright (c) 1994-2008 by SIL International
Copyright (c) 2009-2022 by Jonathan Kew
Copyright (c) 2010-2012 by Han The Thanh
Copyright (c) 2012-2013 by Khaled Hosny

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the name of the copyright holders shall
not be used in advertising or otherwise to promote the sale, use or other
dealings in this Software without prior written authorization from the
copyright holders.
```

## BibTeX

`crates/engine_bibtex` is Tectonic's port of `bibtex.web` by Oren Patashnik.

## dvipdfmx

`crates/pdf_io/pdf_io` and `crates/engine_xdvipdfmx/xdvipdfmx` are dvipdfmx,
used to write PDF files and to read images and PDF figures, synced with the
TeX Live 2026 sources (dvipdfm-x 20260317).

Copyright (C) 2002-2026 by Jin-Hwan Cho, Matthias Franz, Shunsaku Hirata and
the dvipdfmx project team. Copyright (C) 1998, 1999 by Mark A. Wicks.
Copyright (c) 2006 SIL International (XeTeX extensions). Copyright (C)
2012-2015 by Khaled Hosny.

GNU General Public License, version 2 or later (`licenses/GPL-2.0.txt`).

## TECkit

`crates/engine_xetex/xetex/teckit-*` is the TECkit mapping engine.

Copyright (C) 2002-2016 SIL International. Used under the GNU Lesser General
Public License, version 2.1 or later (`licenses/LGPL-2.1.txt`). TECkit is also
available under the Common Public License.

## MuPDF

The preview renderer can link MuPDF (the `preview-mupdf` feature, and always in
the wasm engine). MuPDF builds in jbig2dec and extract (AGPL-3.0-or-later),
lcms2 (MIT), OpenJPEG (BSD-2-Clause), libjpeg (IJG license) and Gumbo
(Apache-2.0). Their notices ship with the MuPDF source in `thirdparty/`.

Copyright (C) Artifex Software, Inc. GNU Affero General Public
License, version 3 or later (`licenses/AGPL-3.0.txt`).

## Libraries linked by the native and wasm builds

Depending on the platform these are linked from the system or built into the
binary:

- **HarfBuzz**, Old MIT license. Source in `crates/bridge_harfbuzz/harfbuzz`,
  notice in its `COPYING`.
- **FreeType**, used under the FreeType License. Portions of this software are
  copyright (C) 2006-2025 The FreeType Project (www.freetype.org). All rights
  reserved.
- **ICU**, Unicode License v3. Copyright (C) 2016-2025 Unicode, Inc.
- **Graphite2**, used under the Mozilla Public License 2.0. Copyright (C) 2010
  SIL International.
- **Fontconfig**, MIT-style license. Copyright (C) 2000-2007 Keith Packard and
  others.
- **libpng** (libpng license), **zlib** (zlib license), and the compression and
  XML libraries these depend on, each under its own permissive license.

Rust crates from crates.io are listed in `Cargo.lock` and keep their own
licenses, almost all MIT or Apache-2.0.

## TeX Live

TeXyoN downloads TeX Live packages (classes, macros and fonts) when a document
needs them. They are not part of this repository, and each keeps its own
license, most often the LaTeX Project Public License.
