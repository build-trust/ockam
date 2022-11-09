use core::fmt;
use core::{cmp::Ordering, str::FromStr};
use minicbor::{Decode, Encode};
use ockam_core::compat::string::String;
use ockam_core::compat::vec::{vec, Vec};

#[cfg(test)]
use quickcheck::{Arbitrary, Gen};

use crate::ParseError;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
pub enum Expr {
    #[n(1)] Str   (#[n(0)] String),
    #[n(2)] Int   (#[n(0)] i64),
    #[n(3)] Float (#[n(0)] f64),
    #[n(4)] Bool  (#[n(0)] bool),
    #[n(5)] Ident (#[n(0)] String),
    #[n(6)] Seq   (#[n(0)] Vec<Expr>),
    #[n(7)] List  (#[n(0)] Vec<Expr>),
    #[n(8)] Null
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
pub enum Val {
    #[n(1)] Str   (#[n(0)] String),
    #[n(2)] Int   (#[n(0)] i64),
    #[n(3)] Float (#[n(0)] f64),
    #[n(4)] Bool  (#[n(0)] bool),
    #[n(5)] Seq   (#[n(0)] Vec<Val>),
    #[n(6)] Null
}

impl From<Val> for Expr {
    fn from(v: Val) -> Self {
        match v {
            Val::Str(s) => Expr::Str(s),
            Val::Int(i) => Expr::Int(i),
            Val::Float(f) => Expr::Float(f),
            Val::Bool(b) => Expr::Bool(b),
            Val::Seq(s) => Expr::Seq(s.into_iter().map(Expr::from).collect()),
            Val::Null => Expr::Null,
        }
    }
}

#[rustfmt::skip]
impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        let mut ctrl = vec![(self, other)];

        while let Some(x) = ctrl.pop() {
            match x {
                (Expr::Str(a),   Expr::Str(b))   => if !a.eq(b) { return false }
                (Expr::Bool(a),  Expr::Bool(b))  => if !a.eq(b) { return false }
                (Expr::Ident(a), Expr::Ident(b)) => if !a.eq(b) { return false }
                (Expr::Int(a),   Expr::Int(b))   => if !a.eq(b) { return false }
                (Expr::Float(a), Expr::Float(b)) => if !a.eq(b) { return false }
                (Expr::Int(a),   Expr::Float(b)) => if !(*a as f64).eq(b)  { return false }
                (Expr::Float(a), Expr::Int(b))   => if !a.eq(&(*b as f64)) { return false }
                (Expr::Seq(a),   Expr::Seq(b))   => {
                    if a.len() != b.len() {
                        return false
                    }
                    for (a, b) in a.iter().zip(b).rev() {
                        ctrl.push((a, b))
                    }
                }
                (Expr::List(a), Expr::List(b)) => {
                    if a.len() != b.len() {
                        return false
                    }
                    for (a, b) in a.iter().zip(b).rev() {
                        ctrl.push((a, b))
                    }
                }
                (Expr::Null, Expr::Null) => {}
                _                        => return false
            }
        }

        true
    }
}

#[rustfmt::skip]
impl PartialOrd for Expr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut ctrl = vec![(self, other)];

        let mut result = None;

        while let Some(x) = ctrl.pop() {
            match x {
                (Expr::Str(a),   Expr::Str(b))   => { result = a.partial_cmp(b) }
                (Expr::Bool(a),  Expr::Bool(b))  => { result = a.partial_cmp(b) }
                (Expr::Ident(a), Expr::Ident(b)) => { result = a.partial_cmp(b) }
                (Expr::Int(a),   Expr::Int(b))   => { result = a.partial_cmp(b) }
                (Expr::Float(a), Expr::Float(b)) => { result = a.partial_cmp(b) }
                (Expr::Int(a),   Expr::Float(b)) => { result = (*a as f64).partial_cmp(b) }
                (Expr::Float(a), Expr::Int(b))   => { result = a.partial_cmp(&(*b as f64)) }
                (Expr::Seq(a),   Expr::Seq(b))   => {
                    result = a.len().partial_cmp(&b.len());
                    if Some(Ordering::Equal) == result {
                        for (a, b) in a.iter().zip(b).rev() {
                            ctrl.push((a, b))
                        }
                        continue
                    } else {
                        return result
                    }
                }
                (Expr::List(a), Expr::List(b)) => {
                    result = a.len().partial_cmp(&b.len());
                    if Some(Ordering::Equal) == result {
                        for (a, b) in a.iter().zip(b).rev() {
                            ctrl.push((a, b))
                        }
                        continue
                    } else {
                        return result
                    }
                }
                (Expr::Null, Expr::Null) => { result = Some(Ordering::Equal) }
                _                        => return None
            }
            if Some(Ordering::Equal) != result {
                return result
            }
        }

        result
    }
}

impl Expr {
    pub fn is_true(&self) -> bool {
        matches!(self, Expr::Bool(true))
    }

    pub fn is_false(&self) -> bool {
        matches!(self, Expr::Bool(false))
    }

    pub fn is_unit(&self) -> bool {
        if let Expr::List(xs) = self {
            xs.is_empty()
        } else {
            false
        }
    }

    pub fn is_ident(&self) -> bool {
        matches!(self, Expr::Ident(_))
    }
}

impl From<bool> for Expr {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for Expr {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<f64> for Expr {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

pub fn t() -> Expr {
    Expr::Bool(true)
}

pub fn f() -> Expr {
    Expr::Bool(false)
}

pub fn unit() -> Expr {
    Expr::List(Vec::new())
}

pub fn int<I: Into<i64>>(i: I) -> Expr {
    Expr::Int(i.into())
}

pub fn float<F: Into<f64>>(f: F) -> Expr {
    Expr::Float(f.into())
}

pub fn ident<S: Into<String>>(s: S) -> Expr {
    Expr::Ident(s.into())
}

pub fn seq<T: IntoIterator<Item = Expr>>(xs: T) -> Expr {
    Expr::Seq(xs.into_iter().collect())
}

pub fn str<S: Into<String>>(s: S) -> Expr {
    Expr::Str(s.into())
}

pub fn and<I>(exprs: I) -> Expr
where
    I: IntoIterator<Item = Expr>,
{
    with_op(ident("and"), exprs)
}

pub fn or<I>(exprs: I) -> Expr
where
    I: IntoIterator<Item = Expr>,
{
    with_op(ident("or"), exprs)
}

pub fn exists<I>(exprs: I) -> Expr
where
    I: IntoIterator<Item = Expr>,
{
    with_op(ident("exists?"), exprs)
}

pub fn when(test: Expr, then: Expr, orelse: Expr) -> Expr {
    with_op(ident("if"), [test, then, orelse])
}

pub fn eq<I>(exprs: I) -> Expr
where
    I: IntoIterator<Item = Expr>,
{
    with_op(ident("="), exprs)
}

fn with_op<I>(op: Expr, exprs: I) -> Expr
where
    I: IntoIterator<Item = Expr>,
{
    let xs = Vec::from_iter([op].into_iter().chain(exprs));
    Expr::List(xs)
}

#[rustfmt::skip]
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        /// Control stack element.
        enum Op<'a> {
            Show(&'a Expr),
            ListEnd,
            SeqEnd,
            Whitespace,
        }

        // Control stack.
        let mut ctrl = vec![Op::Show(self)];

        while let Some(e) = ctrl.pop() {
            match e {
                Op::Show(Expr::Str(s)) => write!(f, "{s:?}")?,
                Op::Show(Expr::Int(i)) => write!(f, "{i}")?,
                Op::Show(Expr::Null)   => f.write_str("null")?,
                Op::Show(Expr::Float(x)) => {
                    if x.is_nan() {
                        f.write_str("nan")?
                    } else if x.is_infinite() {
                        if x.is_sign_negative() {
                            f.write_str("-inf")?
                        } else {
                            f.write_str("+inf")?
                        }
                    } else {
                        write!(f, "{:?}", x)?
                    }
                }
                Op::Show(Expr::Bool(b)) => write!(f, "{b}")?,
                Op::Show(Expr::Ident(v)) => f.write_str(v)?,
                Op::Show(Expr::List(es)) => {
                    ctrl.push(Op::ListEnd);
                    f.write_str("(")?;
                    let mut n = es.len();
                    for e in es.iter().rev() {
                        ctrl.push(Op::Show(e));
                        if n > 1 {
                            ctrl.push(Op::Whitespace)
                        }
                        n -= 1
                    }
                }
                Op::Show(Expr::Seq(es)) => {
                    ctrl.push(Op::SeqEnd);
                    f.write_str("[")?;
                    let mut n = es.len();
                    for e in es.iter().rev() {
                        ctrl.push(Op::Show(e));
                        if n > 1 {
                            ctrl.push(Op::Whitespace)
                        }
                        n -= 1
                    }
                }
                Op::ListEnd    => f.write_str(")")?,
                Op::SeqEnd     => f.write_str("]")?,
                Op::Whitespace => f.write_str(" ")?,
            }
        }

        Ok(())
    }
}

impl TryFrom<&str> for Expr {
    type Error = ParseError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        if let Some(x) = crate::parse(input)? {
            Ok(x)
        } else {
            Err(ParseError::message("empty expression value"))
        }
    }
}

impl TryFrom<String> for Expr {
    type Error = ParseError;

    fn try_from(input: String) -> Result<Self, Self::Error> {
        Self::try_from(input.as_str())
    }
}

impl FromStr for Expr {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

#[cfg(test)]
impl Arbitrary for Expr {
    fn arbitrary(g: &mut Gen) -> Self {
        fn gen_string() -> String {
            use rand::distributions::{Alphanumeric, DistString};
            let mut s = Alphanumeric.sample_string(&mut rand::thread_rng(), 23);
            s.retain(|c| !['(', ')', '[', ']'].contains(&c));
            s.insert(0, 'a');
            s
        }
        match g.choose(&[1, 2, 3, 4, 5, 6, 7, 8]).unwrap() {
            1 => Expr::Str(gen_string()),
            2 => Expr::Int(i64::arbitrary(g)),
            3 => Expr::Float({
                let x = f64::arbitrary(g);
                if x.is_nan() {
                    1.0
                } else {
                    x
                }
            }),
            4 => Expr::Bool(bool::arbitrary(g)),
            5 => Expr::Ident(gen_string()),
            6 => Expr::Seq(Arbitrary::arbitrary(g)),
            7 => Expr::List(Arbitrary::arbitrary(g)),
            _ => Expr::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Expr;
    use crate::{eval, parser::parse, Env};
    use core::cmp::Ordering;
    use ockam_core::compat::string::ToString;
    use quickcheck::{Arbitrary, Gen, QuickCheck};

    #[test]
    fn write_read() {
        fn property(e: Expr) -> bool {
            let s = e.to_string();
            let x = parse(&s).unwrap();
            Some(e) == x
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_) -> bool)
    }

    #[test]
    fn symm_eq() {
        fn property(a: Expr, b: Expr) {
            if a == b {
                assert_eq!(b, a);
                assert_eq!(a.partial_cmp(&b), Some(Ordering::Equal));
                assert_eq!(b.partial_cmp(&a), Some(Ordering::Equal))
            }
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_, _))
    }

    #[test]
    fn trans_eq() {
        fn property(a: Expr, b: Expr, c: Expr) {
            if a == b && b == c {
                assert_eq!(a, c)
            }
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_, _, _))
    }

    #[test]
    fn not_eq() {
        fn property(a: Expr, b: Expr) {
            if a != b {
                assert!(!(a == b))
            }
            if !(a == b) {
                assert!(a != b)
            }
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_, _))
    }

    #[test]
    fn lt() {
        fn property(a: Expr, b: Expr) {
            if a.partial_cmp(&b) == Some(Ordering::Less) {
                assert!(a < b)
            }
            if a < b {
                assert_eq!(a.partial_cmp(&b), Some(Ordering::Less))
            }
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_, _))
    }

    #[test]
    fn lt_eq() {
        fn property(a: Expr, b: Expr) {
            if a <= b {
                assert!(a < b || a == b)
            }
            if a < b || a == b {
                assert!(a <= b)
            }
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_, _))
    }

    #[test]
    fn gt() {
        fn property(a: Expr, b: Expr) {
            if a.partial_cmp(&b) == Some(Ordering::Greater) {
                assert!(a > b)
            }
            if a > b {
                assert_eq!(a.partial_cmp(&b), Some(Ordering::Greater))
            }
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_, _))
    }

    #[test]
    fn gt_eq() {
        fn property(a: Expr, b: Expr) {
            if a >= b {
                assert!(a > b || a == b)
            }
            if a > b || a == b {
                assert!(a >= b)
            }
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_, _))
    }

    #[test]
    fn trans_lt() {
        fn property(a: Expr, b: Expr, c: Expr) {
            if a < b && b < c {
                assert!(a < c)
            }
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_, _, _))
    }

    #[test]
    fn trans_gt() {
        fn property(a: Expr, b: Expr, c: Expr) {
            if a > b && b > c {
                assert!(a > c)
            }
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_, _, _))
    }

    #[test]
    fn dual() {
        fn property(a: Expr, b: Expr) {
            if a > b {
                assert!(b < a)
            }
            if b < a {
                assert!(a > b)
            }
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_, _))
    }

    const EVIL: &str = r#"
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
         "#;

    #[test]
    fn evil() {
        let x = parse(EVIL).unwrap().unwrap();
        eval(&x, &Env::new()).unwrap();
        let y = x.to_string();
        let z = parse(&y).unwrap().unwrap();
        assert_eq!(x, z);
        assert!(x == z);
        assert_eq!(Some(Ordering::Equal), x.partial_cmp(&z));
    }

    #[derive(Debug, Clone)]
    struct S(String);

    impl Arbitrary for S {
        fn arbitrary(g: &mut Gen) -> Self {
            const ALPHABET: &[char] = &[
                ' ', ')', '(', '[', ']', '"', 'a', 'b', 'c', '1', '2', '3', '.',
            ];
            let mut s = String::new();
            for _ in 0u8..u8::arbitrary(g) {
                s.push(*g.choose(ALPHABET).unwrap())
            }
            s.push('#'); // guarantee parse error
            S(s)
        }
    }

    #[test]
    fn malformed() {
        fn property(s: S) {
            assert!(parse(&s.0).is_err())
        }
        QuickCheck::new()
            .tests(1000)
            .min_tests_passed(1000)
            .quickcheck(property as fn(_))
    }
}
