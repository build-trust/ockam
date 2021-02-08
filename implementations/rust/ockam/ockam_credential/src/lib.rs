//! Ockam Credential implements the structures, traits, and protocols
//! for creating, issuing, and verifying ockam credentials.
//!
//! Ockam credentials are used for authentication and authorization among Ockam compatible connections.
#![no_std]
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "heapless")]
mod structs {
    pub use core::fmt::{self, Debug, Display};
    use heapless::{consts::*, String, Vec};
    pub type Buffer<T> = Vec<T, U32>;
    pub type ByteString = String<U32>;
}

#[cfg(not(feature = "heapless"))]
mod structs {
    pub use alloc::fmt::{self, Debug, Display};
    use alloc::{string::String, vec::Vec};
    pub type Buffer<T> = Vec<T>;
    pub type ByteString = String;
}

/// The error module
mod error;
/// Helper methods for serializing and deserializing
mod serdes;
pub use error::CredentialError;
/// The attribute types
mod attribute_type;
pub use attribute_type::AttributeType;
/// The attribute struct used in schemas
mod attribute;
pub use attribute::Attribute;
/// Schema used by credentials
mod schema;
pub use schema::Schema;


#[cfg(test)]
mod tests {
    use crate::{Schema, Attribute, AttributeType};
    use std::string::String;
    use std::vec::Vec;

    fn create_test_schema() -> Schema {
        let attribute = Attribute {
            label: String::from("test_attr"),
            description: String::from("test attribute"),
            attribute_type: AttributeType::Utf8String
        };

        let mut attributes = Vec::new();

        attributes.push(attribute);

        Schema {
            id: String::from("test_id"),
            label: String::from("test_label"),
            description: String::from("test_desc"),
            attributes
        }
    }

    #[test]
    fn test_schema_creation() {
        let _schema = create_test_schema();
    }

    #[test]
    fn test_schema_serialization() {
        let mut schema = create_test_schema();

        if let Ok(serialized) = serde_json::to_string(&schema) {
            assert!(serialized.contains("test_id"));
            assert!(serialized.contains("test_label"));
            assert!(serialized.contains("test_desc"));
            assert!(serialized.contains("test_attr"));
            assert!(serialized.contains("test attribute"));

            if let Ok(mut rehydrated) = serde_json::from_str::<Schema>(&serialized) {
                assert_eq!(rehydrated.id, schema.id);
                assert_eq!(rehydrated.label, schema.label);
                assert_eq!(rehydrated.description, schema.description);
                assert_eq!(rehydrated.attributes.len(), schema.attributes.len());

                if let Some(schema_attr) = schema.attributes.pop() {
                    if let Some(rehydrated_attr) = rehydrated.attributes.pop() {
                        assert_eq!(schema_attr.attribute_type, rehydrated_attr.attribute_type);
                        assert_eq!(schema_attr.label, rehydrated_attr.label);
                        assert_eq!(schema_attr.description, rehydrated_attr.description);
                    } else {
                        panic!("Missing rehydrated attribute")
                    }
                } else {
                    panic!("Missing Schema attribute")
                }
            }
        } else {
            panic!("Couldn't serialize Schema")
        }
    }
}
