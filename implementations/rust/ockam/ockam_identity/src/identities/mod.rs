#[allow(clippy::module_inception)]
mod identities;
mod identities_builder;
mod identities_creation;
mod identities_vault;
mod identity_keys;

/// Identities storage functions
pub mod storage;

pub use identities::*;
pub use identities_builder::*;
pub use identities_creation::*;
pub use identities_vault::*;
pub use identity_keys::*;
pub use storage::*;

#[cfg(test)]
mod tests;
