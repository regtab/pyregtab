# IDE support

RTL patterns usually live inside Python string literals, which editors treat as plain
text by default. pyRegTab ships editor tooling that improves this:

1. **Syntax highlighting** — a TextMate grammar for `.rtl` files and for RTL embedded in
   Python strings (VS Code, PyCharm, and any TextMate-compatible editor).
2. **Runtime validation** — an invalid RTL string is reported at `RtlCompiler.compile(...)`
   call time, with the exact source position.

## Syntax highlighting

The grammar lives in the repository under [`ide/`](https://github.com/regtab/pyregtab/tree/main/ide):
`ide/vscode/` is simultaneously a VS Code extension and an IntelliJ/PyCharm TextMate bundle.

### VS Code

Copy the extension into your extensions directory and reload:

```bash
# Linux / macOS
cp -r ide/vscode ~/.vscode/extensions/regtab.rtl-language-0.1.0
```

```bat
:: Windows
xcopy /E /I ide\vscode %USERPROFILE%\.vscode\extensions\regtab.rtl-language-0.1.0
```

(Or package a proper `.vsix` with `npx @vscode/vsce package` inside `ide/vscode`.)

This highlights `*.rtl` files and RTL inside Python strings in these forms:

```python
pattern = RtlCompiler.compile("""
    [ [ATTR] [VAL : (LT{1})->REC]+ ]+
""")

single = RtlCompiler.compile("[ [ATTR] [VAL]+ ]")
```

!!! note "Limitation"
    TextMate matching is line-based: the opening quote must be on the same line as
    `RtlCompiler.compile(`.

### PyCharm / IntelliJ IDEA

*Settings → Editor → TextMate Bundles → “+”* and select the `ide/vscode` directory —
`*.rtl` files are highlighted. For RTL inside Python string literals, PyCharm's built-in
language injection recognizes the `# language=RTL` line-comment marker, where the IDE
version supports injecting TextMate-backed languages.

## Runtime validation

Python has no `javac`-style compile-time annotation processor (the jRegTab counterpart is
the `@RtlSource` annotation validated during compilation). In pyRegTab, RTL is validated
when the string is compiled: `RtlCompiler.compile(...)` raises `RtlCompileError` with the
source position, expected tokens, and a fragment of the offending input.

```python
from pyregtab import RtlCompiler, RtlCompileError

try:
    RtlCompiler.compile("[ [VAL ]")   # missing ']'
except RtlCompileError as e:
    print(e)   # RTL compile error at 1:8: expected ']', found Eof
```

Catch these early by compiling patterns once at import time (module-level constants) —
an invalid pattern then fails on first import rather than deep inside a run.
