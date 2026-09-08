"""Byte-for-byte comparison of the pyRegTab CLI runner with the Java runner of
regtab-eval-on-atbench on every solution of the evaluation (244 RTL files):
both runners are executed with the harness contract (``./input.csv`` →
``./output.csv`` in a temporary directory, ``harness.run.runner_cmd(engine)``)
and their exit codes and ``output.csv`` bytes must be equal.

The Java outputs do not change between pyRegTab builds, so they are cached in
``--cache`` (default: ``<atbench repo>/.bytecmp-java``); delete the directory
to re-run Java.

Usage: python tools/atbench_bytecmp.py [--repo <regtab-eval-on-atbench>]
                                        [--cache <dir>] [--case <id> ...]
Exit code 0 when all solutions agree, 1 otherwise.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def run_engine(cmd: list[str], case, timeout: int) -> tuple[int, bytes | None, str]:
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        shutil.copy2(case.data_csv, tmpdir / "input.csv")
        proc = subprocess.run(cmd + [str(case.solution)], cwd=tmpdir, capture_output=True, timeout=timeout)
        out = tmpdir / "output.csv"
        data = out.read_bytes() if out.is_file() else None
        return proc.returncode, data, proc.stderr.decode("utf-8", "replace")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", default=os.environ.get("ATBENCH_EVAL_REPO", r"D:\YandexDisk\code2\regtab-eval-on-atbench"))
    ap.add_argument("--cache", default=None)
    ap.add_argument("--case", action="append")
    ap.add_argument("--timeout", type=int, default=600)
    args = ap.parse_args(argv)

    repo = Path(args.repo).resolve()
    sys.path.insert(0, str(repo))
    from harness.cases import Case, all_case_ids  # noqa: E402
    from harness.run import runner_cmd  # noqa: E402

    cache = Path(args.cache) if args.cache else repo / ".bytecmp-java"
    cache.mkdir(parents=True, exist_ok=True)
    ids = args.case or [c for c in all_case_ids() if Case(c).solution.is_file()]
    java_cmd, py_cmd = runner_cmd("java"), runner_cmd("pyregtab")

    mismatches: list[str] = []
    started = time.perf_counter()
    for i, cid in enumerate(ids, 1):
        case = Case(cid)
        cached_out, cached_rc = cache / f"{cid}.csv", cache / f"{cid}.rc"
        if cached_rc.is_file():
            j_rc = int(cached_rc.read_text())
            j_out = cached_out.read_bytes() if cached_out.is_file() else None
        else:
            j_rc, j_out, _ = run_engine(java_cmd, case, args.timeout)
            cached_rc.write_text(str(j_rc))
            if j_out is not None:
                cached_out.write_bytes(j_out)
        p_rc, p_out, p_err = run_engine(py_cmd, case, args.timeout)
        ok = (j_rc == p_rc) and (j_out == p_out)
        if not ok:
            detail = f"rc java={j_rc} py={p_rc}"
            if j_out is not None and p_out is not None:
                detail += (f", bytes java={len(j_out)} py={len(p_out)}, sha java="
                           f"{hashlib.sha256(j_out).hexdigest()[:12]} py={hashlib.sha256(p_out).hexdigest()[:12]}")
            if p_err.strip():
                detail += f"; stderr: {p_err.strip().splitlines()[-1]}"
            mismatches.append(f"{cid}: {detail}")
        print(f"[{i}/{len(ids)}] {cid}: {'ok' if ok else 'MISMATCH'}", flush=True)
    print(f"\n{len(ids)} solutions, {len(mismatches)} mismatches, {time.perf_counter() - started:.0f} s")
    for m in mismatches:
        print("  " + m)
    return 1 if mismatches else 0


if __name__ == "__main__":
    sys.exit(main())
