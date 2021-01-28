use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum ProfileKeyType {
    Root,
    Issuing,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum ProfileKeyPurpose {
    ProfileUpdate,
    IssueCredentials,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct KeyAttributes {
    label: String,
    key_type: ProfileKeyType,
    key_purpose: ProfileKeyPurpose,
}

impl KeyAttributes {
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn key_type(&self) -> ProfileKeyType {
        self.key_type
    }
    pub fn key_purpose(&self) -> ProfileKeyPurpose {
        self.key_purpose
    }
}

impl KeyAttributes {
    pub fn new(label: String, key_type: ProfileKeyType, key_purpose: ProfileKeyPurpose) -> Self {
        KeyAttributes {
            label,
            key_type,
            key_purpose,
        }
    }
}
