use crate::database::sqlx_migration::{map_migrate_err, SqlxMigration};
use ockam_core::{async_trait, Result};
use sqlx::SqlitePool;

/// This struct defines the migration to apply to the persistent database
pub struct ApplicationMigration;

#[async_trait]
impl SqlxMigration for ApplicationMigration {
    async fn migrate(&self, pool: &SqlitePool) -> Result<()> {
        sqlx::migrate!("./src/storage/database/application_migrations")
            .run(pool)
            .await
            .map_err(map_migrate_err)?;
        Ok(())
    }
}
