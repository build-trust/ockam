use crate::env::Env;
use crate::error::EvalError;
use crate::expr::{unit, Expr};
use ockam_core::compat::string::ToString;
use ockam_core::compat::vec::Vec;

#[rustfmt::skip]
pub fn eval(expr: &Expr, env: &Env) -> Result<Expr, EvalError> {
    /// A stack operation.
    ///
    /// Each operation uses the arguments stack as input. The number of
    /// arguments to pop off the stack is either static (e.g. 1 in case
    /// of `Not`) or a `usize` parameter of the operation.
    enum Op<'a> {
        Eval(&'a Expr),
        And(usize),
        Or(usize),
        Not,
        If,
        Eq(usize),
        Gt(usize),
        Lt(usize),
        Member,
        Seq(usize),
    }

    // Control stack.
    let mut ctrl: Vec<Op> = Vec::new();
    // Arguments stack.
    let mut args: Vec<Expr> = Vec::new();

    // Start with the toplevel expression.
    ctrl.push(Op::Eval(expr));

    while let Some(x) = ctrl.pop() {
        match x {
            Op::Eval(Expr::Ident(id)) => ctrl.push(Op::Eval(env.get(id)?)),
            Op::Eval(Expr::List(xs))  => match &xs[..] {
                []                    => args.push(unit()),
                [Expr::Ident(id), ..] => {
                    let nargs = xs.len() - 1; // number of arguments
                    match id.as_str() {
                        "and" => ctrl.push(Op::And(nargs)),
                        "or"  => ctrl.push(Op::Or(nargs)),
                        "not" => {
                            if nargs != 1 {
                                return Err(EvalError::malformed("'not' requires one argument"))
                            }
                            ctrl.push(Op::Not)
                        }
                        "if" => {
                            if nargs != 3 {
                                return Err(EvalError::malformed("'if' requires three arguments"))
                            }
                            ctrl.push(Op::If)
                        }
                        "<" => {
                            if nargs < 2 {
                                let msg = "'<' requires at least two arguments";
                                return Err(EvalError::malformed(msg))
                            }
                            ctrl.push(Op::Lt(nargs))
                        }
                        ">" => {
                            if nargs < 2 {
                                let msg = "'>' requires at least two arguments";
                                return Err(EvalError::malformed(msg))
                            }
                            ctrl.push(Op::Gt(nargs))
                        }
                        "=" => {
                            if nargs < 2 {
                                let msg = "'=' requires at least two arguments";
                                return Err(EvalError::malformed(msg))
                            }
                            ctrl.push(Op::Eq(nargs))
                        }
                        "!=" => {
                            ctrl.push(Op::Not);
                            ctrl.push(Op::Eq(nargs))
                        }
                        "member?" => {
                            if nargs != 2 {
                                let msg = "'member?' requires two arguments";
                                return Err(EvalError::malformed(msg))
                            }
                            ctrl.push(Op::Member)
                        }
                        "exists?" => {
                            let mut b = true;
                            for x in &xs[1 ..] {
                                match x {
                                    Expr::Ident(id) => if !env.contains(id) {
                                        b = false;
                                        break
                                    }
                                    other => {
                                        let msg = "'exists?' expects identifiers as arguments";
                                        return Err(EvalError::InvalidType(other.clone(), msg))
                                    }
                                }
                            }
                            args.push(Expr::Bool(b));
                            continue
                        }
                        _  => return Err(EvalError::Unknown(id.to_string()))
                    }
                    for x in xs[1 ..].iter().rev() {
                        ctrl.push(Op::Eval(x))
                    }
                }
                [other, ..] => {
                    let msg = "expected (op ...)";
                    return Err(EvalError::InvalidType(other.clone(), msg))
                }
            }
            Op::Eval(Expr::Seq(xs)) => {
                ctrl.push(Op::Seq(xs.len()));
                for x in xs.iter().rev() {
                    ctrl.push(Op::Eval(x))
                }
            }
            Op::Eval(expr) => args.push(expr.clone()),
            Op::And(n) => {
                let mut b = true;
                for x in args.drain(args.len() - n ..) {
                    match x {
                        Expr::Bool(true)  => continue,
                        Expr::Bool(false) => { b = false; break }
                        other => {
                            let msg = "'and' expects boolean arguments";
                            return Err(EvalError::InvalidType(other, msg))
                        }
                    }
                }
                args.push(Expr::Bool(b))
            }
            Op::Or(n) => {
                let mut b = false ;
                for x in args.drain(args.len() - n ..) {
                    match x {
                        Expr::Bool(true)  => { b = true; break }
                        Expr::Bool(false) => continue,
                        other => {
                            let msg = "'or' expects boolean arguments";
                            return Err(EvalError::InvalidType(other, msg))
                        }
                    }
                }
                args.push(Expr::Bool(b))
            }
            Op::Not => {
                match pop(&mut args) {
                    Expr::Bool(b) => args.push(Expr::Bool(!b)),
                    other => {
                        let msg = "'not' expects boolean arguments";
                        return Err(EvalError::InvalidType(other, msg))
                    }
                }
            }
            Op::If => {
                let f = pop(&mut args);
                let t = pop(&mut args);
                match pop(&mut args) {
                    Expr::Bool(true)  => args.push(t),
                    Expr::Bool(false) => args.push(f),
                    other => {
                        let msg = "'if' expects test to evaluate to bool";
                        return Err(EvalError::InvalidType(other, msg))
                    }
                }
            }
            Op::Eq(n) => eval_predicate(n, &mut args, |x, y| x == y),
            Op::Lt(n) => eval_predicate(n, &mut args, |x, y| x < y),
            Op::Gt(n) => eval_predicate(n, &mut args, |x, y| x > y),
            Op::Member => {
                let s = pop(&mut args);
                let x = pop(&mut args);
                match s {
                    Expr::Seq(xs) => args.push(Expr::Bool(xs.contains(&x))),
                    other => {
                        let msg = "'member?' expects sequence as second argument";
                        return Err(EvalError::InvalidType(other, msg))
                    }
                }
            }
            Op::Seq(n) => {
                let s = args.split_off(args.len() - n);
                args.push(Expr::Seq(s))
            }
        }
    }

    debug_assert_eq!(1, args.len());
    Ok(pop(&mut args))
}

/// Pop off the topmost stack value.
///
/// # Panics
///
/// If stack is empty a panic occurs.
fn pop<T>(s: &mut Vec<T>) -> T {
    s.pop().expect("stack is not empty")
}

/// Evaluate a predicate against the `n` topmost arguments.
fn eval_predicate<F>(n: usize, args: &mut Vec<Expr>, f: F)
where
    F: Fn(&Expr, &Expr) -> bool,
{
    let mut b = true;
    let start = args.len() - n;
    for (x, y) in args.iter().skip(start).zip(args.iter().skip(start + 1)) {
        if !f(x, y) {
            b = false;
            break;
        }
    }
    args.truncate(start);
    args.push(Expr::Bool(b))
}
