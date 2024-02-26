use crate::database::Migrator;
use ockam_core::Result;

/// This trait runs migrations on a given database
pub trait MigrationSet {
    /// Migrate the content of a database: schema and or data
    fn create_migrator(&self) -> Result<Migrator>;
}
