use ockam_core::compat::collections::BTreeMap;
use serde::{Deserialize, Serialize};

/// ABAC parameters used to perform an ABAC authorization request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    /// The [`Subject`] performing the authorization request
    pub subject: Subject,
    /// The [`Resource`] the action will be performed on
    pub resource: Resource,
    /// The [`Action`] to request authorization for
    pub action: Action,
}

/// An ABAC `Subject` entity.
///
/// `Subject` will usually map to an entity performing an
/// authorization request such as a user id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Subject(u64);

impl From<u64> for Subject {
    fn from(u: u64) -> Self {
        Subject(u)
    }
}

/// An ABAC `Resource` entity.
///
/// `Resource` maps to the given resource being placed under access
/// control such as a file or network path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Resource(String);

impl From<&str> for Resource {
    fn from(s: &str) -> Self {
        Resource(s.to_string())
    }
}

/// An ABAC `Action` entity.
///
/// `Action` corresponds to the action the requesting `Subject` wants
/// to perform on a `Resource`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Action(String);

impl From<&str> for Action {
    fn from(s: &str) -> Self {
        Action(s.to_string())
    }
}

/// An ABAC `Attribute`
///
/// ABAC attributes are tuples consisting of a string representing the
/// attribute name and the [`Value`] of the attribute.
pub type Attribute = (String, Value);

/// Primitive value types used to construct ABAC attributes and
/// conditionals.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    /// A string
    S(String),
    /// A signed integer
    I(i64),
    /// A boolean
    B(bool),
}

/// Create a new ABAC [`Value::S`] string value.
pub fn string<S: Into<String>>(s: S) -> Value {
    Value::S(s.into())
}

/// Create a new ABAC [`Value::I`] signed integer value.
pub fn int(n: i64) -> Value {
    Value::I(n)
}

/// Create a new ABAC [`Value::B`] boolean value.
pub fn bool(b: bool) -> Value {
    Value::B(b)
}

/// Pimitive conditional operators used to construct ABAC policies.
#[derive(Debug, Clone)]
pub enum Conditional {
    /// Equality condition
    Eq(String, Value),
    /// Equality condition
    Lt(String, Value),
    /// Equality condition
    Gt(String, Value),
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
    /// Evaluate Policy condition with the given [`Attribute`]s.
    pub fn apply(&self, attrs: &BTreeMap<String, Value>) -> bool {
        match self {
            Conditional::Eq(k, v) => attrs.get(k).map(|a| a == v).unwrap_or(false),
            Conditional::Lt(k, v) => attrs.get(k).map(|a| a < v).unwrap_or(false),
            Conditional::Gt(k, v) => attrs.get(k).map(|a| a > v).unwrap_or(false),
            Conditional::Not(c) => !c.apply(attrs),
            Conditional::And(cs) => cs.iter().all(|c| c.apply(attrs)),
            Conditional::Or(cs) => cs.iter().any(|c| c.apply(attrs)),
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
pub fn eq<S: Into<String>>(k: S, a: Value) -> Conditional {
    Conditional::Eq(k.into(), a)
}

/// Create a new [`Conditional::Lt`].
pub fn lt<S: Into<String>>(k: S, a: Value) -> Conditional {
    Conditional::Lt(k.into(), a)
}

/// Create a new [`Conditional::Gt`].
pub fn gt<S: Into<String>>(k: S, a: Value) -> Conditional {
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

#[cfg(test)]
mod tests {
    use super::{eq, gt, int, string, Action, Resource, Subject};
    use crate::abac::mem::Memory;

    #[test]
    fn example1() {
        let is_adult = gt("age", int(17));
        let is_john = eq("name", string("John"));
        let condition = is_adult.or(&is_john);

        let read = Action::from("r");
        let resource = Resource::from("/foo/bar/baz");

        let mem = Memory::new();
        mem.inner
            .write()
            .unwrap()
            .set_policy(resource.clone(), read.clone(), &condition);
        mem.inner.write().unwrap().set_subject(
            Subject::from(1),
            [
                ("name".to_string(), string("John")),
                ("age".to_string(), int(25)),
            ],
        );
        mem.inner.write().unwrap().set_subject(
            Subject::from(2),
            [
                ("name".to_string(), string("Jack")),
                ("age".to_string(), int(12)),
                ("city".to_string(), string("London")),
            ],
        );
        mem.inner.write().unwrap().set_subject(
            Subject::from(3),
            [
                ("name".to_string(), string("Bill")),
                ("age".to_string(), int(32)),
            ],
        );

        assert!(mem
            .inner
            .read()
            .unwrap()
            .is_authorized(&Subject::from(1), &resource, &read)); // John
        assert!(mem
            .inner
            .read()
            .unwrap()
            .is_authorized(&Subject::from(3), &resource, &read)); // adult
        assert!(!mem
            .inner
            .read()
            .unwrap()
            .is_authorized(&Subject::from(2), &resource, &read)); // not John and no adult
    }
}
