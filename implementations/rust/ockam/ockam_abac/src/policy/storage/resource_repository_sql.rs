use core::str::FromStr;
use sqlx::*;
use tracing::debug;

use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};

use crate::{Resource, ResourceName, ResourceType, ResourcesRepository};

#[derive(Clone)]
pub struct ResourcesSqlxDatabase {
    database: SqlxDatabase,
}

impl ResourcesSqlxDatabase {
    /// Create a new database for resources
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for resources");
        Self { database }
    }

    /// Create a new in-memory database for resources
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("resources").await?))
    }
}

#[async_trait]
impl ResourcesRepository for ResourcesSqlxDatabase {
    async fn store_resource(&self, resource: &Resource) -> Result<()> {
        let query = query(
            r#"INSERT OR REPLACE INTO resource
            VALUES (?, ?, ?)"#,
        )
        .bind(resource.resource_name.to_sql())
        .bind(resource.resource_type.to_sql())
        .bind(self.database.node_name()?.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_resource(&self, resource_name: &ResourceName) -> Result<Option<Resource>> {
        let query = query_as(
            r#"SELECT resource_name, resource_type
            FROM resource
            WHERE node_name=$1 and resource_name=$2"#,
        )
        .bind(self.database.node_name()?.to_sql())
        .bind(resource_name.to_sql());
        let row: Option<ResourceRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.try_into()).transpose()?)
    }

    async fn delete_resource(&self, resource_name: &ResourceName) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        let query = query(
            r#"DELETE FROM resource
            WHERE node_name=? and resource_name=?"#,
        )
        .bind(self.database.node_name()?.to_sql())
        .bind(resource_name.to_sql());
        query.execute(&mut *transaction).await.void()?;

        let query = sqlx::query(
            r#"DELETE FROM resource_policy
            WHERE node_name=? and resource_name=?"#,
        )
        .bind(self.database.node_name()?.to_sql())
        .bind(resource_name.to_sql());
        query.execute(&mut *transaction).await.void()?;

        transaction.commit().await.void()
    }
}

/// Low-level representation of a row in the resource_type_policy table
#[derive(FromRow)]
#[allow(dead_code)]
struct ResourceRow {
    resource_name: String,
    resource_type: String,
}

impl ResourceRow {
    fn resource_type(&self) -> Result<ResourceType> {
        Ok(ResourceType::from_str(&self.resource_type)?)
    }
}

impl TryFrom<ResourceRow> for Resource {
    type Error = ockam_core::Error;

    fn try_from(row: ResourceRow) -> Result<Self, Self::Error> {
        Ok(Resource {
            resource_name: ResourceName::from(row.resource_name.clone()),
            resource_type: row.resource_type()?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ockam_core::compat::rand::random_string;
    use ockam_core::compat::sync::Arc;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let repository = create_repository().await?;

        // create mapping between resource and resource type
        let rt = ResourceType::TcpOutlet;
        let rn1 = ResourceName::new(&random_string());
        let r1 = Resource::new(rn1.clone(), rt.clone());
        repository.store_resource(&r1).await?;
        assert_eq!(repository.get_resource(&rn1).await?.unwrap(), r1);

        // create another entry for a new resource name
        let rn2 = ResourceName::new(&random_string());
        let r2 = Resource::new(rn2.clone(), rt.clone());
        repository.store_resource(&r2).await?;

        // we can delete a given entry
        repository.delete_resource(&rn1).await?;
        assert!(repository.get_resource(&rn1).await?.is_none());

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn ResourcesRepository>> {
        Ok(Arc::new(ResourcesSqlxDatabase::create().await?))
    }
}
