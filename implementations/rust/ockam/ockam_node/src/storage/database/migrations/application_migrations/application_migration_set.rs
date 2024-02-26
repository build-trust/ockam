use crate::database::migrations::migration_set::MigrationSet;
use crate::database::Migrator;
use crate::migrate;
use ockam_core::Result;

/// This struct defines the migration to apply to the persistent database
pub struct ApplicationMigrationSet;

impl MigrationSet for ApplicationMigrationSet {
    fn create_migrator(&self) -> Result<Migrator> {
        migrate!("./src/storage/database/migrations/application_migrations/sql")
    }
}

#[cfg(test)]
mod tests {
    use crate::database::application_migration_set::ApplicationMigrationSet;
    use crate::database::{MigrationSet, SqlxDatabase};
    use ockam_core::Result;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();

        let db = SqlxDatabase::create_no_migration(db_file.path()).await?;

        ApplicationMigrationSet
            .create_migrator()?
            .migrate(&db.pool)
            .await?;

        Ok(())
    }
}
