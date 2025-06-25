use crate::database::migrations::migration_set::MigrationSet;
use crate::database::Migrator;
use crate::migrate;
use ockam_core::Result;

/// This struct defines the migration to apply to the persistent database
#[derive(Default)]
pub struct ApplicationMigrationSet;

impl ApplicationMigrationSet {
    /// Create a new migration set
    pub fn new() -> Self {
        Self {}
    }
}

impl MigrationSet for ApplicationMigrationSet {
    fn create_migrator(&self) -> Result<Migrator> {
        migrate!("./src/storage/database/migrations/application_migrations/sql/sqlite")
    }
}

#[cfg(test)]
mod tests {
    use crate::database::application_migration_set::ApplicationMigrationSet;
    use crate::database::{DatabaseConfiguration, MigrationSet, SqlxDatabase};
    use ockam_core::Result;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();

        let db = SqlxDatabase::create_no_migration(&DatabaseConfiguration::sqlite(db_file.path())?)
            .await?;

        ApplicationMigrationSet::new()
            .create_migrator()?
            .migrate(&db.pool)
            .await?;

        Ok(())
    }
}
