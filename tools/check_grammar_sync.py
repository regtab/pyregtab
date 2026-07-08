"""Verify that the pinned normative grammar copy is in sync (plan §5.2).

`grammar/RTL.g4` is a verbatim copy of the normative RTL grammar from jRegTab.
It is a reference document only — pyRegTab's parser is hand-written in Rust — so
nothing generates from it and it could silently drift. This check keeps it honest:

1. Offline (always): the SHA-256 of `grammar/RTL.g4` must equal the `sha256:`
   recorded in `grammar/UPSTREAM`. Catches any local edit of the copy.
2. Upstream (opt-in): if a `JREGTAB_TOKEN` env var with read access to the
   private jRegTab repo is set, fetch the grammar at the pinned commit and assert
   it is byte-identical to the local copy. Catches upstream drift. Skipped
   (with a notice) when no token is available.

Exit code 0 = in sync, 1 = drift detected.

Usage: python tools/check_grammar_sync.py
"""

from __future__ import annotations

import hashlib
import os
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GRAMMAR = ROOT / "grammar" / "RTL.g4"
UPSTREAM = ROOT / "grammar" / "UPSTREAM"
REPO = "regtab/jregtab"


def parse_pin(text: str) -> dict[str, str]:
    pin: dict[str, str] = {}
    for line in text.splitlines():
        line = line.strip()
        if not line or ":" not in line:
            continue
        key, _, value = line.partition(":")
        pin[key.strip()] = value.strip()
    return pin


def main() -> int:
    pin = parse_pin(UPSTREAM.read_text(encoding="utf-8"))
    commit = pin.get("commit")
    upstream_path = pin.get("path", "src/main/antlr4/ru/icc/regtab/rtl/RTL.g4")
    expected_hash = pin.get("sha256")
    if not commit or not expected_hash:
        print("FAIL: grammar/UPSTREAM must record both 'commit:' and 'sha256:'")
        return 1

    local_bytes = GRAMMAR.read_bytes()
    local_hash = hashlib.sha256(local_bytes).hexdigest()

    # 1) offline hash check
    if local_hash != expected_hash:
        print(
            "FAIL: grammar/RTL.g4 has drifted from the pinned hash.\n"
            f"  recorded (grammar/UPSTREAM): {expected_hash}\n"
            f"  actual   (grammar/RTL.g4):   {local_hash}\n"
            "  If you intentionally re-synced the grammar, update the sha256 in "
            "grammar/UPSTREAM (and the conformance corpus) in the same commit."
        )
        return 1
    print(f"OK: grammar/RTL.g4 matches pinned sha256 ({expected_hash[:12]}…)")

    # 2) opt-in upstream cross-check
    token = os.environ.get("JREGTAB_TOKEN")
    if not token:
        print(
            "NOTE: JREGTAB_TOKEN not set — skipping the upstream byte-for-byte "
            f"cross-check against {REPO}@{commit[:12]}. The offline hash check above "
            "still guarantees the copy is unchanged since it was pinned."
        )
        return 0

    url = f"https://raw.githubusercontent.com/{REPO}/{commit}/{upstream_path}"
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {token}"})
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            upstream_bytes = resp.read()
    except Exception as e:  # noqa: BLE001
        print(f"FAIL: could not fetch upstream grammar from {url}: {e}")
        return 1

    if upstream_bytes != local_bytes:
        print(
            f"FAIL: grammar/RTL.g4 differs from {REPO}@{commit[:12]}:{upstream_path}. "
            "Re-copy the upstream grammar and update grammar/UPSTREAM."
        )
        return 1
    print(f"OK: grammar/RTL.g4 is byte-identical to {REPO}@{commit[:12]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
