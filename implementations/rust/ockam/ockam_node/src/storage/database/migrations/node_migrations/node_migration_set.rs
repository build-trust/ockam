use crate::database::migrations::sqlite::migration_20231231100000_node_name_identity_attributes::NodeNameIdentityAttributes;
use crate::database::migrations::sqlite::migration_20240111100001_add_authority_tables::AuthorityAttributes;
use crate::database::migrations::sqlite::migration_20240111100002_delete_trust_context::PolicyTrustContextId;
use crate::database::migrations::sqlite::migration_20240212100000_split_policies::SplitPolicies;
use crate::database::migrations::sqlite::migration_20240313100000_remove_orphan_resources::RemoveOrphanResources;
use crate::database::migrations::sqlite::migration_20240503100000_update_policy_expressions::UpdatePolicyExpressions;
use crate::database::DatabaseType;
use ockam_core::Result;

use crate::database::migrations::migration_set::MigrationSet;
use crate::database::migrations::{Migrator, RustMigration};
use crate::database::sqlite::migration_20250114100000_members_authority_id::SetAuthorityId;
use crate::migrate;

/// This struct defines the migration to apply to the nodes database
pub struct NodeMigrationSet {
    database_type: DatabaseType,
}

impl NodeMigrationSet {
    /// Create a new migration set for a node
    pub fn new(database_type: DatabaseType) -> Self {
        Self { database_type }
    }
}

impl MigrationSet for NodeMigrationSet {
    fn create_migrator(&self) -> Result<Migrator> {
        let rust_migrations: Vec<Box<dyn RustMigration>> = match self.database_type {
            DatabaseType::Sqlite => vec![
                Box::new(NodeNameIdentityAttributes),
                Box::new(AuthorityAttributes),
                Box::new(PolicyTrustContextId),
                Box::new(SplitPolicies),
                Box::new(RemoveOrphanResources),
                Box::new(UpdatePolicyExpressions),
                Box::new(SetAuthorityId),
            ],
            DatabaseType::Postgres => vec![],
        };
        let mut migrator = match self.database_type {
            DatabaseType::Sqlite => {
                migrate!("./src/storage/database/migrations/node_migrations/sql/sqlite")?
            }
            DatabaseType::Postgres => {
                migrate!("./src/storage/database/migrations/node_migrations/sql/postgres")?
            }
        };
        migrator.set_rust_migrations(rust_migrations)?;

        Ok(migrator)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::migrations::node_migration_set::NodeMigrationSet;
    use crate::database::{DatabaseConfiguration, DatabaseType, MigrationSet, SqlxDatabase};
    use ockam_core::Result;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();

        let db = SqlxDatabase::create_no_migration(&DatabaseConfiguration::sqlite(db_file.path()))
            .await?;

        NodeMigrationSet::new(DatabaseType::Sqlite)
            .create_migrator()?
            .migrate(&db.pool)
            .await?;

        Ok(())
    }
}
