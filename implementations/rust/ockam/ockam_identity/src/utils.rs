use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

use crate::models::{Attributes, SchemaId, TimestampInSeconds};
use crate::IdentityError;

/// Create a new timestamp using the system time
#[cfg(feature = "std")]
pub fn now() -> Result<TimestampInSeconds> {
    if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(TimestampInSeconds::new(now.as_secs()))
    } else {
        Err(IdentityError::UnknownTimestamp.into())
    }
}

/// Create a new timestamp using the system time
#[cfg(not(feature = "std"))]
pub fn now() -> Result<TimestampInSeconds> {
    Err(IdentityError::UnknownTimestamp.into())
}

/// Add a number of seconds to the [`TimestampInSeconds`]
pub fn add_seconds(timestamp: &TimestampInSeconds, seconds: u64) -> TimestampInSeconds {
    TimestampInSeconds::new(timestamp.saturating_add(seconds))
}

/// Convenient builder for the [`Attributes`] struct
pub struct AttributesBuilder {
    schema_id: SchemaId,
    map: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl AttributesBuilder {
    /// Create an empty [`Attributes`] struct with a given [`SchemaId`]
    pub fn with_schema(schema_id: SchemaId) -> Self {
        Self {
            schema_id,
            map: Default::default(),
        }
    }

    /// Add an attributes to the [`Attributes`]
    pub fn with_attribute(mut self, key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) -> Self {
        self.map.insert(key.into(), value.into());

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
