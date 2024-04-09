pub use variables::Variables;
pub use version::Version;
#[cfg(test)]
pub use version::VersionValue;

pub(crate) mod building_blocks;
pub mod config;
pub(crate) mod resource;
pub mod variables;
pub mod version;
