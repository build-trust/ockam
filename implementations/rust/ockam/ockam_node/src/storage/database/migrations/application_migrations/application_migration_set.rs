use crate::database::migrations::migration_set::MigrationSet;
use crate::database::{DatabaseType, Migrator};
use crate::migrate;
use ockam_core::Result;

/// This struct defines the migration to apply to the persistent database
pub struct ApplicationMigrationSet {
    database_type: DatabaseType,
}

impl ApplicationMigrationSet {
    /// Create a new migration set
    pub fn new(database_type: DatabaseType) -> Self {
        Self { database_type }
    }
}

impl MigrationSet for ApplicationMigrationSet {
    fn create_migrator(&self) -> Result<Migrator> {
        match self.database_type {
            DatabaseType::Sqlite => {
                migrate!("./src/storage/database/migrations/application_migrations/sql/sqlite")
            }
            DatabaseType::Postgres => {
                migrate!("./src/storage/database/migrations/application_migrations/sql/postgres")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::database::application_migration_set::ApplicationMigrationSet;
    use crate::database::{DatabaseConfiguration, DatabaseType, MigrationSet, SqlxDatabase};
    use ockam_core::Result;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();

        let db = SqlxDatabase::create_no_migration(&DatabaseConfiguration::sqlite(db_file.path()))
            .await?;

        ApplicationMigrationSet::new(DatabaseType::Sqlite)
            .create_migrator()?
            .migrate(&db.pool)
            .await?;

        Ok(())
    }
}
