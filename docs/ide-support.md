# IDE support

RTL patterns usually live inside Python string literals, which editors treat as plain
text by default. Editor tooling for RTL — syntax highlighting, compile diagnostics,
live match preview, and more — is developed in a companion repository so that every
RegTab implementation (jRegTab, pyRegTab, future ports) shares the same extension:

**[github.com/regtab/vscode-rtl](https://github.com/regtab/vscode-rtl)**

Highlights relevant here:

1. **Syntax highlighting** — a TextMate grammar for `.rtl` files and for RTL embedded in
   Python strings.
2. **Runtime validation** — an invalid RTL string is reported at `RtlCompiler.compile(...)`
   call time, with the exact source position.

## Syntax highlighting

### VS Code

The extension isn't on the Marketplace/Open VSX yet — install a VSIX from the
[Releases page](https://github.com/regtab/vscode-rtl/releases), e.g.
[v0.8.3](https://github.com/regtab/vscode-rtl/releases/tag/v0.8.3):

1. Download the package for your platform (`regtab-rtl-<version>-<platform>.vsix`).
2. Install it:

   ```bash
   code --install-extension regtab-rtl-<version>-<platform>.vsix
   ```

   or from the UI: *Extensions* panel → **⋯** menu (top right) →
   **Install from VSIX…** → pick the file. Reload the window afterwards.

A VSIX installed this way does not auto-update — install a newer VSIX over it to
upgrade.

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

Clone [vscode-rtl](https://github.com/regtab/vscode-rtl), then
*Settings → Editor → TextMate Bundles → “+”* and select its repository root
(it holds `package.json`, `language-configuration.json` and `syntaxes/`) —
`*.rtl` files are highlighted. For RTL inside Python string literals, PyCharm's built-in
language injection recognizes the `# language=RTL` line-comment marker, where the IDE
version supports injecting TextMate-backed languages.

The rest of the extension's feature set (compile diagnostics as you type, live match
preview against CSV fixtures, Test Explorer integration, hover reference, completion,
`$fragment` navigation) is VS Code-specific — see the
[vscode-rtl README](https://github.com/regtab/vscode-rtl#readme) for details.

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
