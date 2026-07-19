# IDE support for RTL

Editor tooling for RTL (Regular Table Language) — the VS Code extension, the
TextMate grammars for `.rtl` files, and the injection grammars that highlight
RTL inside Python and Java string literals — lives in its own repository:

**https://github.com/regtab/vscode-rtl**

The extension is universal: it serves every RegTab implementation (jRegTab,
pyRegTab, future ports) and requires neither Python nor a JDK.

- **VS Code**: until the Marketplace listing is live, download the VSIX for
  your platform from [Releases](https://github.com/regtab/vscode-rtl/releases)
  and install via `code --install-extension <file>.vsix` or
  *Extensions → ⋯ → Install from VSIX…* (the `universal` build ships
  highlighting and snippets only, without the bundled language server).
  Besides highlighting, the extension bundles a native language server
  (`rtl-lsp`) with compile diagnostics — in `.rtl` files and inside RTL
  string literals in Python — live match preview, and more.
- **PyCharm / IntelliJ IDEA**: clone vscode-rtl and register its repository
  root as a TextMate bundle (*Settings → Editor → TextMate Bundles*) — see
  the vscode-rtl README. This gives `.rtl` highlighting; the language-server
  features are VS Code-only.

## Keeping the grammar in sync

vscode-rtl is the canonical home of the RTL TextMate grammars. Any change to
the normative grammar `grammar/RTL.g4` (pinned from jRegTab) must be
accompanied by an issue/PR in vscode-rtl updating `rtl.tmLanguage.json`;
its CI runs a keyword-coverage sync-check against the pinned `RTL.g4`.

## Runtime validation

Python has no compile-time annotation processor (the jRegTab counterpart is
`@RtlSource` + a `javac` processor). In pyRegTab, an invalid RTL string is
reported at call time: `RtlCompiler.compile(...)` raises `RtlCompileError` with
the source position (`line:col`), the expected tokens, and a fragment of the
offending input.
