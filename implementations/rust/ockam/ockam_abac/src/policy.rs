use crate::{Action, Key, Resource, Subject, Value};

use ockam_core::compat::{boxed::Box, vec::Vec};
use serde::{Deserialize, Serialize};

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec;

/// Pimitive conditional operators used to construct ABAC policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Conditional {
    /// Equality condition
    Eq(Key, Value),
    /// Equality condition
    Lt(Key, Value),
    /// Equality condition
    Gt(Key, Value),
    /// Boolean condition
    Not(Box<Conditional>),
    /// Boolean condition
    And(Vec<Conditional>),
    /// Boolean condition
    Or(Vec<Conditional>),
    /// Always true
    True,
    /// Always false
    False,
}

impl Conditional {
    /// Evaluate Policy for the given [`Subject`], [`Resource`],
    /// [`Action`].
    ///
    /// TODO add support for resource, action attributes
    pub fn evaluate(&self, subject: &Subject, resource: &Resource, action: &Action) -> bool {
        let attrs = subject.attributes();
        match self {
            Conditional::Eq(k, v) => attrs.get(k).map(|a| a == v).unwrap_or(false),
            Conditional::Lt(k, v) => attrs.get(k).map(|a| a < v).unwrap_or(false),
            Conditional::Gt(k, v) => attrs.get(k).map(|a| a > v).unwrap_or(false),
            Conditional::Not(c) => !c.evaluate(subject, resource, action),
            Conditional::And(cs) => cs.iter().all(|c| c.evaluate(subject, resource, action)),
            Conditional::Or(cs) => cs.iter().any(|c| c.evaluate(subject, resource, action)),
            Conditional::True => true,
            Conditional::False => false,
        }
    }

    /// Create a new `Conditional::And` with the given `Conditional`.
    pub fn and(&self, other: &Conditional) -> Conditional {
        Conditional::And(vec![self.clone(), other.clone()])
    }

    /// Create a new `Conditional::Or` with the given `Conditional`.
    pub fn or(&self, other: &Conditional) -> Conditional {
        Conditional::Or(vec![self.clone(), other.clone()])
    }

    /// Create a new `Conditional::And` with the given [`Vec`] of `Conditional`s.
    pub fn all(self, mut others: Vec<Conditional>) -> Conditional {
        others.insert(0, self);
        Conditional::And(others)
    }

    /// Create a new `Conditional::Or` with the given [`Vec`] of `Conditional`s.
    pub fn any(self, mut others: Vec<Conditional>) -> Conditional {
        others.insert(0, self);
        Conditional::Or(others)
    }
}

/// Create a new [`Conditional::Eq`].
pub fn eq<K: Into<Key>>(k: K, a: Value) -> Conditional {
    Conditional::Eq(k.into(), a)
}

/// Create a new [`Conditional::Lt`].
pub fn lt<K: Into<Key>>(k: K, a: Value) -> Conditional {
    Conditional::Lt(k.into(), a)
}

/// Create a new [`Conditional::Gt`].
pub fn gt<K: Into<Key>>(k: K, a: Value) -> Conditional {
    Conditional::Gt(k.into(), a)
}

/// Create a new [`Conditional::Not`].
pub fn not(c: Conditional) -> Conditional {
    Conditional::Not(c.into())
}

/// Create a new [`Conditional::True`].
pub fn t() -> Conditional {
    Conditional::True
}

/// Create a new [`Conditional::False`].
pub fn f() -> Conditional {
    Conditional::False
}
