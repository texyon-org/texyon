# 08 — Large-scale parity test plan (pre-idiomatization gate)

## Why this document exists

The C → Rust translation is essentially complete (session 132: ~4.8%
C LOC remaining, all of it `xetex-ini.c` glue and out-of-scope
third-party code). The next phase is **idiomatization**: restructuring
the Rust into clean, typed, borrow-checked code with `EngineState`,
real enums, iterators, etc. (see `00-overview.md` and `02-state.md`).

Idiomatization changes *internal structure* but must preserve
*observable output exactly*. The existing signals — 29 `tex-outputs`
cases, the engine_rust goldens, TRIP/eTRIP — were enough to catch
function-local translation bugs because every commit had a direct C
referent. Idiomatization is different: a refactor can silently shift
behavior across many functions at once, and there is no longer a "the C
version of this exact function" to diff against. We need a **much
larger, document-level safety net** that:

1. exercises real-world TeX/LaTeX/XeTeX features at scale, not toy
   snippets;
2. compares output **byte-for-byte against the engine as it was before
   any Rust was introduced** (the only definition of "correct" we
   trust);
3. is **cheap to rerun** so it can gate every idiomatization commit and
   run in CI.

This is the contract: *if the corpus is green, idiomatization did not
change engine semantics.* Everything below serves that contract.

## Success criteria

- A single command (`plan/scripts/corpus.sh` / a cargo test) compiles
  the whole corpus and reports pass/fail with first-divergence detail.
- Every corpus document's `engine_xetex` output (primarily **XDV**,
  plus `.log` and the format file) is **byte-identical** to the
  pre-Rust oracle, modulo a small, explicit, documented normalization
  set (dates, absolute paths).
- The corpus is tiered so a fast subset runs per-commit (seconds–low
  minutes) and the full set runs nightly / pre-milestone.
- Adding a new document is one drop-in step (place source, run the
  bless command, commit a small hash file).
- Re-running requires **only the candidate engine** in the common case
  (hash comparison); the oracle is only rebuilt when a divergence needs
  byte-level triage.

## 1. The oracle: the pre-Rust engine

The ground truth is the engine **before the first line of Rust**.

- **Baseline commit:** `1a99fcf5` ("Security: Fix 5 Dependabot alerts
  (#1348)") — the last commit before `231b13d2` (plan added) and
  `5806c164` (Phase 0 scaffolding). At `1a99fcf5`, `engine_xetex` is
  pure C with zero translation. Tag it:

  ```sh
  git tag oracle-pre-rust 1a99fcf5
  ```

- **Build the oracle in a dedicated worktree** so it stays available
  without disturbing the working tree:

  ```sh
  git worktree add ../tectonic-oracle oracle-pre-rust
  cargo build --release -p tectonic --manifest-path ../tectonic-oracle/Cargo.toml
  # binary: ../tectonic-oracle/target/release/tectonic
  ```

  Keep the worktree around for the duration of idiomatization. The
  oracle never changes.

- **The oracle binary is the reference.** We do *not* trust the
  current Rust engine to define correctness even for the goldens it
  currently passes — those goldens were themselves blessed from the
  translated engine. The pre-Rust C build is the independent authority.

> Note: a handful of post-baseline commits touched things *outside*
> `engine_xetex` (bundle plumbing, dependency bumps). If the oracle
> can't fetch the same bundle the current tree uses, cherry-pick only
> the bundle/config commits onto the oracle worktree — never an
> engine commit. Record any such cherry-picks at the top of
> `corpus/ORACLE.md` so the oracle's provenance is auditable.

## 2. Determinism prerequisites (do this before generating any golden)

Byte-identical comparison is only meaningful if both engines are run in
a fully reproducible configuration. Pin all of these for *both* oracle
and candidate:

1. **Build date / clock.** `engine_xetex` reads the clock for `\year`,
   `\month`, `\day`, `\time`, and the `.fmt`/`.log` banners
   (`TexEngine::build_date`, default `UNIX_EPOCH`). Force a fixed value
   via `SOURCE_DATE_EPOCH` and the engine's `build_date` /
   xdvipdfmx `deterministic_tags` (see
   `crates/engine_xetex/src/lib.rs:187`,
   `crates/engine_xdvipdfmx/src/lib.rs:40`). Pick one epoch and use it
   everywhere (e.g. `SOURCE_DATE_EPOCH=0`).
2. **Bundle pin.** Font/package resolution must be identical. Pin the
   exact bundle (URL + format version) and require it to be locally
   cached before a run; skip (do not fail) if absent, mirroring
   `tests/latex_goldens.rs`. Record the pinned bundle in
   `corpus/ORACLE.md`.
3. **Working directory & paths.** `.log` (and synctex) embed input
   paths. Run every doc from a controlled CWD with relative input
   names, exactly as `tests/util` already arranges, so paths are
   stable across machines.
4. **Locale / env.** Force `LC_ALL=C`, fixed `TZ=UTC`. ICU/HarfBuzz
   versions are baked into the build; the oracle worktree shares the
   same vendored bridge crates, so shaping is identical by
   construction.
5. **Threads.** The engine has global state; run single-threaded
   (`RUST_TEST_THREADS=1` for the cargo path, serial in the script),
   as `tests/trip.rs` already documents.

If any prerequisite can't be pinned for a given feature (e.g. a doc
that genuinely calls `\pdfcreationdate`), either normalize that field
(§4) or exclude the doc and note why.

## 3. What we compare, and in what priority

`engine_xetex`'s actual product is the **XDV** stream and the **format
file**; the PDF is produced downstream by `engine_xdvipdfmx` (not in
scope for translation). Prioritize accordingly:

| Artifact | Signal strength | Notes |
|----------|-----------------|-------|
| **`.xdv`** | **Primary.** The engine's true typeset output. | Compare byte-for-byte. A divergence here is an engine bug, full stop. |
| **`.fmt`** | **Primary (initex path).** | Compare SHA-256 of the *uncompressed* dump, exactly as `tests/formats.rs` does. Catches bugs in the dump/undump + initialization path that document runs may not reach. |
| **`.log`** | **Secondary.** | Byte-identical after normalization (§4). Catches print-routine, error-recovery, and tracing divergences that never reach shipout. |
| **`.pdf`** | **Tertiary / end-to-end smoke.** | Downstream of the engine; a weaker localization signal but a valuable full-pipeline check. Compare under deterministic tags. |
| **synctex** | **Optional.** | Already handled with rooting hacks in `tests/util`; include for docs where synctex is enabled. |

Rule: **XDV and `.fmt` divergences are hard failures.** `.log`
divergence after normalization is a hard failure. PDF-only divergence
(XDV identical) is investigated but flagged as likely-downstream.

## 4. Log normalization (the only sanctioned fudge)

`.log` comparison normalizes a *closed, documented* set of volatile
fields before diffing. Keep this list tiny and explicit — anything not
on it is a real difference:

- Engine banner version/date line.
- Absolute paths → replaced with `${ROOT}` (reuse the existing
  `tests/util` rooting machinery).
- Any field deliberately fed by the clock when `SOURCE_DATE_EPOCH`
  doesn't already neutralize it.

Implement normalization as a single function applied identically to
oracle and candidate logs. Never normalize XDV — if XDV needs
"normalization" to match, that is a bug to fix, not to paper over (see
`07-anti-patterns.md` on output-chasing).

## 5. The corpus

Goal: **breadth of real engine behavior**, with deliberate weight on
the XeTeX-specific paths that TRIP and the current goldens barely
touch (OpenType/`fontspec`, Unicode, bidi, Graphite, native-word
nodes, math). Organize into tiers by cost.

### Tier 0 — micro (existing, keep)
`tex-outputs` (29 cases), `engine_rust` goldens, TRIP + eTRIP,
`formats`. Already fast and authoritative for function-local bugs.
Runs on **every** commit. This plan *adds to*, never replaces, these.

### Tier 1 — curated medium documents (the per-commit workhorse)
20–60 self-contained documents that are known to compile cleanly in
Tectonic's bundle and finish in well under a minute each. Target
coverage:

- **Plain TeX / LaTeX core:** `lshort` ("Not So Short Introduction"),
  a `memoir`/`book` chapter, `article` with cross-refs, floats,
  `hyperref`, `amsmath`-heavy math, `tabular`/`booktabs`, `tikz`
  (a few small pictures), `listings`, `biblatex`/`natbib` doc + run.
- **XeTeX-specific (highest priority — least covered today):**
  - `fontspec` loading OpenType fonts from the bundle; explicit
    feature sets (ligatures, small caps, stylistic sets, OpenType math
    via `unicode-math`).
  - Unicode-heavy text (the existing `hallöchen 🐨 welt 🌍` case
    scaled up to a multi-page mixed-script document).
  - **Bidi:** Arabic and Hebrew paragraphs (`bidi`/`polyglossia`),
    mixed LTR/RTL — exercises `reverse`, edge nodes, the bidi paths in
    `hpack`/`shipout` that had real bugs during translation (sessions
    56, 116).
  - **Graphite** shaping (extends the existing `graphite_basic` case).
  - CJK text (line breaking via `do_locale_linebreaks`, native-word
    nodes).
- **Math stress:** `mathtools`, `unicode-math`, large displayed
  equations, matrices, `\left\right` delimiters, radicals, fractions —
  exercises the entire `src/math.rs` cluster (sessions 92–108).
- **Edge/regression docs:** any document that surfaced a bug during
  translation, frozen as a permanent case (the bidi GLUE-field bug,
  the fi-ligature `lig_result=2` bug, the `is_char_node`/`hi_mem_min`
  XDV bug — sessions 70, 116, 117).

Sources: package manuals shipping with TeX Live (each `.dtx`/`.tex`),
the LaTeX project's own `doc` sources, `lshort`, the `fontspec` /
`polyglossia` / `unicode-math` manuals (these *are* the XeTeX
showcase), and a few hand-written stress files we author.

### Tier 2 — bulk real-world corpus (nightly / pre-milestone)
Hundreds to a few thousand real documents for statistical confidence.

- **arXiv source dumps** are the canonical large LaTeX corpus
  (bulk-downloadable; thousands of papers across physics/math/CS with
  wildly varying package use). This is the "research papers" source.
- **CTAN package documentation** (every package ships typeset-able
  sources).
- **TUGboat** back-issue sources, the LaTeX companion examples,
  book-length open sources.

Tier 2 is **not** expected to be 100% compilable under Tectonic's
bundle (missing packages, shell-escape, external tools, `\write18`).
That's fine — see the curation pipeline (§6). The metric for Tier 2 is
*"of the documents the oracle compiles, 100% must match the
candidate."* A document the **oracle** can't compile is dropped from
the corpus, not counted as a candidate failure.

### Tier T — engine regression suite (`.lvt`, log-parity)
The arXiv / lshort / authored docs are all *generic typesetting* — they
exercise the engine the way real documents do, but they do **not**
deliberately probe edge cases. The **LaTeX kernel regression test
suite** does exactly that: hundreds of small `.lvt` files that
`\input{regression-test}` and then `\showbox` / `\showlists` /
`\tracingall` engine internals into the **`.log`**. This is the highest
edge-case density available.

Key differences from the document tiers:

- **The parity signal is the `.log`, not the `.xdv`.** Most `.lvt`
  tests never ship a page out (`No pages of output`); the log dump *is*
  the result. Compare oracle `.log` vs candidate `.log`.
- **Run non-halting.** Regression tests intentionally trigger errors to
  test recovery, so they must run with `-Z continue-on-errors` (and
  `-Z deterministic-mode`), and **not** `--halt-on-error`.
- **Support files required.** Tests `\input{test2e}` /
  `\input{regression-test}`, which live in the suite's `support/` dir,
  not the bundle. Put it on the path with
  `-Z search-path=$RAW/latex-tests/support` (which is why these runs
  drop `--untrusted` — that flag disables `search-path`).
- **Pass/"ran" criterion is `END-TEST-LOG` in the log**, not the exit
  code (the exit code is nonzero whenever a test fired an expected
  error).

Source: `github.com/latex3/latex2e` (`base/testfiles*`, ~1500 files,
incl. `testfiles-TU` = the Unicode/XeTeX-engine set — the most relevant
to us). `github.com/latex3/l3kernel` adds thousands more and can be
wired the same way later. Fetched by `corpus_fetch.sh lvt`. The list of
tests that run to `END-TEST-LOG` under the pinned engine is committed as
`tests/corpus/lvt-viable.txt` (a generated pass-list, not per-file
manifest rows — there are too many for that). Version skew with the
bundle's kernel will fail some; those are re-surveyed when the bundle is
upgraded, exactly like `tier2-pending`.

### Licensing & storage
Do **not** vendor arXiv/CTAN sources into the repo (size + licensing).
Tier 2 lives in an out-of-repo cache directory
(`$TECTONIC_CORPUS_DIR`, default `~/.cache/tectonic-corpus`) populated
by a fetch script. The repo stores only: Tier 0/1 sources we author or
that are clearly redistributable, and **hash goldens** (tiny) for all
tiers. The fetch script + a manifest (URL + checksum per doc) make
Tier 2 reproducible without committing the documents.

## 6. Corpus curation pipeline (Tier 2)

A document only enters the corpus if the **oracle** compiles it
cleanly. One-time (re-runnable) pipeline:

1. **Fetch** raw sources into `$TECTONIC_CORPUS_DIR/raw/<id>/`.
2. **Identify main file** (the `.tex` with `\documentclass`, or a
   `00README`/manifest hint).
3. **Trial-compile with the oracle** in deterministic mode.
   - Clean success (`TexOutcome::Spotless` or `Warnings`) → **accept**.
   - Errors, missing packages, needs shell-escape/external tools, or
     times out → **reject**, log reason to `corpus/REJECTED.tsv`.
4. **Generate goldens** from the oracle for accepted docs: store
   `sha256(xdv)`, `sha256(normalized log)`, and (optionally)
   `sha256(pdf)` in `corpus/goldens/<id>.json`, plus the oracle's
   `TexOutcome`.
5. **Stash exact bytes only on demand.** We don't store full XDV/PDF
   for thousands of docs. Because the oracle is reproducible, exact
   bytes for triage are regenerated by re-running the oracle on the one
   failing doc.

The accept/reject decision and goldens are the durable, committed
artifacts (small JSON + TSV). The raw sources are reproducible from the
manifest.

## 7. Golden representation & why it's cheap to rerun

- **Common case (green run):** run **only the candidate** engine on each
  doc, hash its outputs, compare to the stored hash. No oracle needed,
  no large files read. This is what makes per-commit reruns fast.
- **Divergence case:** when a candidate hash ≠ golden hash, the harness
  re-runs the **oracle** on that single doc to produce reference bytes,
  then performs a byte-level diff (XDV opcode disassembly, log text
  diff) and writes both `*.expected`/`*.observed` to disk for
  inspection (the `tests/util` `Expected` machinery already does this).
- **Bless workflow:** `TECTONIC_BLESS=1` regenerates hash goldens from
  the **oracle** (never from the candidate), matching the existing
  `engine_rust.rs` / `latex_goldens.rs` convention. Blessing from the
  candidate would defeat the entire purpose — guard against it in the
  script (require an `ORACLE_BIN` env var to bless).

## 8. Harness design

Two entry points sharing one core:

### a) `cargo test` integration (CI-friendly, Tier 0/1)
Extend the pattern in `tests/latex_goldens.rs`:
- Iterate Tier 1 docs, compile through the driver with the pinned
  bundle, compare hashes to `corpus/goldens/*.json`.
- **Skip (not fail)** when the bundle isn't cached, exactly as
  `latex_goldens.rs` does today.
- Honor `TECTONIC_BLESS=1` (oracle-sourced) for regeneration.

### b) `plan/scripts/corpus.sh` (scales to Tier 2, parallel-aware)
```
plan/scripts/corpus.sh [tier0|tier1|tier2|all] [--bless] [--jobs N]
```
- Resolves `ORACLE_BIN` (the worktree release binary) and
  `CANDIDATE_BIN` (current `cargo build --release`).
- For each doc: run candidate in deterministic mode, hash, compare.
- Aggregates a report: `N passed / M failed`, and for each failure the
  artifact that diverged + first-divergence location.
- Appends a one-line summary to `plan/corpus-history.log` (mirrors the
  existing `plan/parity-history.log` convention from `05-testing.md`).
- Parallelism: documents are independent *processes* (separate engine
  invocations), so `--jobs` can fan out safely even though the engine
  is internally single-threaded.

### Divergence triage (reuse `05-testing.md` §"failure diagnosis")
- **XDV:** disassemble both streams via the `xdv` crate
  (`crates/xdv/src/lib.rs`) and report the **first** divergent opcode +
  byte offset. Map it to the responsible area (shipout / packaging /
  linebreak) per `05-testing.md`.
- **log:** first differing line after normalization → usually a print
  routine or an error-recovery branch.
- **fmt:** falls back to a structured dump diff via `xetex_format`.
- Always chase the **first** difference; later diffs are cascades.

## 9. Directory layout

```
crates/engine_xetex/
  plan/
    08-parity-corpus.md         # this file
    scripts/
      corpus.sh                 # runner (tier0|tier1|tier2|all)
      corpus_fetch.sh           # populate $TECTONIC_CORPUS_DIR (Tier 2)
      corpus_bless.sh           # (re)generate goldens from ORACLE_BIN
    corpus-history.log          # appended pass counts per run
  tests/
    corpus/
      ORACLE.md                 # baseline commit, bundle pin, cherry-picks
      tier1/                    # authored/redistributable sources
        <stem>.tex
      goldens/
        <id>.json               # {xdv_sha, log_sha, pdf_sha?, outcome}
      REJECTED.tsv              # Tier 2 docs the oracle couldn't build + reason
      manifest.tsv              # Tier 2: id -> source URL + checksum
  tests/
    corpus.rs                   # cargo-test entry for Tier 0/1
```
(Tier 2 raw sources live outside the repo in `$TECTONIC_CORPUS_DIR`.)

## 10. Execution cadence

| Trigger | Scope | Budget |
|---------|-------|--------|
| Every idiomatization commit (local + CI) | Tier 0 + Tier 1 | seconds–minutes |
| Nightly CI | Tier 0 + 1 + a rotating Tier 2 sample (e.g. 200 docs) | tens of minutes |
| Pre-milestone / pre-merge of a big refactor | Full Tier 2 | hours, parallel |

`PROGRESS.md` gains a corpus column alongside the existing parity
count. The green-build-streak rule (`05-testing.md`) extends to the
corpus: **no idiomatization commit may reduce the corpus pass count.**

## 11. Phased rollout

1. **Pin & build the oracle.** Tag `oracle-pre-rust`, create the
   worktree, build release, write `corpus/ORACLE.md` (commit, bundle,
   epoch). Verify the oracle compiles the 29 existing `tex-outputs`
   docs and that its XDVs match today's goldens — this validates the
   oracle path end-to-end before we trust it.
2. **Lock determinism.** Wire `SOURCE_DATE_EPOCH` / `build_date` /
   deterministic tags / `LC_ALL`/`TZ` into both the script and the
   cargo harness. Prove a single doc is byte-identical across two
   independent oracle runs.
3. **Build the comparison core + triage.** Hash compare, on-divergence
   oracle re-run, XDV opcode diff, log normalization. Land
   `tests/corpus.rs` and `plan/scripts/corpus.sh`.
4. **Assemble Tier 1.** Author/collect 20–60 curated docs with strong
   XeTeX coverage (fontspec, bidi, unicode-math, CJK, Graphite). Bless
   from the oracle. This is the per-commit gate going forward.
5. **Stand up Tier 2 pipeline.** `corpus_fetch.sh` + curation +
   manifest + bless. Start with a few hundred arXiv/CTAN docs; grow.
6. **Wire CI cadence** (§10) and extend `PROGRESS.md`.
7. **Only then begin idiomatization**, running Tier 0/1 per commit and
   Tier 2 at milestones.

## 12. Risks & mitigations

- **Non-determinism leaks** (a doc using clock/random/`\pdfcreationdate`):
  caught immediately because two oracle runs won't match in step 2 —
  add to normalization or exclude with a logged reason.
- **Bundle drift** (a re-resolved font/package changes output): bundle
  is pinned and required-cached; CI fails loudly if the pin moves.
- **Oracle rot** (oracle stops building on a new toolchain): the
  worktree is built once and the binary archived; rebuild instructions
  + exact toolchain in `corpus/ORACLE.md`.
- **Corpus too slow for per-commit:** that's why Tier 1 is bounded and
  Tier 2 is nightly/milestone. Hash-only green runs keep Tier 1 cheap.
- **Blessing from the wrong engine:** the bless path *requires*
  `ORACLE_BIN` and refuses to source goldens from the candidate.
- **Storage blowup:** only hashes + small authored sources are
  committed; Tier 2 bytes stay in the cache, reproducible from the
  manifest.
- **Platform shaping differences** (ICU/HarfBuzz/freetype versions):
  oracle and candidate share the same vendored bridge crates and are
  built on the same host, so shaping matches by construction; document
  the host in `ORACLE.md` and treat cross-platform parity as a separate
  concern, out of scope here.

## 13. Open questions (decide during phase 1–2)

- Which exact bundle version/URL to pin? (Use whatever the current tree
  resolves; record it.)
- One canonical `SOURCE_DATE_EPOCH` value — propose `0`.
- Tier 2 initial size and source split (how many arXiv vs CTAN)?
- Do we gate PDF (tertiary) in CI, or only XDV+log+fmt, given PDF is
  downstream of the engine under test?
- Timeout per document for Tier 2 trial compiles?
```

## 14. Bulk arXiv retrieval (MANUAL — not automated)

Scaling Tier 2 to hundreds/thousands of papers cannot be done from this
agent: the public arXiv API rate-limits aggressively (HTTP 429 after a
short burst, even with delays), and the canonical bulk dataset is on a
**requester-pays** S3 bucket that needs your own AWS credentials and
incurs (small) egress charges. Both require human-owned credentials /
acceptable-use decisions. Do one of the following.

### Option A — requester-pays S3 bulk dataset (recommended for scale)
arXiv publishes all source files as monthly `.tar` files under
`s3://arxiv/src/` (US-East-1), requester-pays.

```sh
# one-time: an AWS account + credentials with billing enabled
aws configure                 # set access key / secret / region us-east-1

# list the source tarballs (note --request-payer)
aws s3 ls s3://arxiv/src/ --request-payer requester

# pull one monthly chunk (each ~ a few GB; this is the cost-incurring step)
aws s3 cp s3://arxiv/src/arXiv_src_2403_001.tar . --request-payer requester

# the .tar contains per-paper gzipped sources; unpack into the corpus cache
tar xf arXiv_src_2403_001.tar -C "$TECTONIC_CORPUS_DIR/raw/arxiv-bulk"
```
Docs: https://info.arxiv.org/help/bulk_data_s3.html — read the pricing
note first. Pull a handful of monthly chunks to reach ~1–2k papers,
then run the curation pipeline (§6) so the **oracle** decides which
compile.

### Option B — polite API harvesting (no AWS, slower, hundreds not thousands)
If you only want a few hundred papers and want to avoid S3, harvest via
the API with **strict** rate limiting (arXiv asks ~1 request / 3 s; back
off hard on 429):

```sh
# Use ONE query per run, paginate with `start`, sleep >=3s between calls,
# and exponential-backoff on HTTP 429. Do NOT use sortBy/sortOrder — that
# query path times out. Then fetch each https://arxiv.org/e-print/<id>.
# IMPORTANT: ~20% of papers are PDF-only (no TeX source); `file` each
# payload and discard any that is a "PDF document" (see corpus_fetch.sh).
```
This is the same logic `corpus_fetch.sh fetch_arxiv` uses for the
per-ID list — extend `manifest.tsv` with more `tier2` rows and rerun.
Expect it to take a long wall-clock time due to the rate limit.

### Status of the starter set fetched so far
- `1606.00915` and `2012.12757` — real arXiv LaTeX sources, in the cache
  and in `manifest.tsv`. Enough to prove the Tier 2 pipeline end-to-end.
- `2012.12675` was fetched but is **PDF-only** (no source) and was
  discarded — the curation pipeline must always `file`-check payloads.

### CTAN / TUGboat (also manual, lower priority)
CTAN package sources and TUGboat back issues are bulk-downloadable over
plain HTTPS (no rate-limit drama) but each needs per-package handling of
build deps; defer until the arXiv set is exhausted as a coverage source.
