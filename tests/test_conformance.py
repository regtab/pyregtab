"""RTL conformance corpus (port of RtlConformanceTest, contract in
conformance/README.md):

1. every positive case compiles;
2. serialize(compile(x)) equals the canonical form byte-for-byte;
3. the canonical form is a fixed point;
4. every negative case is rejected with RtlCompileError.
"""

from pathlib import Path

import pytest

from pyregtab import AtpToRtlSerializer, RtlCompileError, RtlCompiler

CONFORMANCE = Path(__file__).parent.parent / "conformance"

POSITIVE = sorted(
    p for p in (CONFORMANCE / "positive").glob("*.rtl") if not p.name.endswith(".expected.rtl")
)
NEGATIVE = sorted((CONFORMANCE / "negative").glob("*.rtl"))


def read(p: Path) -> str:
    # binary read: string literals may carry raw CR/CRLF payload
    return p.read_bytes().decode("utf-8")


@pytest.mark.parametrize("case", POSITIVE, ids=lambda p: p.stem)
def test_positive_canonical(case):
    expected = read(case.with_name(case.stem + ".expected.rtl")).rstrip("\n")
    pattern = RtlCompiler.compile(read(case))
    canonical = AtpToRtlSerializer.serialize(pattern)
    assert canonical == expected


@pytest.mark.parametrize("case", POSITIVE, ids=lambda p: p.stem)
def test_positive_fixed_point(case):
    expected = read(case.with_name(case.stem + ".expected.rtl")).rstrip("\n")
    again = AtpToRtlSerializer.serialize(RtlCompiler.compile(expected))
    assert again == expected


@pytest.mark.parametrize("case", NEGATIVE, ids=lambda p: p.stem)
def test_negative_rejected(case):
    with pytest.raises(RtlCompileError):
        RtlCompiler.compile(read(case))
