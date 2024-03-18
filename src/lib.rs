#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
mod expand;

use crate::expand::GlobGroup;
use chumsky::prelude::*;
use std::str::FromStr;

pub use crate::expand::ExpandGlobIter;

type Text = Box<str>;
type ErrorImpl = Simple<char>;

//
// main type
//

/// A glob expansion expression
#[derive(Debug, Clone)]
pub struct GlobExpr {
    kind: GlobExprKind,
}
impl GlobExpr {
    /// Expand the glob expression, iterating
    #[inline]
    pub fn expand(&self) -> ExpandGlobIter<'_> {
        ExpandGlobIter::new(self)
    }
}
impl GlobExpr {
    fn literal(text: impl Into<Text>) -> Self {
        GlobExpr {
            kind: GlobExprKind::Literal(text.into()),
        }
    }

    /// A [`chumsky::Parser`] for glob expressions
    fn parser() -> impl Parser<char, GlobExpr, Error = ErrorImpl> {
        let open_group = just('{');
        let close_group = just('}');
        let comma = just(',');
        let escape_char = just('\\');
        let special_char = open_group.or(close_group).or(comma).or(escape_char);
        let escape_seq = escape_char
            .then(special_char)
            .map(|(_, escaped_char)| escaped_char);
        let literal = special_char
            .not()
            .or(escape_seq)
            .repeated()
            .collect::<String>()
            .map(GlobExpr::literal);

        let expr = recursive(|expr| {
            assert_parser_type::<char, GlobExpr>(&expr);
            let group = {
                let group_pattern = expr
                    .clone()
                    .separated_by(comma)
                    .delimited_by(open_group, close_group);
                literal
                    .then(group_pattern)
                    .then(expr)
                    .map(|((prefix, children), suffix)| {
                        GlobGroup {
                            prefix,
                            children,
                            suffix,
                        }
                        .into_expr()
                    })
            };
            group.or(literal)
        });

        expr.then_ignore(end())
    }
}
impl FromStr for GlobExpr {
    type Err = GlobParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        GlobExpr::parser().parse(s).map_err(|mut causes| {
            let first_error = causes.swap_remove(0);
            GlobParseError { first_error }
        })
    }
}

/// An error that occurs parsing a glob expression
#[derive(thiserror::Error, Debug)]
#[error("{first_error}")]
pub struct GlobParseError {
    // for now, only report first error
    first_error: ErrorImpl,
}

#[derive(Debug, Clone)]
enum GlobExprKind {
    Literal(Text),
    Group(Box<GlobGroup>),
}

fn assert_parser_type<I: Clone, O>(_parser: &impl Parser<I, O>) {}
