# IDE support for RTL

Editor tooling for RTL (Regular Table Language) — the VS Code extension, the
TextMate grammars for `.rtl` files, and the injection grammars that highlight
RTL inside Python and Java string literals — lives in its own repository:

**https://github.com/regtab/vscode-rtl**

The extension is universal: it serves every RegTab implementation (jRegTab,
pyRegTab, future ports) and requires neither Python nor a JDK.

- **VS Code**: install “RTL — Regular Table Language (RegTab)” from the
  VS Code Marketplace or Open VSX (extension id `regtab.rtl-language`).
- **PyCharm / IntelliJ IDEA**: register the extension directory as a TextMate
  bundle (*Settings → Editor → TextMate Bundles*) — see the vscode-rtl README.

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
