use core::str::FromStr;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::*;
use sqlx_core::any::AnyArgumentBuffer;
use std::sync::Arc;
use tracing::debug;

use crate::{Resource, ResourceName, ResourceType, ResourcesRepository};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToVoid};

#[derive(Clone)]
pub struct ResourcesSqlxDatabase {
    database: SqlxDatabase,
    node_name: String,
}

impl ResourcesSqlxDatabase {
    /// Create a new database for resources
    pub fn new(database: SqlxDatabase, node_name: &str) -> Self {
        debug!("create a repository for resources");
        Self {
            database,
            node_name: node_name.to_string(),
        }
    }

    /// Create a repository
    pub fn make_repository(
        database: SqlxDatabase,
        node_name: &str,
    ) -> Arc<dyn ResourcesRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database, node_name)))
        } else {
            Arc::new(Self::new(database, node_name))
        }
    }

    /// Create a new in-memory database for resources
    pub async fn create() -> Result<Self> {
        Ok(Self::new(
            SqlxDatabase::in_memory("resources").await?,
            "default",
        ))
    }
}

#[async_trait]
impl ResourcesRepository for ResourcesSqlxDatabase {
    async fn store_resource(&self, resource: &Resource) -> Result<()> {
        let query = query(
            r#"
            INSERT INTO resource (resource_name, resource_type, node_name, tenant_id)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (resource_name, node_name)
            DO UPDATE SET resource_type = $2"#,
        )
        .bind(&resource.resource_name)
        .bind(&resource.resource_type)
        .bind(&self.node_name)
        .bind(self.database.tenant_id());
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_resource(&self, resource_name: &ResourceName) -> Result<Option<Resource>> {
        let query = query_as(
            r#"SELECT resource_name, resource_type
            FROM resource
            WHERE node_name = $1 and resource_name = $2"#,
        )
        .bind(&self.node_name)
        .bind(resource_name);
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
            WHERE node_name = $1 and resource_name = $2"#,
        )
        .bind(&self.node_name)
        .bind(resource_name);
        query.execute(&mut *transaction).await.void()?;

        let query = sqlx::query(
            r#"DELETE FROM resource_policy
            WHERE node_name = $1 and resource_name = $2"#,
        )
        .bind(&self.node_name)
        .bind(resource_name);
        query.execute(&mut *transaction).await.void()?;

        transaction.commit().await.void()
    }
}

// Database serialization / deserialization

impl Type<Any> for ResourceName {
    fn type_info() -> <Any as Database>::TypeInfo {
        <String as Type<Any>>::type_info()
    }
}

impl sqlx::Encode<'_, Any> for ResourceName {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as sqlx::Encode<'_, Any>>::encode_by_ref(&self.to_string(), buf)
    }
}

/// Low-level representation of a row in the resource_type_policy table
#[derive(FromRow)]
#[allow(dead_code)]
struct ResourceRow {
    resource_name: String,
    resource_type: Option<String>,
}

impl ResourceRow {
    fn resource_type(&self) -> Result<Option<ResourceType>> {
        if let Some(resource_type) = &self.resource_type {
            Ok(Some(ResourceType::from_str(resource_type.as_str())?))
        } else {
            Ok(None)
        }
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
        let r1 = Resource::new(rn1.clone(), Some(rt.clone()));
        repository.store_resource(&r1).await?;
        assert_eq!(repository.get_resource(&rn1).await?.unwrap(), r1);

        // create another entry for a new resource name
        let rn2 = ResourceName::new(&random_string());
        let r2 = Resource::new(rn2.clone(), Some(rt.clone()));
        repository.store_resource(&r2).await?;

        // create another entry with empty type
        let rn3 = ResourceName::new(&random_string());
        let r3 = Resource::new(rn3.clone(), None);
        repository.store_resource(&r3).await?;

        let rns2 = repository.get_resource(&rn2).await?.unwrap();
        let rns3 = repository.get_resource(&rn3).await?.unwrap();
        assert_eq!(rns2, r2);
        assert_eq!(rns3, r3);

        // duplicate resource name
        let rn4 = ResourceName::new(&random_string());
        let r4 = Resource::new(rn4.clone(), None);
        let r5 = Resource::new(rn4.clone(), Some(ResourceType::Echoer));
        repository.store_resource(&r4).await?;
        repository.store_resource(&r5).await?;

        let rns5 = repository.get_resource(&rn4).await?.unwrap();
        assert_eq!(rns5, r5);

        // we can delete a given entry
        repository.delete_resource(&rn1).await?;
        assert!(repository.get_resource(&rn1).await?.is_none());

        Ok(())
    }

    // HELPERS
    async fn create_repository() -> Result<Arc<dyn ResourcesRepository>> {
        Ok(Arc::new(ResourcesSqlxDatabase::create().await?))
    }
}
