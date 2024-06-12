use core::fmt::Debug;
use sqlx::AnyConnection;

use ockam_core::{async_trait, Result};

/// Individual rust migration
#[async_trait]
pub trait RustMigration: Debug + Send + Sync {
    /// Name of the migration used to track which one was already applied
    fn name(&self) -> &str;

    /// Version if format "yyyymmddnumber"
    fn version(&self) -> i64;

    /// Execute the migration
    async fn migrate(&self, connection: &mut AnyConnection) -> Result<bool>;
}
