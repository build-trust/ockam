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
                    "and" => eval_and(&es[1..], env),
                    "or"  => eval_or(&es[1..], env),
                    "not" => eval_not(&es[1..], env),
                    "if"  => eval_if(&es[1..], env),
                    "+"   => eval_arith(&es[1..], env, Op::Add),
                    "*"   => eval_arith(&es[1..], env, Op::Mul),
                    "-"   => eval_arith(&es[1..], env, Op::Sub),
                    "/"   => eval_arith(&es[1..], env, Op::Div),
                    "<"   => eval_pred(&es[1..], env, "<",  |a, b| a < b),
                    "<="  => eval_pred(&es[1..], env, "<=", |a, b| a <= b),
                    ">"   => eval_pred(&es[1..], env, ">",  |a, b| a > b),
                    ">="  => eval_pred(&es[1..], env, ">=", |a, b| a >= b),
                    "in" | "member" => eval_in(&es[1..], env),
                    "="  | "eq?"    => eval_eq(&es[1..], env),
                    "!=" | "ne?"    => eval_ne(&es[1..], env),
                    _               => Err(EvalError::Unknown(id.to_string()))
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

enum Op {
    Add,
    Mul,
    Sub,
    Div,
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

fn eval_arith(expr: &[Expr], env: &Env, op: Op) -> Result<Expr, EvalError> {
    match expr {
        [] => match op {
            Op::Add => Ok(Expr::Int(0)),
            Op::Mul => Ok(Expr::Int(1)),
            Op::Sub => Err(EvalError::malformed("- requires an argument")),
            Op::Div => Err(EvalError::malformed("/ requires an argument")),
        },
        [a, ..] => match eval(a, env)? {
            Expr::Int(mut i) => {
                for e in &expr[1..] {
                    match eval(e, env)? {
                        Expr::Int(j) => match op {
                            Op::Add => i = i.checked_add(j).ok_or(EvalError::Overflow)?,
                            Op::Mul => i = i.checked_mul(j).ok_or(EvalError::Overflow)?,
                            Op::Sub => i = i.checked_sub(j).ok_or(EvalError::Underflow)?,
                            Op::Div => i = i.checked_div(j).ok_or(EvalError::Division)?,
                        },
                        other => return Err(EvalError::InvalidType(other, "expected int")),
                    }
                }
                match op {
                    Op::Sub if expr.len() == 1 => Ok(Expr::Int(-i)),
                    Op::Div if expr.len() == 1 => {
                        Ok(Expr::Int(1i64.checked_div(i).ok_or(EvalError::Division)?))
                    }
                    _ => Ok(Expr::Int(i)),
                }
            }
            Expr::Float(mut i) => {
                for e in &expr[1..] {
                    match eval(e, env)? {
                        Expr::Float(j) => match op {
                            Op::Add => i += j,
                            Op::Mul => i *= j,
                            Op::Sub => i -= j,
                            Op::Div => i /= j,
                        },
                        other => return Err(EvalError::InvalidType(other, "expected float")),
                    }
                }
                match op {
                    Op::Sub if expr.len() == 1 => Ok(Expr::Float(-i)),
                    Op::Div if expr.len() == 1 => Ok(Expr::Float(1.0 / i)),
                    _ => Ok(Expr::Float(i)),
                }
            }
            other => Err(EvalError::InvalidType(other, "expected int")),
        },
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
        let msg = format!("{op} requires at leat two arguments");
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
