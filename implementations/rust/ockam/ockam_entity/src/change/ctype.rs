use serde::{Deserialize, Serialize};

mod create_key;
pub use create_key::*;
mod rotate_key;
pub use rotate_key::*;

/// Possible types of [`crate::Profile`] changes
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ProfileChangeType {
    /// Create key
    CreateKey(CreateKeyChange),
    /// Rotate key
    RotateKey(RotateKeyChange),
}
