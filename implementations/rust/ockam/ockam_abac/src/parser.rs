use crate::expr::Expr;
use crate::{error::ParseError, EvalError};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::format;
use ockam_core::compat::str::{from_utf8, FromStr};
use ockam_core::compat::string::ToString;
use ockam_core::compat::vec::Vec;
use once_cell::race::OnceBox;
use regex::Regex;

use wast::lexer::Lexer;
use wast::lexer::{FloatKind, TokenKind};

/// Allowed identifier patterns.
fn ident_pattern() -> &'static Regex {
    static INSTANCE: OnceBox<Regex> = OnceBox::new();
    INSTANCE.get_or_init(|| {
        Box::new(Regex::new("^[a-zA-Z!$%&*/<=>?~_^][a-zA-Z0-9!$%&*/<=>?~_^.+-@]*$").unwrap())
    })
}

pub const OPERATORS: [&str; 10] = [
    "and", "or", "not", "if", "<", ">", "=", "!=", "member?", "exists?",
];

#[rustfmt::skip]
pub fn parse(s: &str) -> Result<Option<Expr>, ParseError> {
    /// A stack operation.
    enum Op {
        Next,
        Value(Expr),
        ListStart,
        ListEnd,
        SeqStart,
        SeqEnd,
    }

    let lx = Lexer::new(s);

    // Control stack.
    let mut ctrl: Vec<Op> = Vec::new();

    // Result values.
    let mut vals: Vec<Expr> = Vec::new();

    // Start by parsing the next expression.
    ctrl.push(Op::Next);

    let mut parse_position = 0;
    while let Some(e) = ctrl.pop() {
        match e {
            Op::Next => match lx.parse(&mut parse_position)? {
                None => continue,
                Some(token) => {
                    match token.kind {
                        TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment =>
                            ctrl.push(Op::Next),
                        TokenKind::Integer(integer_kind) => {
                            let integer = token.integer(s, integer_kind);
                            let (s, r) = integer.val();
                            let x = i64::from_str_radix(s, r)?;
                            ctrl.push(Op::Value(Expr::Int(x)));
                            ctrl.push(Op::Next)
                        }
                        TokenKind::Float(float_kind) => {
                            match float_kind {
                                FloatKind::Inf { negative: true } =>
                                    ctrl.push(Op::Value(Expr::Float(f64::NEG_INFINITY))),
                                FloatKind::Inf { negative: false } =>
                                    ctrl.push(Op::Value(Expr::Float(f64::INFINITY))),
                                FloatKind::Nan { .. } =>
                                    ctrl.push(Op::Value(Expr::Float(f64::NAN))),
                                FloatKind::NanVal { .. } =>
                                    ctrl.push(Op::Value(Expr::Float(f64::NAN))),
                                FloatKind::Normal { .. } => {
                                    let float: f64 = FromStr::from_str(token.src(s))?;
                                    ctrl.push(Op::Value(Expr::Float(float)))
                                }
                            }
                            ctrl.push(Op::Next)
                        }
                        TokenKind::String => {
                            ctrl.push(Op::Value(Expr::Str(from_utf8(&token.string(s))?.to_string())));
                            ctrl.push(Op::Next)
                        }
                        TokenKind::LParen => {
                            ctrl.push(Op::ListStart);
                            ctrl.push(Op::Next)
                        }
                        TokenKind::RParen => {
                            ctrl.push(Op::ListEnd)
                        }
                        TokenKind::Reserved if token.reserved(s) == "]" => {
                            ctrl.push(Op::SeqEnd)
                        }
                        TokenKind::Reserved if token.reserved(s) == "[" => {
                            ctrl.push(Op::SeqStart);
                            ctrl.push(Op::Next)
                        }
                        TokenKind::Keyword if token.keyword(s) == "true" => {
                            ctrl.push(Op::Value(Expr::Bool(true)));
                            ctrl.push(Op::Next)
                        }
                        TokenKind::Keyword if token.keyword(s) == "false" => {
                            ctrl.push(Op::Value(Expr::Bool(false)));
                            ctrl.push(Op::Next)
                        }
                        TokenKind::Id => {
                            ctrl.push(Op::Value(Expr::Ident(token.id(s).to_string())));
                            ctrl.push(Op::Next)
                        }
                        TokenKind::Keyword => {
                            let keyword = token.keyword(s);
                            if ident_pattern().is_match(keyword) {
                                ctrl.push(Op::Value(Expr::Ident(keyword.to_string())));
                                ctrl.push(Op::Next)
                            } else {
                                return Err(ParseError::message(format!("invalid keyword token '{keyword}'")))
                            }
                        }
                        TokenKind::Reserved  => {
                            let reserved = token.reserved(s);
                            if ident_pattern().is_match(reserved) {
                                ctrl.push(Op::Value(Expr::Ident(reserved.to_string())));
                                ctrl.push(Op::Next)
                            } else {
                                return Err(ParseError::message(format!("invalid reserved token '{reserved}'")))
                            }
                        }
                    }
                }
            }
            Op::Value(x) => vals.push(x),
            Op::ListEnd => {
                let mut v = Vec::new();
                while let Some(x) = ctrl.pop() {
                    match x {
                        Op::ListStart => break,
                        Op::Value(x)  => v.push(x),
                        Op::ListEnd   => return Err(ParseError::message("')' without matching '('")),
                        Op::SeqStart  => return Err(ParseError::message("'[' without matching ']'")),
                        Op::SeqEnd    => return Err(ParseError::message("']' without matching '['")),
                        Op::Next      => unreachable!("consecutive next operations are impossible")
                    }
                }
                v.reverse();
                ctrl.push(Op::Value(Expr::List(v)));
                ctrl.push(Op::Next)
            }
            Op::SeqEnd => {
                let mut v = Vec::new();
                while let Some(x) = ctrl.pop() {
                    match x {
                        Op::SeqStart  => break,
                        Op::Value(x)  => v.push(x),
                        Op::ListEnd   => return Err(ParseError::message("')' without matching '('")),
                        Op::ListStart => return Err(ParseError::message("'(' without matching ')'")),
                        Op::SeqEnd    => return Err(ParseError::message("']' without matching '['")),
                        Op::Next      => unreachable!("consecutive next operations are impossible")
                    }
                }
                v.reverse();
                for (x, y) in v.iter().zip(v.iter().skip(1)) {
                    if let Err(EvalError::TypeMismatch(x, y)) = x.equals(y) {
                        return Err(ParseError::TypeMismatch(x, y))
                    }
                }
                ctrl.push(Op::Value(Expr::Seq(v)));
                ctrl.push(Op::Next)
            }
            Op::ListStart => return Err(ParseError::message("unclosed '('")),
            Op::SeqStart  => return Err(ParseError::message("unclosed '['"))
        }
    }

    let expression = match vals.len() {
        0 => None,
        1 => Some(vals.remove(0)),
        _ => {
            vals.reverse();
            Some(Expr::List(vals))
        }
    };
    match expression {
        Some(e) => if is_operation(&e) {
            Ok(Some(e))
        } else {
            Err(ParseError::message(format!("The first identifier of the expression: `{s}` must be an operation. The available operations are: {}", OPERATORS.join(", "))))
        },
        None => Ok(None),
    }
}

/// Return true if this expression starts with an operation
fn is_operation(expression: &Expr) -> bool {
    match expression {
        Expr::Ident(name) => OPERATORS.contains(&name.as_str()),
        Expr::List(vs) => match vs.first() {
            Some(v) => is_operation(v),
            None => false,
        },
        _ => false,
    }
}
