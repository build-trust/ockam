use core::str::FromStr;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::any::AnyArgumentBuffer;
use sqlx::*;
use std::sync::Arc;
use tracing::debug;

use crate::policy::ResourceTypePolicy;
use crate::{Action, Expr, ResourceType, ResourceTypePoliciesRepository};
use ockam_core::async_trait;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToVoid};

#[derive(Clone)]
pub struct ResourceTypePolicySqlxDatabase {
    database: SqlxDatabase,
    node_name: String,
}

impl ResourceTypePolicySqlxDatabase {
    /// Create a new database for resource type policies
    pub fn new(database: SqlxDatabase, node_name: &str) -> Self {
        debug!("create a repository for resource type policies");
        Self {
            database,
            node_name: node_name.to_string(),
        }
    }

    /// Create a repository
    pub fn make_repository(
        database: SqlxDatabase,
        node_name: &str,
    ) -> Arc<dyn ResourceTypePoliciesRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database, node_name)))
        } else {
            Arc::new(Self::new(database, node_name))
        }
    }

    /// Create a new in-memory database for policies
    pub async fn create() -> Result<Self> {
        Ok(Self::new(
            SqlxDatabase::in_memory("resource_type_policies").await?,
            "default",
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
            r#"INSERT INTO
            resource_type_policy (resource_type, action, expression, node_name)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (node_name, resource_type, action)
            DO UPDATE SET expression = $3"#,
        )
        .bind(resource_type)
        .bind(action)
        .bind(expression)
        .bind(&self.node_name);
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
            WHERE node_name = $1 and resource_type = $2 and action = $3"#,
        )
        .bind(&self.node_name)
        .bind(resource_type)
        .bind(action);
        let row: Option<PolicyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.try_into()).transpose()?)
    }

    async fn get_policies(&self) -> Result<Vec<ResourceTypePolicy>> {
        let query = query_as(
            r#"SELECT resource_type, action, expression
            FROM resource_type_policy where node_name = $1"#,
        )
        .bind(&self.node_name);
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
            FROM resource_type_policy where node_name = $1 and resource_type = $2"#,
        )
        .bind(&self.node_name)
        .bind(resource_type);
        let row: Vec<PolicyRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        row.into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<ResourceTypePolicy>>>()
    }

    async fn delete_policy(&self, resource_type: &ResourceType, action: &Action) -> Result<()> {
        let query = query(
            r#"DELETE FROM resource_type_policy
            WHERE node_name = $1 and resource_type = $2 and action = $3"#,
        )
        .bind(&self.node_name)
        .bind(resource_type)
        .bind(action);
        query.execute(&*self.database.pool).await.void()
    }
}

// Database serialization / deserialization

impl Type<Any> for ResourceType {
    fn type_info() -> <Any as Database>::TypeInfo {
        <String as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for ResourceType {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as Encode<'_, Any>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl Type<Any> for Action {
    fn type_info() -> <Any as Database>::TypeInfo {
        <String as Type<Any>>::type_info()
    }
}

impl sqlx::Encode<'_, Any> for Action {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as Encode<'_, Any>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl Type<Any> for Expr {
    fn type_info() -> <Any as Database>::TypeInfo {
        <String as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for Expr {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as Encode<'_, Any>>::encode_by_ref(&self.to_string(), buf)
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
