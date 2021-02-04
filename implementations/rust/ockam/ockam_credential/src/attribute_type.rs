use serde::{Deserialize, Serialize};

/// A Mapper converts an arbitrary value to cryptographic field element
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum AttributeType {
    /// The attribute is a UTF8 encoded string
    Utf8String,
    /// The attribute is a number either real or an integer
    Number,
    /// The value is a byte sequence
    Blob,
}
