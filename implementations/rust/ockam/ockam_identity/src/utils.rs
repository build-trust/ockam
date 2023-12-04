use minicbor::bytes::ByteVec;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

use crate::models::{Attributes, CredentialSchemaIdentifier, TimestampInSeconds};

/// Create a new timestamp using the system time
pub fn now() -> Result<TimestampInSeconds> {
    let utc_seconds = ockam_core::compat::time::now()?;
    Ok(TimestampInSeconds(utc_seconds))
}

/// Convenient builder for the [`Attributes`] struct
pub struct AttributesBuilder {
    schema_id: CredentialSchemaIdentifier,
    map: BTreeMap<ByteVec, ByteVec>,
}

impl AttributesBuilder {
    /// Create an empty [`Attributes`] struct with a given [`CredentialSchemaIdentifier`]
    pub fn with_schema(schema_id: CredentialSchemaIdentifier) -> Self {
        Self {
            schema_id,
            map: Default::default(),
        }
    }

    /// Add an attributes to the [`Attributes`]
    pub fn with_attribute(mut self, key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) -> Self {
        self.map.insert(key.into().into(), value.into().into());

        self
    }

    /// Build the corresponding [`Attributes`]
    pub fn build(self) -> Attributes {
        Attributes {
            schema: self.schema_id,
            map: self.map,
        }
    }
}
