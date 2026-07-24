# IDE support

RTL patterns usually live inside Python string literals, which editors treat as plain
text by default. Two layers of tooling improve this:

1. **Editor support** — the [RegTab RTL extension](https://github.com/regtab/vscode-rtl)
   for VS Code: syntax highlighting for `.rtl` files and for RTL embedded in Python
   strings, plus a native language server with compile diagnostics, live match preview,
   and more. It is developed in its own repository so that every RegTab implementation
   (jRegTab, pyRegTab, future ports) shares the same extension.
2. **Runtime validation** — an invalid RTL string is reported at `RtlCompiler.compile(...)`
   call time, with the exact source position.

## Editor support

### VS Code

Install the [RegTab RTL extension](https://marketplace.visualstudio.com/items?itemName=regtab.regtab)
from the Visual Studio Marketplace: open the *Extensions* panel (`Ctrl+Shift+X`), search for
**RegTab RTL**, and click *Install*. From the command line:

```bash
code --install-extension regtab.regtab
```

Marketplace installs update automatically.

If you need a build that is not yet on the Marketplace, install it from a VSIX instead:
download the package for your platform from
[Releases](https://github.com/regtab/vscode-rtl/releases) (the `universal` build ships
highlighting and snippets only, without the bundled language server), then:

```bash
code --install-extension regtab-rtl-<version>-<platform>.vsix
```

or via the UI: *Extensions panel → ⋯ menu → Install from VSIX…* → pick the file, then reload
the window. A VSIX installed this way does not auto-update — install a newer VSIX over it to
upgrade (settings and state are kept).

Beyond highlighting, the extension bundles a native language server (`rtl-lsp`, built on
the same Rust core as pyRegTab): compile diagnostics as you type — in `.rtl` files and
inside RTL string literals in Python — live match preview against CSV fixtures,
expected-result diffing, fragment navigation, completion, and code snippets.

The extension highlights `*.rtl` files and RTL inside Python strings in these forms:

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

The language-server features (diagnostics, match preview, completion, navigation) are
VS Code-only — a TextMate bundle carries highlighting alone.

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
