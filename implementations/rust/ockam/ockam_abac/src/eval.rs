use crate::env::Env;
use crate::error::EvalError;
use crate::expr::{unit, Expr};
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;

#[rustfmt::skip]
pub fn eval(expr: &Expr, env: &Env) -> Result<Expr, EvalError> {
    match expr {
        Expr::Ident(id) => env.get(id).cloned(),
        Expr::List(es)  => match &es[..] {
            []                    => Ok(unit()),
            [Expr::Ident(id), ..] => {
                match id.as_str() {
                    "and"     => eval_and(&es[1..], env),
                    "or"      => eval_or(&es[1..], env),
                    "not"     => eval_not(&es[1..], env),
                    "if"      => eval_if(&es[1..], env),
                    "<"       => eval_pred(&es[1..], env, "<",  |a, b| a < b),
                    ">"       => eval_pred(&es[1..], env, ">",  |a, b| a > b),
                    "="       => eval_eq(&es[1..], env),
                    "!="      => eval_ne(&es[1..], env),
                    "member?" => eval_in(&es[1..], env),
                    _         => Err(EvalError::Unknown(id.to_string()))
                }
            }
            [other, ..] => Err(EvalError::InvalidType(other.clone(), "expected (op ...)"))
        }
        Expr::Seq(es) => {
            let xs = es.iter().map(|e| eval(e, env)).collect::<Result<_, _>>()?;
            Ok(Expr::Seq(xs))
        }
        expr => Ok(expr.clone())
    }
}

fn eval_and(expr: &[Expr], env: &Env) -> Result<Expr, EvalError> {
    for e in expr {
        match eval(e, env)? {
            Expr::Bool(true) => continue,
            Expr::Bool(false) => return Ok(Expr::Bool(false)),
            other => return Err(EvalError::InvalidType(other, "expected bool")),
        }
    }
    Ok(Expr::Bool(true))
}

fn eval_or(expr: &[Expr], env: &Env) -> Result<Expr, EvalError> {
    for e in expr {
        match eval(e, env)? {
            Expr::Bool(true) => return Ok(Expr::Bool(true)),
            Expr::Bool(false) => continue,
            other => return Err(EvalError::InvalidType(other, "expected bool")),
        }
    }
    Ok(Expr::Bool(false))
}

fn eval_if(expr: &[Expr], env: &Env) -> Result<Expr, EvalError> {
    match expr {
        [test, t, f] => match eval(test, env)? {
            Expr::Bool(true) => eval(t, env),
            Expr::Bool(false) => eval(f, env),
            other => Err(EvalError::InvalidType(other, "expected bool")),
        },
        _ => Err(EvalError::malformed(
            "expected (if <test> <consequent> <alternative>)",
        )),
    }
}

fn eval_not(expr: &[Expr], env: &Env) -> Result<Expr, EvalError> {
    if expr.is_empty() {
        return Err(EvalError::malformed("not requires an argument"));
    }
    match eval(&expr[0], env)? {
        Expr::Bool(b) => Ok(Expr::Bool(!b)),
        other => Err(EvalError::InvalidType(other, "expected bool")),
    }
}

fn eval_eq(expr: &[Expr], env: &Env) -> Result<Expr, EvalError> {
    if let Some(a) = expr.first() {
        let x = eval(a, env)?;
        for e in expr.iter().skip(1) {
            let y = eval(e, env)?;
            if x != y {
                return Ok(Expr::Bool(false));
            }
        }
    }
    Ok(Expr::Bool(true))
}

fn eval_ne(expr: &[Expr], env: &Env) -> Result<Expr, EvalError> {
    match eval_eq(expr, env)? {
        Expr::Bool(b) => Ok(Expr::Bool(!b)),
        other => Err(EvalError::InvalidType(other, "expected bool")),
    }
}

fn eval_in(expr: &[Expr], env: &Env) -> Result<Expr, EvalError> {
    if expr.len() != 2 {
        return Err(EvalError::malformed("in requires two arguments"));
    }
    let a = eval(&expr[0], env)?;
    match eval(&expr[1], env)? {
        Expr::Seq(vs) => Ok(Expr::Bool(vs.contains(&a))),
        other => Err(EvalError::InvalidType(other, "expected sequence")),
    }
}

fn eval_pred<F>(expr: &[Expr], env: &Env, op: &str, pred: F) -> Result<Expr, EvalError>
where
    F: Fn(&Expr, &Expr) -> bool,
{
    if expr.len() < 2 {
        let msg = format!("{op} requires at least two arguments");
        return Err(EvalError::malformed(msg));
    }
    let mut last = eval(&expr[0], env)?;
    for x in &expr[1..] {
        let y = eval(x, env)?;
        if !pred(&last, &y) {
            return Ok(Expr::Bool(false));
        }
        last = y
    }
    Ok(Expr::Bool(true))
}
