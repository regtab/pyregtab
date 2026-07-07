"""One-off translator: jRegTab AtpTaskNNNTest.java -> tests/atp_patterns.py.

The Java ATP task tests are flat classes (static final constants + a
buildPattern() method) written against a closed factory vocabulary, so the
builder expressions can be translated mechanically. Result correctness is
verified by running every generated pattern against the task fixtures.

Usage: python tools/translate_atp.py <jregtab_repo> <output.py>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Java camelCase -> Python snake_case for methods
def snake(name: str) -> str:
    s = re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", name).lower()
    return {"and": "and_", "or": "or_"}.get(s, s)


# `new X.Y(...)` / `X.Y.INSTANCE` -> Python factory on class X
CTOR_FACTORY_ROOTS = {"CellPredicate", "FilterTerm", "StringExtractor"}

INT_MAX = "UNBOUNDED"  # Integer.MAX_VALUE -> pyregtab.UNBOUNDED


class Translator:
    def __init__(self, src: str, symbols: dict[str, str]):
        self.s = src
        self.i = 0
        self.symbols = symbols  # name -> Java type

    # ---------------- low-level ----------------

    def ws(self):
        while self.i < len(self.s):
            if self.s[self.i].isspace():
                self.i += 1
            elif self.s.startswith("//", self.i):
                self.i = self.s.find("\n", self.i)
                if self.i < 0:
                    self.i = len(self.s)
            elif self.s.startswith("/*", self.i):
                self.i = self.s.find("*/", self.i) + 2
            else:
                break

    def peek(self) -> str:
        self.ws()
        return self.s[self.i] if self.i < len(self.s) else ""

    def eat(self, ch: str) -> bool:
        if self.peek() == ch:
            self.i += 1
            return True
        return False

    def expect(self, ch: str):
        if not self.eat(ch):
            raise SyntaxError(f"expected {ch!r} at ...{self.s[self.i:self.i+40]!r}")

    def ident(self) -> str:
        self.ws()
        m = re.match(r"[A-Za-z_$][A-Za-z0-9_$]*", self.s[self.i:])
        if not m:
            raise SyntaxError(f"expected identifier at ...{self.s[self.i:self.i+40]!r}")
        self.i += m.end()
        return m.group(0)

    def string_literal(self) -> str:
        # returns the literal verbatim (Java escapes == Python escapes here)
        start = self.i
        assert self.s[self.i] == '"'
        self.i += 1
        while self.s[self.i] != '"':
            if self.s[self.i] == "\\":
                self.i += 1
            self.i += 1
        self.i += 1
        return self.s[start:self.i]

    # ---------------- expressions ----------------

    def parse_args(self) -> list["Expr"]:
        self.expect("(")
        args = []
        if self.peek() == ")":
            self.i += 1
            return args
        while True:
            args.append(self.parse_expr())
            if self.eat(","):
                continue
            self.expect(")")
            return args

    def parse_expr(self) -> "Expr":
        e = self.parse_primary()
        # postfix chains
        while self.peek() == ".":
            self.i += 1
            name = self.ident()
            if self.peek() == "(":
                args = self.parse_args()
                e = Expr(kind="call", text=f"{e.py}.{snake(name)}({join(args)})",
                         root=e.root)
            else:
                # field access mid-chain (rare)
                e = Expr(kind="ref", text=f"{e.py}.{name}", root=e.root)
        return e

    def parse_primary(self) -> "Expr":
        c = self.peek()
        if c == '"':
            return Expr(kind="str", text=self.string_literal(), root="str")
        if c == "-" or c.isdigit():
            m = re.match(r"-?\d+", self.s[self.i:])
            self.i += m.end()
            return Expr(kind="int", text=m.group(0), root="int")
        if c == "(":
            self.i += 1
            e = self.parse_expr()
            self.expect(")")
            return Expr(kind=e.kind, text=f"({e.py})", root=e.root)
        name = self.ident()
        if name == "new":
            return self.parse_new()
        if name == "null":
            return Expr(kind="null", text="None", root="null")
        if name in ("true", "false"):
            return Expr(kind="bool", text=name.capitalize(), root="bool")
        # bare call via static import (e.g. `and(...)` from
        # `import static ...ItemFilterConditionSpec.and;`)
        if self.peek() == "(" and name in ("and", "or", "bare"):
            args = self.parse_args()
            return Expr(
                "call",
                f"ItemFilterConditionSpec.{snake(name)}({join(args)})",
                "ItemFilterConditionSpec",
            )
        # qualified name: A.B.C(...)? — collect the dotted path greedily,
        # stopping at the first "(" which decides the call target
        path = [name]
        save = self.i
        while self.peek() == ".":
            self.i += 1
            nxt = self.ident()
            path.append(nxt)
            if self.peek() == "(":
                return self.qualified_call(path)
            save = self.i
        self.i = save
        return self.qualified_ref(path)

    def parse_new(self) -> "Expr":
        path = [self.ident()]
        while self.peek() == ".":
            self.i += 1
            path.append(self.ident())
        args = self.parse_args()
        return self.constructor(path, args)

    # ---------------- semantic mapping ----------------

    def constructor(self, path: list[str], args: list["Expr"]) -> "Expr":
        root, leaf = path[0], path[-1]
        if root in CTOR_FACTORY_ROOTS and len(path) == 2:
            if root == "StringExtractor" and leaf == "Chain":
                # Chain(List.of(a, b)) -> chain(a, b)
                inner = args[0].list_items if args[0].kind == "list" else [args[0]]
                return Expr("call", f"StringExtractor.chain({join(inner)})", root)
            return Expr("call", f"{root}.{snake(leaf)}({join(args)})", root)
        if leaf == "CellMatchCondition":
            return Expr("call", f"CellMatchCondition({join(args)})", leaf)
        if leaf == "CompoundSegment":
            return Expr("tuple", f"({join(args)})", leaf)
        if leaf == "CompoundContentSpec":
            return Expr("call", f"CompoundContentSpec({join(args)})", leaf)
        if leaf == "DelimitedContentSpec":
            return Expr("call", f"DelimitedContentSpec({join(args)})", leaf)
        if leaf == "ConditionalContentSpec":
            return Expr("call", f"ConditionalContentSpec({join(args)})", leaf)
        if leaf == "AtomicContentSpec":
            return Expr("call", f"AtomicContentSpec({join(args)})", leaf)
        if leaf in ("FieldSplitting", "DelimitedFieldSplit", "SchemaReordering",
                    "AnchorAttributeAtPosition", "WhitespaceNormalization"):
            return Expr("call", f"{leaf}({join(args)})", leaf)
        if leaf in ("CellPattern", "SubrowPattern", "RowPattern", "SubtablePattern",
                    "TablePattern"):
            return Expr("call", f"{leaf}({join(args)})", leaf)
        if root == "ItemFilterConditionSpec" and len(path) == 2:
            # new ItemFilterConditionSpec.Bare(t) / And(List.of(...)) / Or(...)
            if leaf in ("And", "Or") and args and args[0].kind == "list":
                items = args[0].list_items
                return Expr(
                    "call",
                    f"ItemFilterConditionSpec.{snake(leaf)}({join(items)})",
                    root,
                )
            return Expr("call", f"ItemFilterConditionSpec.{snake(leaf)}({join(args)})", root)
        raise SyntaxError(f"unknown constructor: {'.'.join(path)}")

    def qualified_ref(self, path: list[str]) -> "Expr":
        dotted = ".".join(path)
        if dotted == "ProviderSpec.UNBOUNDED":
            return Expr("int", "UNBOUNDED", "int")
        if dotted == "Integer.MAX_VALUE":
            return Expr("int", INT_MAX, "int")
        if len(path) == 3 and path[2] == "INSTANCE" and path[0] in CTOR_FACTORY_ROOTS:
            return Expr("call", f"{path[0]}.{snake(path[1])}()", path[0])
        if len(path) == 2 and path[0] in (
            "TraversalOrder", "SchemaConstructionStrategy", "ActionApplicationStrategy",
            "ItemType", "OperationType", "ItemDerivationDirective",
            "CellDerivedProviderKind", "FontFamily", "HorizontalAlignment",
            "VerticalAlignment",
        ):
            return Expr("ref", dotted, path[0])
        if len(path) == 1:
            ty = self.symbols.get(path[0], "?")
            return Expr("ref", path[0], ty)
        raise SyntaxError(f"unknown reference: {dotted}")

    def qualified_call(self, path: list[str]) -> "Expr":
        args = self.parse_args()
        dotted = ".".join(path)
        root, leaf = path[0], path[-1]

        if dotted in ("List.of", "Arrays.asList", "java.util.List.of"):
            e = Expr("list", f"[{join(args)}]", "list")
            e.list_items = args
            return e
        if dotted == "Set.of":
            if not args:
                return Expr("set", "set()", "set")
            return Expr("set", f"{{{join(args)}}}", "set")
        if leaf == "of" and path[-2] == "Segment":
            return Expr("tuple", f"({join(args)})", "Segment")

        if root == "ProviderSpec" and leaf in ("val", "any", "aux", "attr"):
            return self.provider_call(leaf, args)
        if root == "ActionSpec":
            return self.action_call(leaf, args)
        if root == "AtomicContentSpec" and leaf in ("val", "attr", "aux"):
            return self.atomic_call(leaf, args)
        if root == "AtomicContentSpec" and leaf == "valTagged":
            return Expr("call", f"AtomicContentSpec.val_tagged({join(args)})",
                        "AtomicContentSpec")

        # generic static factory: X.method(args) with snake_case
        return Expr("call", f"{root}.{snake(leaf)}({join(args)})", root)

    def provider_call(self, leaf: str, args: list["Expr"]) -> "Expr":
        # Java: val(cond) | val(card, cond) | val(card, order, cond)
        #       attr(cond) | attr(order, cond)
        # Python: val(condition, cardinality=1, traversal_order=None)
        #         attr(condition, traversal_order=None)
        cond = args[-1]
        rest = args[:-1]
        out = [cond.py]
        if leaf == "attr":
            if rest:  # (order, cond)
                out.append(f"traversal_order={rest[0].py}")
        else:
            if len(rest) >= 1:  # cardinality first
                out.append(rest[0].py)
            if len(rest) == 2:  # then traversal order
                out.append(f"traversal_order={rest[1].py}")
        return Expr("call", f"ProviderSpec.{leaf}({', '.join(out)})", "ProviderSpec")

    def action_call(self, leaf: str, args: list["Expr"]) -> "Expr":
        def is_cond(e: Expr) -> bool:
            return e.root in ("ItemFilterConditionSpec", "FilterTerm") or (
                e.kind == "ref" and self.symbols.get(e.py) == "ItemFilterConditionSpec"
            )

        def is_int(e: Expr) -> bool:
            return e.kind == "int" or (
                e.kind == "ref" and self.symbols.get(e.py) in ("int", "Integer")
            )

        def is_str(e: Expr) -> bool:
            return e.kind == "str" or (
                e.kind == "ref" and self.symbols.get(e.py) == "String"
            )

        if leaf == "rec":
            kw = []
            rest = args
            if args and is_int(args[0]):
                head, rest = args[0], args[1:]
                if rest and all(is_cond(a) for a in rest):
                    kw.append(f"cardinality={head.py}")
                else:
                    kw.append(f"anchor_pos={head.py}")
            elif args and is_str(args[0]):
                head, rest = args[0], args[1:]
                kw.append(f"split_delimiter={head.py}")
            parts = [a.py for a in rest] + kw
            return Expr("call", f"ActionSpec.rec({', '.join(parts)})", "ActionSpec")
        if leaf == "join":
            kw = []
            rest = args
            if args and args[0].kind in ("int", "set"):
                kw.append(f"key_positions={args[0].py}")
                rest = args[1:]
            parts = [a.py for a in rest] + kw
            return Expr("call", f"ActionSpec.join({', '.join(parts)})", "ActionSpec")
        if leaf in ("fill", "prefix", "suffix", "avp"):
            return Expr("call", f"ActionSpec.{leaf}({join(args)})", "ActionSpec")
        raise SyntaxError(f"unknown ActionSpec factory: {leaf}")

    def atomic_call(self, leaf: str, args: list["Expr"]) -> "Expr":
        # Java: val(actions...) | val(extractor, actions...)
        def is_extractor(e: Expr) -> bool:
            return e.root == "StringExtractor" or (
                e.kind == "ref" and self.symbols.get(e.py) == "StringExtractor"
            )

        kw = []
        rest = args
        if args and is_extractor(args[0]):
            kw.append(f"extractor={args[0].py}")
            rest = args[1:]
        parts = [a.py for a in rest] + kw
        return Expr("call", f"AtomicContentSpec.{leaf}({', '.join(parts)})",
                    "AtomicContentSpec")


class Expr:
    def __init__(self, kind: str, text: str, root: str):
        self.kind = kind
        self.py = text
        self.root = root
        self.list_items: list[Expr] = []


def join(args: list[Expr]) -> str:
    return ", ".join(a.py for a in args)


# ---------------- per-file processing ----------------

FIELD_RE = re.compile(
    r"private\s+static\s+final\s+([A-Za-z_][\w.<>]*)\s+([A-Z_][A-Z0-9_]*)\s*=", re.M
)
LOCAL_RE = re.compile(r"^\s*(?:var|[A-Za-z_][\w.<>]*)\s+([a-zA-Z_]\w*)\s*=\s*$")


def balanced_expr(src: str, start: int) -> tuple[str, int]:
    """Reads from `start` to the `;` at nesting depth 0. Returns (expr, end)."""
    depth = 0
    i = start
    while i < len(src):
        c = src[i]
        if c == '"':
            i += 1
            while src[i] != '"':
                if src[i] == "\\":
                    i += 1
                i += 1
        elif c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        elif c == ";" and depth == 0:
            return src[start:i], i + 1
        i += 1
    raise SyntaxError("unterminated expression")


def translate_file(path: Path) -> tuple[str, str]:
    src = path.read_text(encoding="utf-8")
    m = re.search(r'return\s+"(\d+)"\s*;', src)
    task_id = m.group(1)

    symbols: dict[str, str] = {}
    lines: list[str] = []

    # static final constants (in declaration order)
    for fm in FIELD_RE.finditer(src):
        ftype, fname = fm.group(1), fm.group(2)
        expr_src, _ = balanced_expr(src, fm.end())
        symbols[fname] = ftype.split("<")[0].split(".")[0]
        t = Translator(expr_src, symbols)
        lines.append(f"    {fname} = {t.parse_expr().py}")

    # buildPattern body
    bm = re.search(r"protected\s+TablePattern\s+buildPattern\(\)\s*\{", src)
    body_start = bm.end()
    i = body_start

    def skip_trivia(j: int) -> int:
        while j < len(src):
            if src[j].isspace():
                j += 1
            elif src.startswith("//", j):
                nl = src.find("\n", j)
                j = nl if nl >= 0 else len(src)
            elif src.startswith("/*", j):
                j = src.find("*/", j) + 2
            else:
                break
        return j

    while True:
        # find next statement start
        i = skip_trivia(i)
        rest = src[i:]
        sm = re.match(
            r"\s*(?:(return)\s+|(var|[A-Za-z_][\w.<>]*)\s+([a-zA-Z_]\w*)\s*=\s*)",
            rest,
        )
        if not sm:
            break
        expr_src, end = balanced_expr(src, i + sm.end())
        t = Translator(expr_src, symbols)
        e = t.parse_expr()
        if sm.group(1):  # return
            lines.append(f"    return {e.py}")
            break
        else:
            jtype, name = sm.group(2), sm.group(3)
            symbols[name] = e.root if jtype == "var" else jtype.split("<")[0].split(".")[0]
            lines.append(f"    {name} = {e.py}")
        i = end

    body = "\n".join(lines)
    return task_id, f"def pattern_{task_id}():\n{body}\n"


HEADER = '''"""ATP patterns for all benchmark tasks, mechanically translated from
jRegTab's AtpTaskNNNTest.java fluent builders (see tools/translate_atp.py).
Do not edit by hand except to fix translation issues."""

# flake8: noqa
from pyregtab import (
    ActionSpec,
    AtomicContentSpec,
    CellMatchCondition,
    CellPattern,
    CellPredicate,
    CompoundContentSpec,
    ConditionalContentSpec,
    DelimitedContentSpec,
    FilterTerm,
    ItemFilterConditionSpec,
    ProviderSpec,
    Quantifier,
    RowPattern,
    StringExtractor,
    SubrowPattern,
    SubtablePattern,
    TablePattern,
    TraversalOrder,
    UNBOUNDED,
)

'''


def main():
    jregtab = Path(sys.argv[1])
    out = Path(sys.argv[2])
    atp_dir = jregtab / "src/test/java/ru/icc/regtab/atp"
    chunks = [HEADER]
    ids = []
    failures = []
    for f in sorted(atp_dir.glob("AtpTask*Test.java")):
        if f.name == "AtpTaskBase.java":
            continue
        try:
            task_id, code = translate_file(f)
            ids.append(task_id)
            chunks.append(code + "\n")
        except Exception as e:  # noqa: BLE001
            failures.append(f"{f.name}: {e}")
    chunks.append(
        "PATTERNS = {\n"
        + "".join(f'    "{t}": pattern_{t},\n' for t in sorted(ids))
        + "}\n"
    )
    out.write_text("".join(chunks), encoding="utf-8")
    print(f"translated {len(ids)} tasks -> {out}")
    if failures:
        print("FAILURES:")
        for f in failures:
            print(" ", f)


if __name__ == "__main__":
    main()
