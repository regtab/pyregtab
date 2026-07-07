"""Differential test against jRegTab (plan §7.3): compares the recordsets
produced by pyRegTab with a JSONL dump produced by the reference Java
implementation (see RecordsetDumpMain.java) on the full fixture corpus.

Usage: python tools/differential.py <java_dump.jsonl>
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "tests"))

from task_runner import (  # noqa: E402
    TASKS_ROOT,
    compile_task,
    load_table,
)

from pyregtab import AtpMatcher, SchemaConstructionStrategy, TableInterpreter  # noqa: E402


def py_result(task_id: str, variant: int):
    pattern = compile_task(task_id)
    syntax = load_table(TASKS_ROOT / f"task_{task_id}" / f"input_{variant}.csv")
    itm = AtpMatcher.match(pattern, syntax)
    if itm is None:
        return {"match": False}
    rs = pattern.transform(
        TableInterpreter()
        .with_strategy(SchemaConstructionStrategy.RECORD_FIRST)
        .interpret(itm)
    )
    attrs = rs.schema.attributes
    return {
        "match": True,
        "schema": attrs,
        "records": [[rs[i][a] for a in attrs] for i in range(len(rs))],
    }


def main():
    dump = Path(sys.argv[1])
    mismatches = []
    total = 0
    for line in dump.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        j = json.loads(line)
        total += 1
        task, variant = j["task"], j["variant"]
        p = py_result(task, variant)
        expected = {k: j[k] for k in ("match", "schema", "records") if k in j}
        if p != expected:
            mismatches.append((task, variant))
            print(f"MISMATCH task_{task} variant_{variant}")
            if p.get("schema") != expected.get("schema"):
                print(f"  java schema: {expected.get('schema')}")
                print(f"  py   schema: {p.get('schema')}")
            else:
                er, pr = expected.get("records", []), p.get("records", [])
                for i, (a, b) in enumerate(zip(er, pr)):
                    if a != b:
                        print(f"  record {i}: java={a!r} py={b!r}")
                        break
    print(f"compared {total} variants: {len(mismatches)} mismatches")
    sys.exit(1 if mismatches else 0)


if __name__ == "__main__":
    main()
