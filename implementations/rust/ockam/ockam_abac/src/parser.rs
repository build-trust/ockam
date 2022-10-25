use crate::error::ParseError;
use crate::expr::Expr;
use core::str;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_core::compat::vec::Vec;
use once_cell::race::OnceBox;
use regex::Regex;
use wast::lexer::{FloatVal, Lexer, Token};

/// Allowed identifier patterns.
fn ident_pattern() -> &'static Regex {
    static INSTANCE: OnceBox<Regex> = OnceBox::new();
    INSTANCE.get_or_init(|| {
        Box::new(Regex::new("^[a-zA-Z!$%&*/<=>?~_^][a-zA-Z0-9!$%&*/<=>?~_^.+-@]*$").unwrap())
    })
}

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

    let mut lx = Lexer::new(s);

    // Control stack.
    let mut ctrl: Vec<Op> = Vec::new();

    // Result values.
    let mut vals: Vec<Expr> = Vec::new();

    // Start by parsing the next expression.
    ctrl.push(Op::Next);

    while let Some(e) = ctrl.pop() {
        match e {
            Op::Next => match lx.parse()? {
                None => continue,
                Some(Token::Whitespace(_) | Token::LineComment(_) | Token::BlockComment(_)) =>
                    ctrl.push(Op::Next),
                Some(Token::Integer(i)) => {
                    let (s, r) = i.val();
                    let x = i64::from_str_radix(s, r)?;
                    ctrl.push(Op::Value(Expr::Int(x)));
                    ctrl.push(Op::Next)
                }
                Some(Token::Float(v)) => {
                    match v.val() {
                        FloatVal::Inf { negative: true } =>
                            ctrl.push(Op::Value(Expr::Float(f64::NEG_INFINITY))),
                        FloatVal::Inf { negative: false } =>
                            ctrl.push(Op::Value(Expr::Float(f64::INFINITY))),
                        FloatVal::Nan { .. } =>
                            ctrl.push(Op::Value(Expr::Float(f64::NAN))),
                        FloatVal::Val { .. } => {
                            let x: f64 = v.src().parse()?;
                            ctrl.push(Op::Value(Expr::Float(x)))
                        }
                    }
                    ctrl.push(Op::Next)
                }
                Some(Token::String(s)) => {
                    ctrl.push(Op::Value(Expr::Str(str::from_utf8(s.val())?.to_string())));
                    ctrl.push(Op::Next)
                }
                Some(Token::LParen(_)) => {
                    ctrl.push(Op::ListStart);
                    ctrl.push(Op::Next)
                }
                Some(Token::RParen(_)) => {
                    ctrl.push(Op::ListEnd)
                }
                Some(Token::Reserved("]")) => {
                    ctrl.push(Op::SeqEnd)
                }
                Some(Token::Reserved("[")) => {
                    ctrl.push(Op::SeqStart);
                    ctrl.push(Op::Next)
                }
                Some(Token::Keyword("true")) => {
                    ctrl.push(Op::Value(Expr::Bool(true)));
                    ctrl.push(Op::Next)
                }
                Some(Token::Keyword("false")) => {
                    ctrl.push(Op::Value(Expr::Bool(false)));
                    ctrl.push(Op::Next)
                }
                Some(Token::Id(v)) => {
                    ctrl.push(Op::Value(Expr::Ident(v.to_string())));
                    ctrl.push(Op::Next)
                }
                Some(Token::Keyword(v) | Token::Reserved(v)) => {
                    if ident_pattern().is_match(v) {
                        ctrl.push(Op::Value(Expr::Ident(v.to_string())));
                        ctrl.push(Op::Next)
                    } else {
                        return Err(ParseError::message(format!("invalid token '{v}'")))
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
