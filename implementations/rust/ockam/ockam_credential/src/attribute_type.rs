use serde::{Deserialize, Serialize};

/// The data type of an attribute's value.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum AttributeType {
    /// The attribute is a UTF8 encoded string.
    Utf8String,
    /// The attribute is a number, either real or an integer.
    Number,
    /// The value is a byte sequence.
    Blob,
}
