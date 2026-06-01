#!/usr/bin/env python3
"""Performance harness for Phase-C engine threading work.

The main mode benchmarks the same candidate/oracle command paths used by the
large corpus parity gate (`plan/scripts/corpus.sh`): authored/real documents are
compared on XDV bytes, and LaTeX `.lvt` regression tests are compared on
normalized logs. Each case is run under both the pre-Rust oracle and the current
candidate, so the output is both a performance comparison and a parity check.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path


SCOPES = {"all", "docs", "lvt", "tier1", "tier1-real", "tier2"}


@dataclass(frozen=True)
class CorpusCase:
    kind: str
    tier: str
    case_id: str
    relpath: str


@dataclass
class TimedRun:
    seconds: float
    digest: str


def repo_root() -> Path:
    return Path(__file__).resolve().parents[4]


def corpus_root(root: Path) -> Path:
    return root / "crates" / "engine_xetex" / "tests" / "corpus"


def want_scope(scope: str, tier: str) -> bool:
    if scope == "all":
        return True
    if scope == "docs":
        return tier != "lvt"
    if scope == "lvt":
        return tier == "lvt"
    return tier == scope


def load_corpus_cases(root: Path, cache: Path, scope: str) -> list[CorpusCase]:
    corpus = corpus_root(root)
    cases: list[CorpusCase] = []

    manifest = corpus / "manifest.tsv"
    with manifest.open(encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            tier, case_id, relpath, _source = line.split("\t", 3)
            if tier not in {"tier1", "tier1-real", "tier2"}:
                continue
            if want_scope(scope, tier):
                main = corpus / relpath if tier == "tier1" else cache / relpath
                if main.exists():
                    cases.append(CorpusCase("doc", tier, case_id, relpath))

    lvt_list = corpus / "lvt-viable.txt"
    if want_scope(scope, "lvt"):
        with lvt_list.open(encoding="utf-8") as f:
            for line in f:
                relpath = line.strip()
                if not relpath or relpath.startswith("#"):
                    continue
                src = cache / "latex-tests" / "base" / f"{relpath}.lvt"
                if src.exists():
                    cases.append(CorpusCase("lvt", "lvt", relpath, relpath))

    return cases


def run_env() -> dict[str, str]:
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "0"
    env["LC_ALL"] = "C"
    env["TZ"] = "UTC"
    return env


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def normalize_log(path: Path, work_root: Path) -> bytes:
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = []
    for line in text.splitlines():
        if line.startswith("This is "):
            continue
        if line.startswith("entering extended mode"):
            continue
        lines.append(line)
    text = "\n".join(lines) + "\n"
    work_pat = re.escape(str(work_root))
    text = re.sub(work_pat + r"[^ ]*", "", text)
    return text.encode("utf-8")


def digest_output(case: CorpusCase, work_dir: Path, work_root: Path) -> str:
    if case.kind == "doc":
        xdvs = sorted(work_dir.glob("*.xdv"))
        if not xdvs:
            raise RuntimeError(f"{case.case_id}: no XDV produced")
        return sha256_bytes(xdvs[0].read_bytes())

    stem = Path(case.relpath).name
    log = work_dir / f"{stem}.log"
    if not log.exists():
        raise RuntimeError(f"{case.case_id}: no log produced")
    return sha256_bytes(normalize_log(log, work_root))


def command_for_case(
    binary: Path,
    case: CorpusCase,
    *,
    root: Path,
    cache: Path,
    work_dir: Path,
) -> list[str]:
    corpus = corpus_root(root)
    if case.kind == "doc":
        main = corpus / case.relpath if case.tier == "tier1" else cache / case.relpath
        return [
            str(binary),
            "-X",
            "compile",
            "--outfmt",
            "xdv",
            "--keep-logs",
            "--untrusted",
            "-Z",
            "deterministic-mode",
            "-o",
            str(work_dir),
            str(main),
        ]

    stem = Path(case.relpath).name
    src = cache / "latex-tests" / "base" / f"{case.relpath}.lvt"
    dst = work_dir / f"{stem}.tex"
    shutil.copyfile(src, dst)
    support = cache / "latex-tests" / "support"
    return [
        str(binary),
        "-X",
        "compile",
        "--keep-logs",
        "-Z",
        "deterministic-mode",
        "-Z",
        "continue-on-errors",
        "-Z",
        f"search-path={support}",
        "-o",
        str(work_dir),
        str(dst),
    ]


def run_case_once(
    binary: Path,
    case: CorpusCase,
    *,
    root: Path,
    cache: Path,
    work_root: Path,
    label: str,
) -> TimedRun:
    safe_id = re.sub(r"[^A-Za-z0-9_.-]+", "_", case.case_id)
    work_dir = work_root / label / safe_id
    if work_dir.exists():
        shutil.rmtree(work_dir)
    work_dir.mkdir(parents=True)

    cmd = command_for_case(binary, case, root=root, cache=cache, work_dir=work_dir)
    start = time.perf_counter()
    proc = subprocess.run(
        cmd,
        cwd=root,
        env=run_env(),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    seconds = time.perf_counter() - start
    (work_dir / "run.txt").write_bytes(proc.stdout)
    try:
        digest = digest_output(case, work_dir, work_root)
    except RuntimeError as exc:
        if proc.returncode != 0:
            raise RuntimeError(
                f"{case.case_id}: {label} exited {proc.returncode} and {exc}; "
                f"see {work_dir / 'run.txt'}"
            ) from exc
        raise
    return TimedRun(seconds, digest)


def median(values: list[float]) -> float:
    return statistics.median(values)


def rel_stdev(values: list[float]) -> float:
    if len(values) < 2:
        return math.nan
    mean = statistics.mean(values)
    if mean == 0:
        return math.nan
    return statistics.stdev(values) / mean


def fmt_seconds(value: float) -> str:
    if value >= 1:
        return f"{value:.3f}s"
    return f"{value * 1000:.1f}ms"


def summarize_row(case: CorpusCase, oracle: list[float], candidate: list[float]) -> dict:
    oracle_median = median(oracle)
    candidate_median = median(candidate)
    ratio = candidate_median / oracle_median if oracle_median else math.inf
    return {
        "case": case.case_id,
        "kind": case.kind,
        "tier": case.tier,
        "oracle": oracle,
        "candidate": candidate,
        "oracle_median": oracle_median,
        "candidate_median": candidate_median,
        "ratio": ratio,
        "candidate_relative_stdev": rel_stdev(candidate),
        "oracle_relative_stdev": rel_stdev(oracle),
    }


def print_row(row: dict) -> None:
    print(
        f"{row['case'][:42]:<42} {row['tier']:<10} "
        f"oracle {fmt_seconds(row['oracle_median']):>8} "
        f"candidate {fmt_seconds(row['candidate_median']):>8} "
        f"ratio {row['ratio']:>6.3f}x"
    )


def corpus_benchmark(args: argparse.Namespace) -> int:
    root = repo_root()
    cache = Path(
        os.environ.get("TECTONIC_CORPUS_DIR", str(Path.home() / ".cache" / "tectonic-corpus"))
    ) / "raw"
    oracle = Path(args.oracle_bin or os.environ.get("ORACLE_BIN", ""))
    candidate = Path(args.candidate_bin or os.environ.get("CANDIDATE_BIN", root / "target" / "release" / "tectonic"))

    if not oracle.is_file() or not os.access(oracle, os.X_OK):
        raise SystemExit("set --oracle-bin or ORACLE_BIN to the executable pre-Rust oracle")
    if not candidate.is_file() or not os.access(candidate, os.X_OK):
        raise SystemExit("set --candidate-bin/CANDIDATE_BIN or build target/release/tectonic")

    cases = load_corpus_cases(root, cache, args.scope)
    if args.case:
        wanted = set(args.case)
        cases = [case for case in cases if case.case_id in wanted or case.relpath in wanted]
    if args.limit is not None:
        cases = cases[: args.limit]
    if not cases:
        raise SystemExit("no corpus cases selected; check corpus cache and scope")

    print(f"repo: {root}")
    print(f"oracle: {oracle}")
    print(f"candidate: {candidate}")
    print(f"scope: {args.scope}; cases: {len(cases)}")
    print(f"warmups: {args.warmups}; iterations: {args.iterations}")

    rows = []
    parity_failures: list[str] = []
    with tempfile.TemporaryDirectory(prefix="tectonic-perf-") as tmp:
        work_root = Path(tmp)
        for idx, case in enumerate(cases, 1):
            oracle_times: list[float] = []
            candidate_times: list[float] = []
            oracle_digest = None
            candidate_digest = None

            total_runs = args.warmups + args.iterations
            for run_idx in range(total_runs):
                # Alternate order to reduce systematic cache/thermal bias.
                order = ("oracle", "candidate")
                if run_idx % 2 == 1:
                    order = ("candidate", "oracle")
                for label in order:
                    binary = oracle if label == "oracle" else candidate
                    timed = run_case_once(
                        binary,
                        case,
                        root=root,
                        cache=cache,
                        work_root=work_root,
                        label=f"{label}-{idx}-{run_idx}",
                    )
                    if label == "oracle":
                        oracle_digest = timed.digest
                        if run_idx >= args.warmups:
                            oracle_times.append(timed.seconds)
                    else:
                        candidate_digest = timed.digest
                        if run_idx >= args.warmups:
                            candidate_times.append(timed.seconds)

            if oracle_digest != candidate_digest:
                parity_failures.append(case.case_id)

            row = summarize_row(case, oracle_times, candidate_times)
            rows.append(row)
            if args.verbose or idx == len(cases) or idx % args.progress_every == 0:
                print_row(row)

    oracle_total = sum(row["oracle_median"] for row in rows)
    candidate_total = sum(row["candidate_median"] for row in rows)
    ratio = candidate_total / oracle_total if oracle_total else math.inf

    print("-" * 92)
    print(
        f"median-sum oracle {fmt_seconds(oracle_total)}; "
        f"candidate {fmt_seconds(candidate_total)}; ratio {ratio:.3f}x"
    )
    worst = sorted(rows, key=lambda row: row["ratio"], reverse=True)[: min(10, len(rows))]
    print("slowest candidate/oracle ratios:")
    for row in worst:
        print_row(row)

    if parity_failures:
        print("parity failures:")
        for case_id in parity_failures:
            print(f"  {case_id}")

    if args.json:
        args.json.write_text(
            json.dumps(
                {
                    "scope": args.scope,
                    "warmups": args.warmups,
                    "iterations": args.iterations,
                    "oracle": str(oracle),
                    "candidate": str(candidate),
                    "rows": rows,
                    "parity_failures": parity_failures,
                    "median_sum_ratio": ratio,
                },
                indent=2,
            ),
            encoding="utf-8",
        )
        print(f"wrote {args.json}")

    return 1 if parity_failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scope", choices=sorted(SCOPES), default="all")
    parser.add_argument("--oracle-bin", help="pre-Rust oracle executable")
    parser.add_argument("--candidate-bin", help="candidate executable")
    parser.add_argument("--case", action="append", help="case id/path to include")
    parser.add_argument("--limit", type=int, help="only run the first N selected cases")
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--iterations", type=int, default=5)
    parser.add_argument("--progress-every", type=int, default=25)
    parser.add_argument("--verbose", action="store_true")
    parser.add_argument("--json", type=Path, help="write detailed JSON results")
    args = parser.parse_args()

    if args.warmups < 0:
        raise SystemExit("--warmups must be >= 0")
    if args.iterations <= 0:
        raise SystemExit("--iterations must be > 0")
    if args.limit is not None and args.limit <= 0:
        raise SystemExit("--limit must be > 0")
    if args.progress_every <= 0:
        raise SystemExit("--progress-every must be > 0")

    return corpus_benchmark(args)


if __name__ == "__main__":
    raise SystemExit(main())
