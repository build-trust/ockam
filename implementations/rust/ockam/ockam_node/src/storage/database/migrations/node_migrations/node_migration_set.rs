use crate::database::migrations::migration_20240111100001_add_authority_tables::AuthorityAttributes;
use crate::database::migrations::migration_20240111100002_delete_trust_context::PolicyTrustContextId;
use crate::database::migrations::migration_20240212100000_split_policies::SplitPolicies;
use crate::database::migrations::node_migrations::migration_20231231100000_node_name_identity_attributes::NodeNameIdentityAttributes;
use ockam_core::Result;

use crate::database::migrations::migration_set::MigrationSet;
use crate::database::migrations::{Migrator, RustMigration};
use crate::migrate;

/// This struct defines the migration to apply to the nodes database
pub struct NodeMigrationSet;

impl MigrationSet for NodeMigrationSet {
    fn create_migrator(&self) -> Result<Migrator> {
        let rust_migrations: Vec<Box<dyn RustMigration>> = vec![
            Box::new(NodeNameIdentityAttributes),
            Box::new(AuthorityAttributes),
            Box::new(PolicyTrustContextId),
            Box::new(SplitPolicies),
        ];
        let mut migrator = migrate!("./src/storage/database/migrations/node_migrations/sql")?;
        migrator.set_rust_migrations(rust_migrations)?;

        Ok(migrator)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::migrations::node_migration_set::NodeMigrationSet;
    use crate::database::{MigrationSet, SqlxDatabase};
    use ockam_core::Result;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();

        let db = SqlxDatabase::create_no_migration(db_file.path()).await?;

        NodeMigrationSet
            .create_migrator()?
            .migrate(&db.pool)
            .await?;

        Ok(())
    }
}
