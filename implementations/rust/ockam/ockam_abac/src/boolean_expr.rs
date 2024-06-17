use crate::{Expr, SUBJECT_KEY};
#[cfg(feature = "std")]
use core::str::FromStr;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::fmt::*;
use ockam_core::compat::format;
use ockam_core::compat::string::*;
use ockam_core::compat::vec::*;
#[cfg(feature = "std")]
use ockam_core::Result;
#[cfg(feature = "std")]
use std::ops::Not;
#[cfg(feature = "std")]
use winnow::error::{ContextError, ErrMode, StrContext};
#[cfg(feature = "std")]
use winnow::Parser;
use Expr::*;

#[cfg(feature = "std")]
const NAME_FORMAT: &str =
    "an alphanumerical name, separated with '.', '-' or '_'. The first character cannot be a digit or a '.'";

/// A BooleanExpr models a boolean expression made of:
///
///  - Names.
///  - Binary operators: and, or.
///  - Unary operator: not.
///  - Optional parentheses: 'and' takes precedence over 'or', and 'not' over 'and'.
///
/// A BooleanExpr can be:
///
///  - Parsed from a string
///  - Printed as a string
///  - Transformed into a policy expression where names become boolean attributes set to the value 'true'.
///
#[derive(Debug, Clone, Encode, Decode, CborLen)]
pub enum BooleanExpr {
    #[n(0)]
    Name(#[n(0)] String),
    #[n(1)]
    Or(#[n(0)] Box<BooleanExpr>, #[n(1)] Box<BooleanExpr>),
    #[n(2)]
    And(#[n(0)] Box<BooleanExpr>, #[n(1)] Box<BooleanExpr>),
    #[n(3)]
    Not(#[n(0)] Box<BooleanExpr>),
    #[n(4)]
    Empty,
}

impl PartialEq for BooleanExpr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BooleanExpr::Name(n1), BooleanExpr::Name(n2)) => n1 == n2,
            (BooleanExpr::Or(e1, e2), BooleanExpr::Or(e3, e4)) => e1 == e3 && e2 == e4,
            (BooleanExpr::And(e1, e2), BooleanExpr::And(e3, e4)) => e1 == e3 && e2 == e4,
            (BooleanExpr::Not(e1), BooleanExpr::Not(e2)) => e1 == e2,
            _ => false,
        }
    }
}

impl Eq for BooleanExpr {}

#[cfg(feature = "std")]
impl Display for BooleanExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        fn to_nested_string(b: &BooleanExpr) -> String {
            match b {
                BooleanExpr::Name(s) => s.clone(),
                BooleanExpr::Or(e1, e2) => format!("({e1} or {e2})"),
                BooleanExpr::And(e1, e2) => format!("({e1} and {e2})"),
                BooleanExpr::Not(e) => format!("(not {e})"),
                BooleanExpr::Empty => "".to_string(),
            }
        }

        match self {
            BooleanExpr::Name(s) => f.write_str(s),
            BooleanExpr::Or(e1, e2) => f.write_str(&format!(
                "{} or {}",
                to_nested_string(e1),
                to_nested_string(e2)
            )),
            BooleanExpr::And(e1, e2) => f.write_str(&format!(
                "{} and {}",
                to_nested_string(e1),
                to_nested_string(e2)
            )),
            BooleanExpr::Not(e) => f.write_str(&format!("not {}", to_nested_string(e))),
            BooleanExpr::Empty => f.write_str(""),
        }
    }
}

#[cfg(feature = "std")]
impl TryFrom<&str> for BooleanExpr {
    type Error = crate::ParseError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        let input = input.to_string();
        let mut i = input.as_str();
        BooleanExpr::parse(&mut i)
    }
}

#[cfg(feature = "std")]
impl FromStr for BooleanExpr {
    type Err = crate::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

#[cfg(feature = "std")]
impl TryFrom<String> for BooleanExpr {
    type Error = crate::ParseError;

    fn try_from(input: String) -> Result<Self, Self::Error> {
        Self::try_from(input.as_str())
    }
}

impl From<BooleanExpr> for Expr {
    fn from(value: BooleanExpr) -> Self {
        value.to_expression()
    }
}

impl BooleanExpr {
    /// Create a name to be used in a boolean expression.
    pub fn name(s: &str) -> BooleanExpr {
        BooleanExpr::Name(s.to_string())
    }

    /// Create the disjunction of 2 boolean expressions.
    pub fn or(e1: BooleanExpr, e2: BooleanExpr) -> BooleanExpr {
        BooleanExpr::Or(Box::new(e1), Box::new(e2))
    }

    /// Create the conjunction of 2 boolean expressions.
    pub fn and(e1: BooleanExpr, e2: BooleanExpr) -> BooleanExpr {
        BooleanExpr::And(Box::new(e1), Box::new(e2))
    }

    /// Create the negation of a boolean expression.
    #[allow(clippy::should_implement_trait)]
    pub fn not(e: BooleanExpr) -> BooleanExpr {
        BooleanExpr::Not(Box::new(e))
    }

    /// Create a empty BooleanExpr (it is mostly useful to reduce a possibly empty list of BooleanExpr)
    pub fn empty() -> BooleanExpr {
        BooleanExpr::Empty
    }

    /// Transform this boolean expression into a policy expression
    /// by using names as attributes and setting them to the value 'true'
    ///
    /// Note: there is no attempt to normalize the expression and for example
    /// transform `not a` into `= subject.a "false"`
    pub fn to_expression(&self) -> Expr {
        match self {
            BooleanExpr::Name(s) => List(vec![
                Ident("=".to_string()),
                Ident(format!("{}.{}", SUBJECT_KEY, s)),
                Str("true".to_string()),
            ]),
            BooleanExpr::Or(e1, e2) => List(vec![
                Ident("or".to_string()),
                e1.to_expression(),
                e2.to_expression(),
            ]),
            BooleanExpr::And(e1, e2) => List(vec![
                Ident("and".to_string()),
                e1.to_expression(),
                e2.to_expression(),
            ]),
            BooleanExpr::Not(e) => List(vec![Ident("not".to_string()), e.to_expression()]),
            BooleanExpr::Empty => List(vec![]),
        }
    }

    /// Parse a string as a boolean expression
    #[cfg(feature = "std")]
    pub fn parse(input: &mut &str) -> Result<BooleanExpr, crate::ParseError> {
        parsers::expr
            .parse_next(input)
            .map_err(|e| {
                let messages = match e {
                    ErrMode::Backtrack(c) => {
                        let context: ContextError<StrContext> = c;
                        context
                            .context()
                            .map(|c| format!("{c}"))
                            // just display the deepest context message
                            .take(1)
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                    e => format!("{e:?}"),
                };
                crate::ParseError::message(messages)
            })
            .and_then(|expr| {
                if input.is_empty() {
                    Ok(expr)
                } else {
                    Err(crate::ParseError::message(format!(
                        "successfully parsed: `{expr}`, but `{input}` cannot be parsed"
                    )))
                }
            })
    }
}

#[cfg(feature = "std")]
impl Not for BooleanExpr {
    type Output = BooleanExpr;

    fn not(self) -> Self::Output {
        BooleanExpr::not(self)
    }
}

/// Parsers for boolean expressions.
///
/// The grammar is:
///
///    expr : and_expr (or and_expr)*
///    and_expr : not_expr (or not_expr)*
///    not_expr : not not_expr | parenthesized | name
///    parenthesized : '(' expr ')'
///    name : (alphanum | '.' | '_' | '-')+
#[cfg(feature = "std")]
mod parsers {
    use crate::boolean_expr::{BooleanExpr, NAME_FORMAT};
    use winnow::ascii::multispace0;
    use winnow::combinator::{alt, delimited, separated};
    use winnow::error::StrContext;
    use winnow::stream::AsChar;
    use winnow::token::{literal, take_while};
    use winnow::{PResult, Parser};

    /// Top-level parser for boolean expressions as a series of 'or-ed' and-expressions
    pub fn expr(i: &mut &str) -> PResult<BooleanExpr> {
        fn or_separated(i: &mut &str) -> PResult<Vec<BooleanExpr>> {
            separated(1.., and_expr, or).parse_next(i)
        }

        Ok(or_separated
            .context(StrContext::Expected("expression (or expression)*".into()))
            .parse_next(i)?
            .into_iter()
            .reduce(BooleanExpr::or)
            .unwrap_or(BooleanExpr::empty()))
    }

    /// Parser for an and expression as a series of 'and-ed' not-expressions
    pub fn and_expr(i: &mut &str) -> PResult<BooleanExpr> {
        fn and_separated(i: &mut &str) -> PResult<Vec<BooleanExpr>> {
            separated(1.., not_expr, and).parse_next(i)
        }

        Ok(and_separated
            .context(StrContext::Expected("expression (and expression)*".into()))
            .parse_next(i)?
            .into_iter()
            .reduce(BooleanExpr::and)
            .unwrap_or(BooleanExpr::empty()))
    }

    /// Parser for a not expression as either:
    ///  - a nested not expression
    ///  - a parenthesized expression
    ///  - a single name
    pub fn not_expr(i: &mut &str) -> PResult<BooleanExpr> {
        fn nested_not_expr(i: &mut &str) -> PResult<BooleanExpr> {
            (not, not_expr)
                .parse_next(i)
                .map(|(_, e)| BooleanExpr::not(e))
        }
        fn parenthesized(i: &mut &str) -> PResult<BooleanExpr> {
            delimited(open_paren, expr, close_paren).parse_next(i)
        }
        alt([nested_not_expr, parenthesized, name])
            .context(StrContext::Expected("not expression".into()))
            .parse_next(i)
    }

    // LEXED VALUES

    /// Parse a name
    pub fn name(input: &mut &str) -> PResult<BooleanExpr> {
        fn parse_identifier(input: &mut &str) -> PResult<String> {
            (
                // we forbid the first character to be a number or a dot
                take_while(1..2, |c| AsChar::is_alpha(c) || c == '_' || c == '-'),
                take_while(0.., |c| {
                    AsChar::is_alphanum(c) || c == '.' || c == '_' || c == '-'
                }),
            )
                .map(|(c, cs): (&str, &str)| format!("{c}{cs}"))
                .parse_next(input)
        }

        parse_identifier
            .context(StrContext::Expected(NAME_FORMAT.into()))
            .parse_next(input)
            .map(|vs| BooleanExpr::Name(vs.to_string()))
    }

    /// Parse the 'and' operator
    pub fn and<'a>(input: &mut &'a str) -> PResult<&'a str> {
        delimited(multispace0, literal("and"), multispace0).parse_next(input)
    }

    /// Parse the 'or' operator
    pub fn or<'a>(input: &mut &'a str) -> PResult<&'a str> {
        delimited(multispace0, literal("or"), multispace0).parse_next(input)
    }

    /// Parse the 'not' operator
    pub fn not<'a>(input: &mut &'a str) -> PResult<&'a str> {
        delimited(multispace0, literal("not"), multispace0).parse_next(input)
    }

    /// Parse an open parentheses '('
    pub fn open_paren<'a>(input: &mut &'a str) -> PResult<&'a str> {
        delimited(multispace0, literal("("), multispace0).parse_next(input)
    }

    /// Parse a close parentheses ')'
    pub fn close_paren<'a>(input: &mut &'a str) -> PResult<&'a str> {
        delimited(multispace0, literal(")"), multispace0).parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use core::fmt::Debug;
    use parsers::*;
    use winnow::Parser;

    #[test]
    fn boolean_expr_to_expr() {
        let boolean_expr = BooleanExpr::name("a");
        let expr = parse("(= subject.a \"true\")").unwrap().unwrap();
        assert_eq!(boolean_expr.to_expression(), expr);

        let boolean_expr = BooleanExpr::and(
            BooleanExpr::or(BooleanExpr::name("a"), BooleanExpr::name("b")),
            BooleanExpr::not(BooleanExpr::name("c")),
        );
        let expr = parse(
            "and (or (= subject.a \"true\") (= subject.b \"true\") (not (= subject.c \"true\")))",
        )
        .unwrap()
        .unwrap();
        assert_eq!(boolean_expr.to_expression(), expr);
    }

    #[test]
    fn boolean_expr_to_string() {
        let boolean_expr = BooleanExpr::name("a");
        let expr = "a".to_string();
        assert_eq!(boolean_expr.to_string(), expr);

        let boolean_expr = BooleanExpr::and(BooleanExpr::name("a"), BooleanExpr::name("b"));
        let expr = "a and b".to_string();
        assert_eq!(boolean_expr.to_string(), expr);

        let boolean_expr = BooleanExpr::or(BooleanExpr::name("a"), BooleanExpr::name("b"));
        let expr = "a or b".to_string();
        assert_eq!(boolean_expr.to_string(), expr);

        let boolean_expr = BooleanExpr::and(
            BooleanExpr::or(BooleanExpr::name("a"), BooleanExpr::name("b")),
            BooleanExpr::not(BooleanExpr::name("c")),
        );
        let expr = "(a or b) and (not c)".to_string();
        assert_eq!(boolean_expr.to_string(), expr);
    }

    #[test]
    fn parse_name() {
        test_parse_name("name");
        test_parse_name("name.1");
        test_parse_name("a-b");
        test_parse_name("a-b-c");
        test_parse_name("a.b.c");
        test_parse_name("a_b-c");
        test_parse_name("a1_2b-3c4");
        test_parse_name("___reserved");

        test_fail_parse_name("*");
        test_fail_parse_name("1");
    }

    #[test]
    fn parse_boolean_expr() {
        test_parse_expr(
            &mut "a and b",
            BooleanExpr::and(BooleanExpr::name("a"), BooleanExpr::name("b")),
        );
        test_parse_expr(
            &mut "a and b and c",
            BooleanExpr::and(
                BooleanExpr::and(BooleanExpr::name("a"), BooleanExpr::name("b")),
                BooleanExpr::name("c"),
            ),
        );
        test_parse_expr(
            &mut "a or b",
            BooleanExpr::or(BooleanExpr::name("a"), BooleanExpr::name("b")),
        );
        test_parse_expr(
            &mut "a or b or c",
            BooleanExpr::or(
                BooleanExpr::or(BooleanExpr::name("a"), BooleanExpr::name("b")),
                BooleanExpr::name("c"),
            ),
        );
        test_parse_expr(&mut "not a", BooleanExpr::not(BooleanExpr::name("a")));
        test_parse_expr(
            &mut "not (not a)",
            BooleanExpr::not(BooleanExpr::not(BooleanExpr::name("a"))),
        );
        test_parse_expr(&mut "(not a)", BooleanExpr::not(BooleanExpr::name("a")));
        test_parse_expr(&mut "( ( (a )))", BooleanExpr::name("a"));
        test_parse_expr(
            &mut "(a and b)",
            BooleanExpr::and(BooleanExpr::name("a"), BooleanExpr::name("b")),
        );
        test_parse_expr(
            &mut "(a or b)",
            BooleanExpr::or(BooleanExpr::name("a"), BooleanExpr::name("b")),
        );
        test_parse_expr(
            &mut "(a or b) and (not c)",
            BooleanExpr::and(
                BooleanExpr::or(BooleanExpr::name("a"), BooleanExpr::name("b")),
                BooleanExpr::not(BooleanExpr::name("c")),
            ),
        );
        test_parse_expr(
            &mut "((a or b) and (not c))",
            BooleanExpr::and(
                BooleanExpr::or(BooleanExpr::name("a"), BooleanExpr::name("b")),
                BooleanExpr::not(BooleanExpr::name("c")),
            ),
        );

        // check the precedence of operators: not > and > or
        test_parse_expr(
            &mut "a or b and not c",
            BooleanExpr::or(
                BooleanExpr::name("a"),
                BooleanExpr::and(
                    BooleanExpr::name("b"),
                    BooleanExpr::not(BooleanExpr::name("c")),
                ),
            ),
        );
    }

    #[test]
    fn parse_boolean_expr_errors() {
        test_parse_error(
            &mut "na*me",
            "successfully parsed: `na`, but `*me` cannot be parsed",
        );
        test_parse_error(&mut "()", &format!("expected `{NAME_FORMAT}`"));
        test_parse_error(
            &mut "a and",
            "successfully parsed: `a`, but ` and` cannot be parsed",
        );
        test_parse_error(
            &mut "a and b not c",
            "successfully parsed: `a and b`, but ` not c` cannot be parsed",
        );
        test_parse_error(
            &mut "(a and b) or (c and d))",
            "successfully parsed: `(a and b) or (c and d)`, but `)` cannot be parsed",
        );
    }

    /// HELPERS

    /// Test the parsing of a name
    fn test_parse_name(input: &str) {
        let i = input.to_string();
        test_parse(
            &mut name,
            &mut i.as_str(),
            BooleanExpr::Name(input.to_string()),
        )
    }

    /// Test a name parsing failure
    fn test_fail_parse_name(input: &str) {
        let i = input.to_string();
        let input_copy = input.to_string();
        let expected = NAME_FORMAT;
        match name.parse_next(&mut i.as_str()) {
            Ok(actual) => panic!("there should be an error '{expected}', when parsing {input_copy}. This expression was parsed instead {actual:?}"),
            Err(e) => assert!(e.to_string().contains(expected), "actual error message:\n{e}\nexpected message:\n{expected}"),
        }
    }

    /// Test the parsing of a boolean expression
    fn test_parse_expr(input: &mut &str, expected: BooleanExpr) {
        let i = input.to_string();
        test_parse(&mut expr, &mut i.as_str(), expected)
    }

    /// Test a parser with a successful input
    fn test_parse<'a, O: Debug + PartialEq + Eq, E: Debug>(
        parser: &mut impl Parser<&'a str, O, E>,
        input: &mut &'a str,
        expected: O,
    ) {
        let input_copy = input.to_string();
        match parser.parse_next(input) {
            Ok(actual) => assert_eq!(actual, expected),
            Err(e) => panic!("error {e:?}. The input is {input_copy}"),
        }
    }

    /// Test a parser with a failing input
    fn test_parse_error(input: &mut &str, expected: &str) {
        let input_copy = input.to_string();
        match BooleanExpr::parse(input) {
            Ok(actual) => panic!("there should be an error '{expected}', when parsing {input_copy}. This expression was parsed instead {actual:?}"),
            Err(crate::ParseError::Message(e)) => assert!(e.contains(expected), "actual error message:\n{e}\nexpected message:\n{expected}"),
            Err(e) => panic!("expected a Message ParseError, got: {e}"),
        }
    }
}
