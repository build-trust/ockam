use crate::expr::Expr;
use crate::{error::ParseError, EvalError};
use core::str;
use core::str::FromStr;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_core::compat::vec::Vec;
use once_cell::race::OnceBox;
use regex::Regex;

#[cfg(feature = "std")]
use wast::lexer::Lexer;
use wast::lexer::{FloatKind, TokenKind};

/// Allowed identifier patterns.
fn ident_pattern() -> &'static Regex {
    static INSTANCE: OnceBox<Regex> = OnceBox::new();
    INSTANCE.get_or_init(|| {
        Box::new(Regex::new("^[a-zA-Z!$%&*/<=>?~_^][a-zA-Z0-9!$%&*/<=>?~_^.+-@]*$").unwrap())
    })
}

#[cfg(feature = "std")]
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
                            ctrl.push(Op::Value(Expr::Str(str::from_utf8(&token.string(s))?.to_string())));
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

    match vals.len() {
        0 => Ok(None),
        1 => Ok(Some(vals.remove(0))),
        _ => {
            vals.reverse();
            Ok(Some(Expr::List(vals)))
        }
    }
}
