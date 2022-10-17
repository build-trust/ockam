use crate::error::ParseError;
use crate::expr::Expr;
use core::str;
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_core::compat::vec::Vec;
use wast::lexer::{FloatVal, Lexer, Token};

pub fn parse(s: &str) -> Result<Option<Expr>, ParseError> {
    fn go(lx: &mut Lexer, xs: &mut Vec<Expr>) -> Result<(), ParseError> {
        while let Some(tk) = next(lx)? {
            match tk {
                Token::Whitespace(_) | Token::LineComment(_) | Token::BlockComment(_) => continue,
                Token::Integer(i) => {
                    let (s, r) = i.val();
                    let x = i64::from_str_radix(s, r)?;
                    xs.push(Expr::Int(x))
                }
                Token::Float(v) => match v.val() {
                    FloatVal::Inf { negative: true } => xs.push(Expr::Float(f64::NEG_INFINITY)),
                    FloatVal::Inf { negative: false } => xs.push(Expr::Float(f64::INFINITY)),
                    FloatVal::Nan { .. } => xs.push(Expr::Float(f64::NAN)),
                    FloatVal::Val { .. } => {
                        let x: f64 = v.src().parse()?;
                        xs.push(Expr::Float(x))
                    }
                },
                Token::String(s) => xs.push(Expr::Str(str::from_utf8(s.val())?.to_string())),
                Token::LParen(_) => {
                    let mut ys = Vec::new();
                    go(lx, &mut ys)?;
                    xs.push(Expr::List(ys))
                }
                Token::RParen(_) => return Ok(()),
                Token::Reserved("]") => return Ok(()),
                Token::Reserved("[") => {
                    let mut ys = Vec::new();
                    go(lx, &mut ys)?;
                    xs.push(Expr::Seq(ys))
                }
                Token::Reserved(t @ ("," | ";" | "{" | "}")) => {
                    return Err(ParseError::message(format!("invalid token '{t}'")))
                }
                Token::Keyword("true") => xs.push(Expr::Bool(true)),
                Token::Keyword("false") => xs.push(Expr::Bool(false)),
                Token::Id(v) => xs.push(Expr::Ident(v.to_string())),
                Token::Keyword(v) => xs.push(Expr::Ident(v.to_string())),
                Token::Reserved(v) => xs.push(Expr::Ident(v.to_string())),
            }
        }
        Ok(())
    }

    let mut lx = Lexer::new(s);
    let mut xs = Vec::new();
    go(&mut lx, &mut xs)?;

    match xs.len() {
        0 => Ok(None),
        1 => Ok(Some(xs.remove(0))),
        _ => Ok(Some(Expr::List(xs))),
    }
}

fn next<'a>(lx: &mut Lexer<'a>) -> Result<Option<Token<'a>>, ParseError> {
    while let Some(tk) = lx.parse()? {
        match tk {
            Token::Whitespace(_) | Token::LineComment(_) | Token::BlockComment(_) => continue,
            other => return Ok(Some(other)),
        }
    }
    Ok(None)
}
