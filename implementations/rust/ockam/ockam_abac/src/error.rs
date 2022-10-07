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
    Other(wast::Error),
    Message(String),
}

impl ParseError {
    pub fn message<S: Into<String>>(s: S) -> Self {
        ParseError::Message(s.into())
    }
}

#[derive(Debug)]
pub enum EvalError {
    Unbound(String),
    Unknown(String),
    InvalidType(Expr, &'static str),
    Malformed(String),
    Overflow,
    Underflow,
    Division,
}

impl EvalError {
    pub fn malformed<S: Into<String>>(s: S) -> Self {
        EvalError::Malformed(s.into())
    }
}

impl From<Utf8Error> for ParseError {
    fn from(e: Utf8Error) -> Self {
        Self::Utf8(e)
    }
}

impl From<ParseIntError> for ParseError {
    fn from(e: ParseIntError) -> Self {
        Self::Int(e)
    }
}

impl From<ParseFloatError> for ParseError {
    fn from(e: ParseFloatError) -> Self {
        Self::Float(e)
    }
}

impl From<wast::Error> for ParseError {
    fn from(e: wast::Error) -> Self {
        Self::Other(e)
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
            EvalError::Overflow => f.write_str("numeric overflow"),
            EvalError::Underflow => f.write_str("numeric underflow"),
            EvalError::Division => f.write_str("numeric division error"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Other(e) => Some(e),
            ParseError::Float(e) => Some(e),
            ParseError::Int(e) => Some(e),
            ParseError::Utf8(e) => Some(e),
            ParseError::Message(_) => None,
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
    fn from(e: ParseError) -> Self {
        ockam_core::Error::new(Origin::Application, Kind::Invalid, e.to_string())
    }
}

impl From<EvalError> for ockam_core::Error {
    fn from(e: EvalError) -> Self {
        ockam_core::Error::new(Origin::Application, Kind::Invalid, e.to_string())
    }
}
