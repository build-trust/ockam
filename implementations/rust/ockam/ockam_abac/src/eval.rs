use core::cmp::Ordering;

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
                        "and" => {
                            // 'and' evaluates its arguments lazily. As soon as a
                            // false value is encountered evaluation stops and
                            // the remaining arguments are just popped off the
                            // control stack. To implement this the `And` operator
                            // is put behind the first ergument and will later put
                            // itself behind each successive argument, stopping
                            // evaluation as soon as an argument evaluates to
                            // false.
                            if nargs == 0 {
                                args.push(Expr::Bool(true))
                            } else {
                                for x in xs[2 ..].iter().rev() {
                                    ctrl.push(Op::Eval(x))
                                }
                                ctrl.push(Op::And(nargs - 1));
                                ctrl.push(Op::Eval(&xs[1]))
                            }
                            continue
                        }
                        "or" => {
                            // 'or' evaluates its arguments lazily. As soon as a
                            // true value is encountered evaluation stops and
                            // the remaining arguments are just popped off the
                            // control stack. To implement this the `Or` operator
                            // is put behind the first ergument and will later put
                            // itself behind each successive argument, stopping
                            // evaluation as soon as an argument evaluates to
                            // true.
                            if nargs == 0 {
                                args.push(Expr::Bool(false))
                            } else {
                                for x in xs[2 ..].iter().rev() {
                                    ctrl.push(Op::Eval(x))
                                }
                                ctrl.push(Op::Or(nargs - 1));
                                ctrl.push(Op::Eval(&xs[1]))
                            }
                            continue
                        }
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
                            // We first evaluate the test and only then, depending on the result
                            // do we either evaluate the true branch or the false branch.
                            ctrl.push(Op::Eval(&xs[3])); // false branch
                            ctrl.push(Op::Eval(&xs[2])); // true branch
                            ctrl.push(Op::If);
                            ctrl.push(Op::Eval(&xs[1])); // test
                            continue
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
            Op::And(0) => {} // the top-level element of the arg stack is the result
            Op::And(n) => {
                match pop(&mut args) {
                    Expr::Bool(true) => {
                        let x = pop(&mut ctrl);
                        ctrl.push(Op::And(n - 1));
                        ctrl.push(x)
                    }
                    Expr::Bool(false) => {
                        for _ in 0 .. n {
                            pop(&mut ctrl);
                        }
                        args.push(Expr::Bool(false))
                    }
                    other => {
                        let msg = "'and' expects boolean arguments";
                        return Err(EvalError::InvalidType(other, msg))
                    }
                }
            }
            Op::Or(0) => {} // the top-level element of the arg stack is the result
            Op::Or(n) => {
                match pop(&mut args) {
                    Expr::Bool(false) => {
                        let x = pop(&mut ctrl);
                        ctrl.push(Op::Or(n - 1));
                        ctrl.push(x)
                    }
                    Expr::Bool(true) => {
                        for _ in 0 .. n {
                            pop(&mut ctrl);
                        }
                        args.push(Expr::Bool(true))
                    }
                    other => {
                        let msg = "'or' expects boolean arguments";
                        return Err(EvalError::InvalidType(other, msg))
                    }
                }
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
                let t = pop(&mut ctrl);
                let f = pop(&mut ctrl);
                match pop(&mut args) {
                    Expr::Bool(true)  => ctrl.push(t),
                    Expr::Bool(false) => ctrl.push(f),
                    other => {
                        let msg = "'if' expects test to evaluate to bool";
                        return Err(EvalError::InvalidType(other, msg))
                    }
                }
            }
            Op::Eq(n) => eval_predicate(n, &mut args, |x, y| x.equals(y))?,
            Op::Lt(n) => eval_predicate(n, &mut args, |x, y| {
                x.compare(y).map(|o| o == Some(Ordering::Less))
            })?,
            Op::Gt(n) => eval_predicate(n, &mut args, |x, y| {
                x.compare(y).map(|o| o == Some(Ordering::Greater))
            })?,
            Op::Member => {
                let s = pop(&mut args);
                let y = pop(&mut args);
                match s {
                    Expr::Seq(xs) => {
                        let mut b = false;
                        for x in &xs {
                            if y.equals(x)? {
                                b = true;
                                break
                            }
                        }
                        args.push(Expr::Bool(b))
                    }
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
fn eval_predicate<F>(n: usize, args: &mut Vec<Expr>, f: F) -> Result<(), EvalError>
where
    F: Fn(&Expr, &Expr) -> Result<bool, EvalError>,
{
    let mut b = true;
    let start = args.len() - n;
    for (x, y) in args.iter().skip(start).zip(args.iter().skip(start + 1)) {
        if !f(x, y)? {
            b = false;
            break;
        }
    }
    args.truncate(start);
    args.push(Expr::Bool(b));
    Ok(())
}
