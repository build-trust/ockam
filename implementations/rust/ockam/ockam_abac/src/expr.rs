use core::fmt;
use minicbor::{Decode, Encode};
use ockam_core::compat::string::String;
use ockam_core::compat::vec::Vec;

#[cfg(test)]
use quickcheck::{Arbitrary, Gen};

#[derive(Debug, Clone, PartialEq, PartialOrd, Encode, Decode)]
#[rustfmt::skip]
pub enum Expr {
    #[n(1)] Str   (#[n(0)] String),
    #[n(2)] Int   (#[n(0)] i64),
    #[n(3)] Float (#[n(0)] f64),
    #[n(4)] Bool  (#[n(0)] bool),
    #[n(5)] Ident (#[n(0)] String),
    #[n(6)] Seq   (#[n(0)] Vec<Expr>),
    #[n(7)] List  (#[n(0)] Vec<Expr>)
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

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Str(s) => write!(f, "{s:?}"),
            Expr::Int(i) => write!(f, "{i}"),
            Expr::Float(x) => {
                if x.is_nan() {
                    f.write_str("nan")
                } else if x.is_infinite() {
                    if x.is_sign_negative() {
                        f.write_str("-inf")
                    } else {
                        f.write_str("+inf")
                    }
                } else {
                    write!(f, "{:?}", x)
                }
            }
            Expr::Bool(b) => write!(f, "{b}"),
            Expr::Ident(v) => f.write_str(v),
            Expr::List(es) => {
                f.write_str("(")?;
                let mut n = es.len();
                for e in es {
                    if n > 1 {
                        write!(f, "{e} ")?
                    } else {
                        write!(f, "{e}")?
                    }
                    n -= 1;
                }
                f.write_str(")")
            }
            Expr::Seq(es) => {
                f.write_str("[")?;
                let mut n = es.len();
                for e in es {
                    if n > 1 {
                        write!(f, "{e} ")?
                    } else {
                        write!(f, "{e}")?
                    }
                    n -= 1;
                }
                f.write_str("]")
            }
        }
    }
}

#[cfg(test)]
impl Arbitrary for Expr {
    fn arbitrary(g: &mut Gen) -> Self {
        fn gen_string() -> String {
            use rand::distributions::{Alphanumeric, DistString};
            let mut s = Alphanumeric.sample_string(&mut rand::thread_rng(), 23);
            s.retain(|c| !['(', ')', '[', ']'].contains(&c));
            s
        }
        match g.choose(&[1, 2, 3, 4, 5, 6, 7]).unwrap() {
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
            _ => Expr::List(Arbitrary::arbitrary(g)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Expr;
    use crate::parser::parse;
    use ockam_core::compat::string::ToString;
    use quickcheck::{Gen, QuickCheck};

    #[test]
    fn write_read() {
        fn property(e: Expr) -> bool {
            let s = e.to_string();
            let x = parse(&s).unwrap();
            Some(e) == x
        }
        QuickCheck::new()
            .gen(Gen::new(4))
            .quickcheck(property as fn(_) -> bool)
    }
}
