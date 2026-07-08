# IDE support for RTL

This directory ships editor tooling for RTL (Regular Table Language):

- a TextMate grammar for `.rtl` files (`vscode/syntaxes/rtl.tmLanguage.json`),
- an injection grammar that highlights RTL inside Python string literals
  (`vscode/syntaxes/rtl-python-injection.tmLanguage.json`),
- a VS Code extension manifest wiring both together (`vscode/`).

The same `vscode/` directory doubles as a TextMate bundle for IntelliJ-based IDEs
(PyCharm and friends).

## VS Code

Copy (or symlink) the extension into your local extensions directory and reload:

```
# Windows
xcopy /E /I ide\vscode %USERPROFILE%\.vscode\extensions\regtab.rtl-language-0.1.0

# Linux / macOS
cp -r ide/vscode ~/.vscode/extensions/regtab.rtl-language-0.1.0
```

Alternatively, package it properly with [vsce](https://github.com/microsoft/vscode-vsce):
`cd ide/vscode && npx @vscode/vsce package`, then install the produced `.vsix` via
*Extensions → … → Install from VSIX*.

You get:

- syntax highlighting for `*.rtl` files;
- highlighting of RTL inside Python strings for these forms:
  - `RtlCompiler.compile("""…""")` / `RtlCompiler.compile('''…''')`,
  - `RtlCompiler.compile("…")` / `RtlCompiler.compile('…')` (optionally raw:
    `r"…"`).

Limitation (inherent to TextMate): the opening quote must be on the same line as
`RtlCompiler.compile(`.

## PyCharm / IntelliJ IDEA

1. *Settings → Editor → TextMate Bundles → “+”* and select the `ide/vscode` directory.
2. `*.rtl` files are now highlighted.

For RTL inside Python string literals, PyCharm's built-in language injection also
recognizes the line-comment marker `# language=RTL` placed on the statement — when the
IDE knows the RTL language (registered TextMate bundle or a dedicated plugin), it injects
RTL into the following literal. Support for injecting TextMate-backed languages varies by
IDE version; where unavailable, you still get `.rtl` file highlighting.

## Runtime validation

Python has no compile-time annotation processor (the jRegTab counterpart is
`@RtlSource` + a `javac` processor). In pyRegTab, an invalid RTL string is reported at
call time: `RtlCompiler.compile(...)` raises `RtlCompileError` with the source position
(`line:col`), the expected tokens, and a fragment of the offending input.

## Keeping the grammar in sync

The TextMate grammar mirrors the tokens of the normative grammar `grammar/RTL.g4`
(pinned from jRegTab). Any change to `RTL.g4` must be accompanied by a matching update to
`vscode/syntaxes/rtl.tmLanguage.json` in the same PR. The `.rtl` grammar
(`rtl.tmLanguage.json`) and the editor configuration (`language-configuration.json`) are
copied verbatim from jRegTab; only the injection grammar and manifest differ (Python
instead of Java host language).
