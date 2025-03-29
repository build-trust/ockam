use crate::database::MigrationFailure;
use crate::storage::database::migrations::migration_support::migrator::Version;
use core::fmt::{Display, Formatter};
use serde::Serialize;

/// This enum models the state of a database with respect to migrations
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum MigrationStatus {
    /// The database is up to date with the latest version.
    UpToDate(Version),
    /// The database needs to be updated.
    Todo(Option<Version>, Version),
    /// A migration was attempted but failed
    Failed(Version, MigrationFailure),
}

impl Display for MigrationStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            MigrationStatus::UpToDate(version) => {
                write!(f, "✅ The database is up to date (version: {})", version)?
            }
            MigrationStatus::Todo(current_version, next_version) => write!(
                f,
                "⚙️ The database needs to be updated ({}next version: {})",
                current_version
                    .map(|v| format!("current version: {v}, "))
                    .unwrap_or("".to_string()),
                next_version
            )?,
            MigrationStatus::Failed(version, reason) => write!(
                f,
                "❌ The database failed to be updated at version: {}.\nReason: {reason}",
                version
            )?,
        };
        Ok(())
    }
}

impl MigrationStatus {
    /// Return true if the database is up to date
    pub fn up_to_date(&self) -> bool {
        matches!(self, MigrationStatus::UpToDate(_))
    }
}
