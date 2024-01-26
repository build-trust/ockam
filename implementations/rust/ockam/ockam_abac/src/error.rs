use crate::expr::Expr;
use core::fmt;
use core::num::{ParseFloatError, ParseIntError};
use core::str::Utf8Error;
use ockam_core::compat::string::{String, ToString};
use ockam_core::errcode::{Kind, Origin};

#[derive(Debug)]
pub enum ParseError {
    Utf8(Utf8Error),
    Int(ParseIntError),
    Float(ParseFloatError),
    Other(String),
    Message(String),
    TypeMismatch(Expr, Expr),
}

#[derive(Debug)]
pub enum EvalError {
    Unbound(String),
    Unknown(String),
    InvalidType(Expr, &'static str),
    TypeMismatch(Expr, Expr),
    Malformed(String),
}

#[derive(Debug)]
pub enum MergeError {
    BindingExists(String),
}

impl ParseError {
    pub fn message<S: Into<String>>(s: S) -> Self {
        ParseError::Message(s.into())
    }
}

impl EvalError {
    pub fn malformed<S: Into<String>>(s: S) -> Self {
        EvalError::Malformed(s.into())
    }

    pub fn is_unbound(&self) -> bool {
        matches!(self, EvalError::Unbound(_))
    }
}

impl From<Utf8Error> for ParseError {
    #[track_caller]
    fn from(e: Utf8Error) -> Self {
        Self::Utf8(e)
    }
}

impl From<ParseIntError> for ParseError {
    #[track_caller]
    fn from(e: ParseIntError) -> Self {
        Self::Int(e)
    }
}

impl From<ParseFloatError> for ParseError {
    #[track_caller]
    fn from(e: ParseFloatError) -> Self {
        Self::Float(e)
    }
}

#[cfg(feature = "std")]
impl From<wast::Error> for ParseError {
    #[track_caller]
    fn from(e: wast::Error) -> Self {
        Self::Other(format!("{e:?}"))
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::Other(e) => write!(f, "{e}"),
            ParseError::Float(e) => write!(f, "{e}"),
            ParseError::Int(e) => write!(f, "{e}"),
            ParseError::Utf8(e) => write!(f, "{e}"),
            ParseError::Message(m) => f.write_str(m),
            ParseError::TypeMismatch(a, b) => write!(f, "{a} and {b} are not of the same type"),
        }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EvalError::Unbound(id) => write!(f, "unbound identifier: {id}"),
            EvalError::Unknown(id) => write!(f, "unknown operator: {id}"),
            EvalError::InvalidType(e, m) => write!(f, "invalid type of expression {e}: {m}"),
            EvalError::Malformed(m) => write!(f, "malformed expression: {m}"),
            EvalError::TypeMismatch(a, b) => write!(f, "{a} and {b} are not of the same type"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Float(e) => Some(e),
            ParseError::Int(e) => Some(e),
            ParseError::Utf8(e) => Some(e),
            ParseError::Other(_) => None,
            ParseError::Message(_) => None,
            ParseError::TypeMismatch(..) => None,
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EvalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<ParseError> for ockam_core::Error {
    #[track_caller]
    fn from(e: ParseError) -> Self {
        ockam_core::Error::new(Origin::Application, Kind::Invalid, e.to_string())
    }
}

impl From<EvalError> for ockam_core::Error {
    #[track_caller]
    fn from(e: EvalError) -> Self {
        ockam_core::Error::new(Origin::Application, Kind::Invalid, e.to_string())
    }
}
