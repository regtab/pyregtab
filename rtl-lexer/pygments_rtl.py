"""Pygments lexer for RTL (Regular Table Language), the pattern DSL
of the RegTab project: https://regtab.github.io/
"""

from pygments.lexer import RegexLexer, bygroups
from pygments.token import (
    Comment,
    Keyword,
    Name,
    Number,
    Operator,
    Punctuation,
    String,
    Text,
    Whitespace,
)

__all__ = ["RtlLexer"]


class RtlLexer(RegexLexer):
    name = "RTL"
    aliases = ["rtl"]
    filenames = ["*.rtl"]
    mimetypes = ["text/x-rtl"]

    tokens = {
        "root": [
            (r"\s+", Whitespace),
            (r"//.*?$", Comment.Single),
            # #'…' tags before plain single-quoted strings
            (r"#'[^']*'", Name.Tag),
            (r"'[^']*'", String.Single),
            (r'"[^"]*"', String.Double),
            # $NAME fragment references
            (r"\$\w+", Name.Variable),
            # ->ACTION: the action name gets its own color
            (
                r"(->)(\s*)([A-Z][A-Z0-9_]*)",
                bygroups(Operator, Whitespace, Name.Function),
            ),
            # Providers, extractors and other ALL-CAPS keywords (VAL, COL, ...)
            (r"[A-Z][A-Z0-9_]*", Keyword),
            (r"\d+", Number.Integer),
            # Quantifiers and the remaining operators
            (r"[+*?=:,]", Operator),
            (r"[\[\]()<>{}]", Punctuation),
            (r"\w+", Name),
            # Catch-all so unknown constructs never render as errors
            (r".", Text),
        ],
    }
