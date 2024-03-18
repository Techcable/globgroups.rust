#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use chumsky::{primitive::end, Parser};

use crate::expand::GlobGroup;

pub use crate::expand::ExpandGlobIter;

mod expand;
mod parse;

/// A literal value in a glob expression
#[derive(Debug, Clone)]
pub struct Literal(Box<str>);
impl Literal {
    /// Create a literal with the specified text
    ///
    /// This implicitly escapes special characters.
    #[inline]
    pub fn from_text(text: impl Into<String>) -> Self {
        Literal(text.into().into_boxed_str())
    }

    /// Return the underlying text value
    ///
    /// Compared to [`Self::equivalent_expr`],
    /// this method interprets escape sequences.
    #[inline]
    pub fn text(&self) -> &str {
        &self.0
    }

    /// Convert this literal into a [`GlobExpr`]
    #[inline]
    pub fn into_expr(self) -> GlobExpr {
        GlobExpr {
            kind: GlobExprKind::Literal(self),
        }
    }

    /// Return an equivalent expression that will parse to the same value.
    ///
    /// See also: [`GlobExpr::equivalent_expr`]
    pub fn equivalent_expr(&self) -> String {
        let mut buffer = String::new();
        self.write_equivalent_expr(&mut buffer).unwrap();
        buffer
    }

    /// Escape any special characters in the specified text,
    /// returning an [equivalent expression](`Self::equivalent_expr`)
    pub fn escape(text: &str) -> String {
        Literal::from_text(text).equivalent_expr()
    }

    fn write_equivalent_expr(&self, out: &mut impl fmt::Write) -> fmt::Result {
        for c in self.0.chars() {
            if parse::SpecialChar::from_char(c).is_some() {
                out.write_char('\\')?;
            }
            out.write_char(c)?;
        }
        Ok(())
    }
}
impl Display for Literal {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.write_equivalent_expr(f)
    }
}
impl FromStr for Literal {
    type Err = LiteralParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse::literal()
            .then_ignore(end())
            .labelled("literal")
            .parse(text)
            .map_err(GlobParseError::from_causes)
            .map_err(LiteralParseError)
    }
}

/// An error that occurs parsing a literal
///
/// In particular, this can happen when encountering an unescaped `{`
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct LiteralParseError(GlobParseError);

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

    /// Create an expression which is semantically equivalent to this glob
    ///
    /// Not necessarily exactly equal to the original expression.
    pub fn equivalent_expr(&self) -> String {
        let mut buffer = String::new();
        self.write_equivalent_expr(&mut buffer);
        buffer
    }

    fn write_equivalent_expr(&self, buffer: &mut String) {
        match self.kind {
            GlobExprKind::Literal(ref text) => text.write_equivalent_expr(buffer).unwrap(),
            GlobExprKind::Group(ref group) => group.write_equivalent_expr(buffer),
        }
    }
}
impl FromStr for GlobExpr {
    type Err = GlobParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse::expr()
            .then_ignore(end())
            .labelled("glob expression")
            .parse(s)
            .map_err(GlobParseError::from_causes)
    }
}

/// An error that occurs parsing a glob expression
#[derive(thiserror::Error, Debug)]
#[error("{first_error}")]
pub struct GlobParseError {
    // for now, only report first error
    first_error: parse::ErrorImpl,
}
impl GlobParseError {
    pub(crate) fn from_causes(mut causes: Vec<parse::ErrorImpl>) -> Self {
        assert!(!causes.is_empty(), "No causes");
        GlobParseError {
            first_error: causes.swap_remove(0),
        }
    }
}

#[derive(Debug, Clone)]
enum GlobExprKind {
    Literal(Literal),
    Group(Box<GlobGroup>),
}

#[cfg(test)]
mod test {
    use super::*;
    use itertools::Itertools;
    use pretty_assertions::assert_eq;

    struct GlobTest {
        expression: &'static str,
        expected_expansions: &'static [&'static str],
    }
    impl GlobTest {
        pub fn parse(&self) -> GlobExpr {
            self.expression.parse().expect("Parse failure")
        }
    }
    macro_rules! test_data {
        ($($expression:literal => [$($expansion:literal),*]),*) => {
            &[$(GlobTest {
                expression: $expression,
                expected_expansions: &[$($expansion),*]
            }),*]
        };
    }
    const TEST_DATA: &[GlobTest] = test_data! {
        "foo-{bar,baz}-beat" => ["foo-bar-beat", "foo-baz-beat"],
        "foo-{bar,beat{nest,foop}}-baz" => [
            "foo-bar-baz",
            "foo-beatnest-baz",
            "foo-beatfoop-baz"
        ],
        "foo{,-\\{baz{teach,wo\\,\\}}}\\\\\\{" => [
            "foo\\{",
            "foo-{bazteach\\{",
            "foo-{bazwo,}\\{"
        ],
        "feet\\{\\\\\\}" => ["feet{\\}"]
    };

    #[test]
    fn test_expansion() {
        for test in TEST_DATA {
            let expanded = test.parse().expand().join("\n");
            let expected_expansions = test
                .expected_expansions
                .iter()
                .cloned()
                .map(String::from)
                .join("\n");
            assert_eq!(expanded, expected_expansions);
        }
    }

    #[test]
    fn test_roundtrip() {
        for test in TEST_DATA {
            assert_eq!(test.parse().equivalent_expr(), test.expression);
        }
    }
}
