use ockam_core::compat::{
    collections::BTreeMap,
    string::{String, ToString},
};
use ockam_identity::IdentityIdentifier;

use serde::{Deserialize, Serialize};

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::format;
use core::fmt;

/// TODO ockam_identity::IdentityIdentifier ?
pub type Identity = String;

/// An ABAC `Subject` entity.
///
/// `Subject` will usually map to an entity performing an
/// authorization request such as a user id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Subject {
    identifier: Identity,
    attributes: BTreeMap<Key, Value>,
}

impl Subject {
    /// Create a new `Subject` with the given attributes.
    pub fn with_attributes<A>(self, attributes: A) -> Self
    where
        A: IntoIterator<Item = Attribute> + Send + 'static,
    {
        Self {
            identifier: self.identifier,
            attributes: self.attributes.into_iter().chain(attributes).collect(),
        }
    }

    /// Return a reference to the `identitfier` field.
    pub fn identifier(&self) -> &Identity {
        &self.identifier
    }

    /// Return a reference to the `attributes` field.
    pub fn attributes(&self) -> &BTreeMap<Key, Value> {
        &self.attributes
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

impl From<u64> for Subject {
    fn from(identifier: u64) -> Self {
        Self {
            identifier: format!("{:x}", identifier),
            attributes: BTreeMap::default(),
        }
    }
}

impl From<IdentityIdentifier> for Subject {
    fn from(identity: IdentityIdentifier) -> Self {
        Self {
            identifier: identity.to_string(),
            attributes: BTreeMap::default(),
        }
    }
}

impl Extend<Attribute> for Subject {
    fn extend<A>(&mut self, attributes: A)
    where
        A: IntoIterator<Item = Attribute>,
    {
        self.attributes.extend(attributes);
    }
}

/// An ABAC `Resource` entity.
///
/// `Resource` maps to the given resource being placed under access
/// control such as a file or network path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Resource {
    path: String,
    attributes: BTreeMap<Key, Value>,
}

impl Resource {
    /// Create a new `Resource` with the given attributes.
    pub fn with_attributes<A>(self, attributes: A) -> Self
    where
        A: IntoIterator<Item = Attribute> + Send + 'static,
    {
        Self {
            path: self.path,
            attributes: self.attributes.into_iter().chain(attributes).collect(),
        }
    }

    /// Return a reference to the `path` field.
    pub fn path(&self) -> &String {
        &self.path
    }

    /// Return a reference to the `attributes` field.
    pub fn attributes(&self) -> &BTreeMap<Key, Value> {
        &self.attributes
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl From<&str> for Resource {
    fn from(path: &str) -> Self {
        Self {
            path: path.to_string(),
            attributes: BTreeMap::default(),
        }
    }
}

impl Extend<Attribute> for Resource {
    fn extend<A>(&mut self, attributes: A)
    where
        A: IntoIterator<Item = Attribute>,
    {
        self.attributes.extend(attributes);
    }
}

/// An ABAC `Action` entity.
///
/// `Action` corresponds to the action the requesting `Subject` wants
/// to perform on a `Resource`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Action {
    method: String,
    attributes: BTreeMap<Key, Value>,
}

impl Action {
    /// Create a new `Action` with the given attributes.
    pub fn with_attributes<A>(self, attributes: A) -> Self
    where
        A: IntoIterator<Item = Attribute> + Send + 'static,
    {
        Self {
            method: self.method,
            attributes: self.attributes.into_iter().chain(attributes).collect(),
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.method)
    }
}

impl From<&str> for Action {
    fn from(s: &str) -> Self {
        Self {
            method: s.to_string(),
            attributes: BTreeMap::default(),
        }
    }
}

impl From<Method> for Action {
    fn from(method: Method) -> Self {
        Self {
            method: method.into(),
            attributes: BTreeMap::default(),
        }
    }
}

impl Extend<Attribute> for Action {
    fn extend<A>(&mut self, attributes: A)
    where
        A: IntoIterator<Item = Attribute>,
    {
        self.attributes.extend(attributes);
    }
}

/// HTTP verbs
///
/// TODO if we can move ockam_api::Method to ockam or ockam_core we
///      should be able to just use that instead.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Method {
    /// HTTP GET
    Get,
    /// HTTP POST
    Post,
    /// HTTP PUT
    Put,
    /// HTTP DELETE
    Delete,
    /// HTTP PATCH
    Patch,
}

impl From<Method> for String {
    fn from(method: Method) -> Self {
        match method {
            Method::Get => "GET".to_string(),
            Method::Post => "POST".to_string(),
            Method::Put => "PUT".to_string(),
            Method::Delete => "DELETE".to_string(),
            Method::Patch => "PATCH".to_string(),
        }
    }
}

/// A set of ABAC `Attribute`s
pub type Attributes = BTreeMap<Key, Value>;

/// An ABAC `Attribute`
///
/// ABAC attributes are tuples consisting of a string representing the
/// attribute name and the [`Value`] of the attribute.
pub type Attribute = (Key, Value);

/// A `Key` for an attribute `Value` in a set of `Attributes`
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Key(String);

impl From<&str> for Key {
    fn from(s: &str) -> Self {
        Key(s.to_string())
    }
}

impl From<&Key> for String {
    fn from(key: &Key) -> Self {
        key.0.clone()
    }
}

impl core::ops::Deref for Key {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Primitive value types used to construct ABAC attributes and
/// conditionals.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
