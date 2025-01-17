use core::fmt::{Display, Formatter};
use serde::Serialize;

/// This enum models the result of executing one migration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationResult {
    /// The migration was successful
    MigrationSuccess,
    /// The migration had a failure
    MigrationFailure(MigrationFailure),
}

impl MigrationResult {
    /// Create a success result
    pub fn success() -> Self {
        MigrationResult::MigrationSuccess
    }

    /// Create a failure for an incorrect checksum
    pub fn incorrect_checksum(
        description: String,
        sql: String,
        actual_checksum: String,
        expected_checksum: String,
    ) -> Self {
        Self::MigrationFailure(MigrationFailure::IncorrectChecksum(
            description,
            sql,
            actual_checksum,
            expected_checksum,
        ))
    }

    /// Create a failure when a down migration was attempted
    pub fn down_migration() -> Self {
        Self::MigrationFailure(MigrationFailure::DownMigration)
    }

    /// Create a failure when a migration failed for a given version
    pub fn dirty_version() -> Self {
        Self::MigrationFailure(MigrationFailure::DirtyVersion)
    }
}

/// This enum models possible causes for migration failures. Either for a simple migration or several.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum MigrationFailure {
    /// Description, sql, actual checksum, expected checksum
    IncorrectChecksum(String, String, String, String),
    /// A down migration was attempted
    DownMigration,
    /// The previous migration version failed to execute
    DirtyVersion,
}

impl Display for MigrationFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            MigrationFailure::IncorrectChecksum(
                description,
                sql,
                actual_checksum,
                expected_checksum,
            ) => write!(f, "❌ Incorrect checksum for the migration: {description}. Actual checksum: {actual_checksum}, expected checksum: {expected_checksum}.\nSQL statements\n{sql}")?,
            MigrationFailure::DownMigration => write!(f, "A down migration was attempted")?,
            MigrationFailure::DirtyVersion => write!(f, "This migration has already been executed and it failed")?,
        }
        Ok(())
    }
}
