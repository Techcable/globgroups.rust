use chumsky::prelude::*;

use super::{GlobExpr, GlobGroup, Literal};

pub(crate) type ErrorImpl = Simple<char>;

macro_rules! impl_special_char {
    ($($name:ident => $c:literal),*) => (impl SpecialChar {
        #[inline]
        pub fn from_char(c: char) -> Option<Self> {
            match c {
                $($c => Some(Self::$name),)*
                _ => None
            }
        }

        #[inline]
        pub fn to_char(self) -> char {
            match self {
                $(Self::$name => $c),*
            }
        }

        #[inline]
        pub fn parser(&self) -> impl Parser<char, SpecialChar, Error = ErrorImpl> + Copy {
            just(self.to_char()).to(*self)
        }

        #[inline]
        pub fn parse_any() -> impl Parser<char, SpecialChar, Error = ErrorImpl> + Copy {
            choice([$(Self::$name.parser()),*])
        }
    });
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum SpecialChar {
    OpenGroup,
    CloseGroup,
    Comma,
    Escape,
}
impl_special_char! {
    OpenGroup => '{',
    CloseGroup => '}',
    Comma => ',',
    Escape => '\\'
}

fn escape_seq() -> impl Parser<char, char, Error = ErrorImpl> + Copy {
    SpecialChar::Escape
        .parser()
        .then(SpecialChar::parse_any())
        .map(|(_, escaped)| escaped.to_char())
        .labelled("escape sequence")
}

pub fn literal() -> impl Parser<char, Literal, Error = ErrorImpl> + Copy {
    SpecialChar::parse_any()
        .not()
        .or(escape_seq())
        .repeated()
        .collect::<String>()
        .map(Literal::from_text)
}

/// A [`chumsky::Parser`] for glob expressions
pub fn expr() -> impl Parser<char, GlobExpr, Error = ErrorImpl> {
    let literal_expr = literal().map(Literal::into_expr);
    recursive(|expr| {
        assert_parser_type::<char, GlobExpr>(&expr);
        let group = {
            let group_pattern = expr
                .clone()
                .separated_by(SpecialChar::Comma.parser())
                .delimited_by(
                    SpecialChar::OpenGroup.parser(),
                    SpecialChar::CloseGroup.parser(),
                );
            literal_expr
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
        group.or(literal_expr)
    })
}

fn assert_parser_type<I: Clone, O>(_parser: &impl Parser<I, O>) {}
