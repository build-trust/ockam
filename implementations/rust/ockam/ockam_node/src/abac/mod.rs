#![allow(dead_code)]
#![allow(missing_docs)]

pub mod mem;

use ockam_core::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait Abac {
    async fn set_subject<I>(&self, s: Subject, attrs: I)
    where
        I: IntoIterator<Item = (String, Val)> + Send + 'static;

    async fn set_policy(&self, r: Resource, a: Action, c: Cond);

    async fn del_subject(&self, s: &Subject);

    async fn del_policy(&self, r: &Resource);

    async fn is_authorised(&self, s: &Subject, r: &Resource, a: &Action) -> bool;
}

/// Subject identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Subject(u64);

/// Resource identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Resource(String);

/// Action identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Action(String);

impl From<&str> for Resource {
    fn from(s: &str) -> Self {
        Resource(s.to_string())
    }
}

impl From<&str> for Action {
    fn from(s: &str) -> Self {
        Action(s.to_string())
    }
}

/// Attribute value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Val {
    S(String),
    I(i64),
    B(bool),
}

pub fn string<S: Into<String>>(s: S) -> Val {
    Val::S(s.into())
}

pub fn int(n: i64) -> Val {
    Val::I(n)
}

pub fn bool(b: bool) -> Val {
    Val::B(b)
}

/// Policy condition.
#[derive(Debug, Clone)]
pub enum Cond {
    False,
    True,
    Eq(String, Val),
    Lt(String, Val),
    Gt(String, Val),
    Not(Box<Cond>),
    And(Vec<Cond>),
    Or(Vec<Cond>),
}

impl Cond {
    pub fn apply(&self, attrs: &HashMap<String, Val>) -> bool {
        match self {
            Cond::True => true,
            Cond::False => false,
            Cond::Eq(k, v) => attrs.get(k).map(|a| a == v).unwrap_or(false),
            Cond::Lt(k, v) => attrs.get(k).map(|a| a < v).unwrap_or(false),
            Cond::Gt(k, v) => attrs.get(k).map(|a| a > v).unwrap_or(false),
            Cond::Not(c) => !c.apply(attrs),
            Cond::And(cs) => cs.iter().all(|c| c.apply(attrs)),
            Cond::Or(cs) => cs.iter().any(|c| c.apply(attrs)),
        }
    }

    pub fn and(self, other: Cond) -> Cond {
        Cond::And(vec![self, other])
    }

    pub fn or(self, other: Cond) -> Cond {
        Cond::Or(vec![self, other])
    }

    pub fn all(self, mut others: Vec<Cond>) -> Cond {
        others.insert(0, self);
        Cond::And(others)
    }

    pub fn any(self, mut others: Vec<Cond>) -> Cond {
        others.insert(0, self);
        Cond::Or(others)
    }
}

pub fn t() -> Cond {
    Cond::True
}

pub fn f() -> Cond {
    Cond::False
}

pub fn eq<S: Into<String>>(k: S, a: Val) -> Cond {
    Cond::Eq(k.into(), a)
}

pub fn lt<S: Into<String>>(k: S, a: Val) -> Cond {
    Cond::Lt(k.into(), a)
}

pub fn gt<S: Into<String>>(k: S, a: Val) -> Cond {
    Cond::Gt(k.into(), a)
}

pub fn not(c: Cond) -> Cond {
    Cond::Not(c.into())
}
