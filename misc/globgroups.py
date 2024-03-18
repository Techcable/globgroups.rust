"""
Expands globs with grouping like foo{bar,baz}

This implementation is not maintained.
It has been superceeded by the rust implementation.
"""

import itertools
import os
import pprint
import sys
from dataclasses import dataclass
from typing import ForwardRef, Iterator, Optional, Sequence, TypeAlias

import funcparserlib.lexer as funclex
from funcparserlib.parser import NoParseError, finished, forward_decl, many, maybe, tok

__all__ = ("Glob",)

_TOKENIZER = funclex.make_tokenizer(
    [
        funclex.TokenSpec("op", r"[\{\},]"),
        funclex.TokenSpec("word", r"[^\{\},]+"),
    ]
)

GlobGroup = ForwardRef("GlobGroup")
GlobExpr: TypeAlias = str | GlobGroup


def _expand_expr(expr: GlobExpr) -> Iterator[str]:
    match expr:
        case str(word):
            yield word
        case GlobGroup(prefix, children, suffix):
            suffixes = list(_expand_expr(suffix))
            for child in children:
                for child_expansion in _expand_expr(child):
                    for suffix_expansion in suffixes:
                        yield prefix + child_expansion + suffix_expansion
        case other:
            raise TypeError(type(other))


class GlobParseError(ValueError):
    pass


@dataclass
class Glob:
    _expr: GlobExpr

    @staticmethod
    def parse(text: str) -> "Glob":
        if not text:
            return Glob("")
        try:
            return _whole_expr.parse(list(_TOKENIZER(text)))
        except (funclex.LexerError, NoParseError):
            raise ValueError(f"Failed to parse glob: {text!r}")

    def expand(self) -> list[str]:
        return list(_expand_expr(self._expr))

    def __str__(self):
        return str(self._expr)


@dataclass
class GlobGroup:
    _prefix: str
    _children: list[GlobExpr]
    _suffix: GlobExpr

    def expand(self) -> list[str]:
        return list(_expand_expr(self))

    @staticmethod
    def _process_parse(
        parts: tuple[
            Optional[str], tuple[GlobExpr, Sequence[GlobExpr]], Optional[GlobExpr]
        ]
    ) -> GlobGroup:
        prefix, (first_child, children), suffix = parts
        return GlobGroup(prefix or "", [first_child, *children], suffix or "")

    def __str__(self):
        parts = [self.prefix]
        if self.children:
            parts.append("{")
            parts.extend(",".join(map(str, self.children)))
            parts.append("}")
        parts.append(str(self.suffix))
        return "".join(parts)


_expr = forward_decl()
_word = tok("word")
_group = (
    maybe(_word)
    + (  # prefix
        -tok("op", "{")
        + (maybe(_expr) + many(-tok("op", ",") + maybe(_expr)))
        + -tok("op", "}")
    )
    + maybe(_expr)
) >> GlobGroup._process_parse
_expr.define(_group | _word)
_whole_expr = _expr + -finished

if __name__ == "__main__":
    if len(args := sys.argv) != 2:
        print("Expected only one argument", file=sys.stderr)
        exit(1)

    glob = sys.argv[1]
    expr = Glob.parse(glob)
    if os.getenv("DEBUG") == "globparse":
        pprint.print(expr)
        print()
        print()
    for expand in expr.expand():
        print(expand)
