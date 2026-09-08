"""The CLI runner (``pyregtab.runner``) and the bulk table constructors it
relies on: ``parse_csv`` (native) against the former pure-Python parser,
``TableSyntax.from_rows`` / ``from_csv`` / ``from_csv_text``, and the
``input.csv → output.csv`` contract."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

from pyregtab import TableSyntax
from pyregtab.runner import load_table, main, parse_csv, write_csv


def reference_parse_csv(text: str) -> list[list[str]]:
    """The pure-Python parser the runner shipped with in 0.7.1 (a transcript
    of ``RtlRunner.parseCsv``); the native parser must agree with it."""
    rows: list[list[str]] = []
    fields: list[str] = []
    cur: list[str] = []
    in_quotes = False
    quoted = False
    i, n = 0, len(text)
    while i < n:
        ch = text[i]
        if in_quotes:
            if ch != '"':
                cur.append(ch)
            elif i + 1 < n and text[i + 1] == '"':
                cur.append('"')
                i += 1
            else:
                in_quotes = False
        elif ch == '"':
            in_quotes = True
            quoted = True
        elif ch == ",":
            fields.append("".join(cur))
            cur = []
        elif ch in "\r\n":
            if ch == "\r" and i + 1 < n and text[i + 1] == "\n":
                i += 1
            if fields or cur or quoted:
                fields.append("".join(cur))
                rows.append(fields)
                fields = []
            cur = []
            quoted = False
        else:
            cur.append(ch)
        i += 1
    if fields or cur or quoted:
        fields.append("".join(cur))
        rows.append(fields)
    return rows


CSV_SAMPLES = [
    "",
    "\n",
    "\r\n\r\n",
    "a,b\n1,2\n",
    "a,b\r\n1,2\r\n",
    "a,b\r1,2\r",
    "a,b\n1,2",
    "a,,c\n,,\n",
    '"a,b","c""d"\n',
    '"x\ny",z\n',
    '"x\r\ny",z\n',
    '""\n',
    '"",""\n',
    'a\n\n\nb\n\n',
    'a"b"c,d\n',
    '"unterminated,x',
    'a,"b\n',
    'a,b\n1\n1,2,3\n',
    'тест,"日本,語"\n',
    '"a""",""""\n',
    ' a , b \n',
    '"a"b,c\n',
]


@pytest.mark.parametrize("text", CSV_SAMPLES)
def test_parse_csv_matches_reference(text: str) -> None:
    assert parse_csv(text) == reference_parse_csv(text)


def test_from_rows_pads_ragged_rows() -> None:
    t = TableSyntax.from_rows([["a", "b", "c"], ["d"], []])
    assert (t.num_rows, t.num_cols) == (3, 3)
    assert [[t.cell(r, c).text for c in range(3)] for r in range(3)] == [
        ["a", "b", "c"], ["d", "", ""], ["", "", ""],
    ]
    assert t.cell(1, 1).text_blank and not t.cell(0, 0).text_blank
    assert t.row(1).subrows()[0].cells()[2].text == ""
    assert len(t.subtables()) == 1


def test_from_rows_num_cols_and_text_properties() -> None:
    t = TableSyntax.from_rows([["  x", "y\nz"]], num_cols=3)
    assert (t.num_rows, t.num_cols) == (1, 3)
    assert t.cell(0, 0).text_indent == 2
    assert t.cell(0, 1).text_multiline
    assert t.cell(0, 2).text == ""
    # the same error class as TableSyntax(0, 0)
    with pytest.raises(RuntimeError):
        TableSyntax.from_rows([["a", "b"]], num_cols=1)
    with pytest.raises(RuntimeError):
        TableSyntax.from_rows([])
    with pytest.raises(RuntimeError):
        TableSyntax.from_rows([[], []])
    with pytest.raises(TypeError):
        TableSyntax.from_rows(["ab", "cd"])  # strings are not rows


def test_from_rows_equals_cell_by_cell_construction() -> None:
    rows = [["a", "b"], ["c", "d"], ["", "e"]]
    bulk = TableSyntax.from_rows(rows)
    manual = TableSyntax(3, 2)
    for r, row in enumerate(rows):
        for c, s in enumerate(row):
            manual.cell(r, c).set_text(s)
    for r in range(3):
        for c in range(2):
            a, b = bulk.cell(r, c), manual.cell(r, c)
            assert (a.text, a.text_blank, a.text_multiline, a.text_indent) == (
                b.text, b.text_blank, b.text_multiline, b.text_indent)


def test_from_csv_and_from_csv_text(tmp_path: Path) -> None:
    text = 'h1,h2,h3\r\n"a,1",b\r\n\r\n"x\r\ny",z,w\r\n'
    p = tmp_path / "t.csv"
    p.write_bytes(text.encode("utf-8"))
    t = TableSyntax.from_csv(p)
    assert (t.num_rows, t.num_cols) == (3, 3)
    assert t.cell(1, 0).text == "a,1" and t.cell(1, 2).text == ""
    # a file is read with universal newlines (as Path.read_text): CRLF inside
    # a quoted field becomes LF, exactly as the former load_table did
    assert t.cell(2, 0).text == "x\ny"
    # text in memory is parsed as is
    assert TableSyntax.from_csv_text(text).cell(2, 0).text == "x\r\ny"
    assert load_table(p).cell(2, 0).text == "x\ny"
    assert TableSyntax.from_csv(str(p)).num_rows == 3
    with pytest.raises(OSError):
        TableSyntax.from_csv(tmp_path / "missing.csv")
    (tmp_path / "bad.csv").write_bytes(b"a,\xff\n")
    with pytest.raises(UnicodeDecodeError):
        TableSyntax.from_csv(tmp_path / "bad.csv")
    (tmp_path / "empty.csv").write_bytes(b"\n\n")
    with pytest.raises(RuntimeError):
        TableSyntax.from_csv(tmp_path / "empty.csv")


def test_from_csv_keeps_bom_like_read_text(tmp_path: Path) -> None:
    p = tmp_path / "bom.csv"
    p.write_bytes("﻿a,b\n".encode("utf-8"))
    assert TableSyntax.from_csv(p).cell(0, 0).text == "﻿a"


RTL = "[ [ATTR] [ATTR] ]\n[ [VAL: COL->AVP, ROW->REC] [VAL: COL->AVP] ]+\n"


def test_runner_contract(tmp_path: Path) -> None:
    (tmp_path / "input.csv").write_text('a,b\n1,"x,y"\n2,z\n', encoding="utf-8")
    (tmp_path / "p.rtl").write_text(RTL, encoding="utf-8")
    inp, out = tmp_path / "input.csv", tmp_path / "output.csv"
    assert main([str(tmp_path / "p.rtl"), "--input", str(inp), "--output", str(out)]) == 0
    assert out.read_bytes() == b'"a","b"\n"1","x,y"\n"2","z"\n'
    # every field double-quoted, quotes doubled, LF
    (tmp_path / "input.csv").write_text('a,b\n1,"say ""hi"""\n', encoding="utf-8")
    assert main([str(tmp_path / "p.rtl"), "--input", str(inp), "--output", str(out)]) == 0
    assert out.read_bytes() == b'"a","b"\n"1","say ""hi"""\n'


def test_runner_exit_codes(tmp_path: Path) -> None:
    (tmp_path / "p.rtl").write_text("[ [VAL: 'x'->AVP] ]\n", encoding="utf-8")
    inp = tmp_path / "input.csv"
    inp.write_text("a,b\nc,d\n", encoding="utf-8")
    assert main([str(tmp_path / "p.rtl"), "--input", str(inp),
                 "--output", str(tmp_path / "o.csv")]) == 1  # no match
    assert main([]) == 2  # usage


def test_runner_as_module(tmp_path: Path) -> None:
    (tmp_path / "input.csv").write_text("a,b\n1,2\n", encoding="utf-8")
    rtl = tmp_path / "p.rtl"
    rtl.write_text(RTL, encoding="utf-8")
    proc = subprocess.run([sys.executable, "-m", "pyregtab.runner", str(rtl)], cwd=tmp_path,
                          capture_output=True, text=True)
    assert proc.returncode == 0, proc.stderr
    assert (tmp_path / "output.csv").read_bytes() == b'"a","b"\n"1","2"\n'


def test_write_csv_format(tmp_path: Path) -> None:
    from pyregtab import Recordset, Schema

    rs = Recordset(Schema(["a", "b"]), [{"a": "x", "b": None}, {"a": 'q"', "b": "y\nz"}])
    p = tmp_path / "o.csv"
    write_csv(p, rs)
    assert p.read_bytes() == b'"a","b"\n"x",""\n"q""","y\nz"\n'


def test_to_csv_file_equals_string(tmp_path: Path) -> None:
    """The streamed file output is byte for byte the string output."""
    from pyregtab import Recordset, Schema

    rs = Recordset(Schema(["a", "b,c"]), [
        {"a": 'say "hi"', "b,c": "x\ny"},
        {"a": None, "b,c": "plain"},
        {"a": "", "b,c": "\r\nend"},
    ])
    for kwargs in ({}, {"quote_all": True, "newline": "\n"}, {"sep": ";", "missing": "NULL"}):
        text = rs.to_csv(**kwargs)
        p = tmp_path / "o.csv"
        assert rs.to_csv(p, **kwargs) is None
        assert p.read_bytes() == text.encode("utf-8"), kwargs
    with pytest.raises(OSError):
        rs.to_csv(tmp_path / "no_such_dir" / "o.csv")
