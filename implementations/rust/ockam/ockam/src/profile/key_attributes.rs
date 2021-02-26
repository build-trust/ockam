use serde::{Deserialize, Serialize};

/// Attributes that are used to identify key
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct KeyAttributes {
    label: String,
}

impl From<&str> for KeyAttributes {
    fn from(str: &str) -> Self {
        Self::new(str.to_string())
    }
}

impl KeyAttributes {
    /// Human-readable key name
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl KeyAttributes {
    pub fn new(label: String) -> Self {
        KeyAttributes { label }
    }
}
