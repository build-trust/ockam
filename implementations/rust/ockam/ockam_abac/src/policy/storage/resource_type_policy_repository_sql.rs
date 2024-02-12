use core::str::FromStr;
use sqlx::*;
use tracing::debug;

use ockam_core::async_trait;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, SqlxType, ToSqlxType, ToVoid};

use crate::policy::ResourceTypePolicy;
use crate::{Action, Expr, ResourceType, ResourceTypePoliciesRepository};

#[derive(Clone)]
pub struct ResourceTypePolicySqlxDatabase {
    database: SqlxDatabase,
}

impl ResourceTypePolicySqlxDatabase {
    /// Create a new database for resource type policies
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for resource type policies");
        Self { database }
    }

    /// Create a new in-memory database for policies
    pub async fn create() -> Result<Self> {
        Ok(Self::new(
            SqlxDatabase::in_memory("resource_type_policies").await?,
        ))
    }
}

#[async_trait]
impl ResourceTypePoliciesRepository for ResourceTypePolicySqlxDatabase {
    async fn store_policy(
        &self,
        resource_type: &ResourceType,
        action: &Action,
        expression: &Expr,
    ) -> Result<()> {
        let query = query(
            r#"INSERT OR REPLACE INTO
            resource_type_policy VALUES (?, ?, ?, ?)"#,
        )
        .bind(resource_type.to_sql())
        .bind(action.to_sql())
        .bind(expression.to_string().to_sql())
        .bind(self.database.node_name()?.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_policy(
        &self,
        resource_type: &ResourceType,
        action: &Action,
    ) -> Result<Option<ResourceTypePolicy>> {
        let query = query_as(
            r#"SELECT resource_type, action, expression
            FROM resource_type_policy
            WHERE node_name=$1 and resource_type=$2 and action=$3"#,
        )
        .bind(self.database.node_name()?.to_sql())
        .bind(resource_type.to_sql())
        .bind(action.to_sql());
        let row: Option<PolicyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.try_into()).transpose()?)
    }

    async fn get_policies(&self) -> Result<Vec<ResourceTypePolicy>> {
        let query = query_as(
            r#"SELECT resource_type, action, expression
            FROM resource_type_policy where node_name=$1"#,
        )
        .bind(self.database.node_name()?.to_sql());
        let row: Vec<PolicyRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        row.into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<ResourceTypePolicy>>>()
    }

    async fn get_policies_by_resource_type(
        &self,
        resource_type: &ResourceType,
    ) -> Result<Vec<ResourceTypePolicy>> {
        let query = query_as(
            r#"SELECT resource_type, action, expression
            FROM resource_type_policy where node_name=$1 and resource_type=$2"#,
        )
        .bind(self.database.node_name()?.to_sql())
        .bind(resource_type.to_sql());
        let row: Vec<PolicyRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        row.into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<ResourceTypePolicy>>>()
    }

    async fn delete_policy(&self, resource_type: &ResourceType, action: &Action) -> Result<()> {
        let query = query(
            r#"DELETE FROM resource_type_policy
            WHERE node_name=? and resource_type=? and action=?"#,
        )
        .bind(self.database.node_name()?.to_sql())
        .bind(resource_type.to_sql())
        .bind(action.to_sql());
        query.execute(&*self.database.pool).await.void()
    }
}

/// Low-level representation of a row in the resource_type_policy table
#[derive(FromRow)]
struct PolicyRow {
    resource_type: String,
    action: String,
    expression: String,
}

impl PolicyRow {
    fn resource_type(&self) -> Result<ResourceType> {
        Ok(ResourceType::from_str(&self.resource_type)?)
    }

    fn action(&self) -> Result<Action> {
        Ok(Action::from_str(&self.action)?)
    }

    fn expression(&self) -> Result<Expr> {
        Ok(Expr::try_from(self.expression.as_str())?)
    }
}

impl TryFrom<PolicyRow> for ResourceTypePolicy {
    type Error = ockam_core::Error;

    fn try_from(row: PolicyRow) -> Result<Self, Self::Error> {
        Ok(ResourceTypePolicy::new(
            row.resource_type()?,
            row.action()?,
            row.expression()?,
        ))
    }
}

// Database serialization / deserialization

impl ToSqlxType for ResourceType {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Text(self.to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::expr::*;
    use ockam_core::compat::sync::Arc;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let repository = create_repository().await?;

        // a policy can be associated to a resource and an action
        let r = ResourceType::TcpOutlet;
        let a = Action::HandleMessage;
        let e = eq([ident("name"), str("me")]);
        repository.store_policy(&r, &a, &e).await?;
        let expected = ResourceTypePolicy::new(r.clone(), a.clone(), e.clone());
        assert_eq!(repository.get_policy(&r, &a).await?.unwrap(), expected);

        // we can retrieve the policies associated to a given resource
        let policies = repository.get_policies_by_resource_type(&r).await?;
        assert_eq!(policies.len(), 1);

        let r = ResourceType::TcpInlet;
        repository.store_policy(&r, &a, &e).await?;
        let policies = repository.get_policies_by_resource_type(&r).await?;
        assert_eq!(policies.len(), 1);

        // we can retrieve all the policies
        let policies = repository.get_policies().await?;
        assert_eq!(policies.len(), 2);

        // we can delete a given policy
        // here we delete the policy for tcp-outlet/handle_message
        repository.delete_policy(&r, &a).await?;
        let policies = repository.get_policies_by_resource_type(&r).await?;
        assert_eq!(policies.len(), 0);

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn ResourceTypePoliciesRepository>> {
        Ok(Arc::new(ResourceTypePolicySqlxDatabase::create().await?))
    }
}
