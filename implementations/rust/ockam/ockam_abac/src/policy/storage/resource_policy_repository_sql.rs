use core::str::FromStr;
use sqlx::*;
use std::sync::Arc;
use tracing::debug;

use crate::{Action, Expr, ResourceName, ResourcePoliciesRepository, ResourcePolicy};
use ockam_core::async_trait;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToVoid};

#[derive(Clone)]
pub struct ResourcePolicySqlxDatabase {
    database: SqlxDatabase,
    node_name: String,
}

impl ResourcePolicySqlxDatabase {
    /// Create a new database for resource policies
    pub fn new(database: SqlxDatabase, node_name: &str) -> Self {
        debug!("create a repository for resource policies");
        Self {
            database,
            node_name: node_name.to_string(),
        }
    }

    /// Create a repository
    pub fn make_repository(
        database: SqlxDatabase,
        node_name: &str,
    ) -> Arc<dyn ResourcePoliciesRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database, node_name)))
        } else {
            Arc::new(Self::new(database, node_name))
        }
    }

    /// Create a new in-memory database for policies
    pub async fn create() -> Result<Self> {
        Ok(Self::new(
            SqlxDatabase::in_memory("resource_policies").await?,
            "default",
        ))
    }
}

#[async_trait]
impl ResourcePoliciesRepository for ResourcePolicySqlxDatabase {
    async fn store_policy(
        &self,
        resource_name: &ResourceName,
        action: &Action,
        expression: &Expr,
    ) -> Result<()> {
        let query = query(
            r#"INSERT INTO resource_policy (resource_name, action, expression, node_name)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (resource_name, action, node_name)
            DO UPDATE SET expression = $3"#,
        )
        .bind(resource_name)
        .bind(action)
        .bind(expression)
        .bind(&self.node_name);
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_policy(
        &self,
        resource_name: &ResourceName,
        action: &Action,
    ) -> Result<Option<ResourcePolicy>> {
        let query = query_as(
            r#"SELECT resource_name, action, expression
            FROM resource_policy
            WHERE node_name = $1 and resource_name = $2 and action = $3"#,
        )
        .bind(&self.node_name)
        .bind(resource_name)
        .bind(action);
        let row: Option<PolicyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.try_into()).transpose()?)
    }

    async fn get_policies(&self) -> Result<Vec<ResourcePolicy>> {
        let query = query_as(
            r#"SELECT resource_name, action, expression
            FROM resource_policy
            WHERE node_name = $1"#,
        )
        .bind(&self.node_name);
        let row: Vec<PolicyRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        row.into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<ResourcePolicy>>>()
    }

    async fn get_policies_by_resource_name(
        &self,
        resource_name: &ResourceName,
    ) -> Result<Vec<ResourcePolicy>> {
        let query = query_as(
            r#"SELECT resource_name, action, expression
            FROM resource_policy
            WHERE node_name = $1 and resource_name = $2"#,
        )
        .bind(&self.node_name)
        .bind(resource_name);
        let row: Vec<PolicyRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        row.into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<ResourcePolicy>>>()
    }

    async fn delete_policy(&self, resource_name: &ResourceName, action: &Action) -> Result<()> {
        let query = query(
            r#"DELETE FROM resource_policy
            WHERE node_name = $1 and resource_name = $2 and action = $3"#,
        )
        .bind(&self.node_name)
        .bind(resource_name)
        .bind(action);
        query.execute(&*self.database.pool).await.void()
    }
}

/// Low-level representation of a row in the resource_policy table
#[derive(FromRow)]
struct PolicyRow {
    resource_name: String,
    action: String,
    expression: String,
}

impl PolicyRow {
    #[allow(dead_code)]
    fn resource_name(&self) -> ResourceName {
        ResourceName::from(self.resource_name.clone())
    }

    fn action(&self) -> Result<Action> {
        Ok(Action::from_str(&self.action)?)
    }

    fn expression(&self) -> Result<Expr> {
        Ok(Expr::try_from(self.expression.as_str())?)
    }
}

impl TryFrom<PolicyRow> for ResourcePolicy {
    type Error = ockam_core::Error;

    fn try_from(row: PolicyRow) -> Result<Self, Self::Error> {
        Ok(ResourcePolicy::new(
            row.resource_name(),
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
        let repo = resource_policy_repository().await?;

        // a policy can be associated to a resource and an action
        let a = Action::HandleMessage;
        let rn = ResourceName::from("outlet1");
        let e = eq([ident("name"), str("me")]);
        repo.store_policy(&rn, &a, &e).await?;
        let expected = ResourcePolicy::new(rn.clone(), a.clone(), e.clone());
        assert_eq!(repo.get_policy(&rn, &a).await?.unwrap(), expected);

        // we can retrieve the policies associated to a given resource
        let policies = repo.get_policies_by_resource_name(&rn).await?;
        assert_eq!(policies.len(), 1);

        let rn = ResourceName::from("outlet2");
        repo.store_policy(&rn, &a, &e).await?;
        let policies = repo.get_policies_by_resource_name(&rn).await?;
        assert_eq!(policies.len(), 1);

        // we can retrieve all the policies
        let policies = repo.get_policies().await?;
        assert_eq!(policies.len(), 2);

        // we can delete a given policy
        // here we delete the policy for outlet/handle_message
        repo.delete_policy(&rn, &a).await?;
        let policies = repo.get_policies_by_resource_name(&rn).await?;
        assert_eq!(policies.len(), 0);

        Ok(())
    }

    /// HELPERS
    async fn resource_policy_repository() -> Result<Arc<dyn ResourcePoliciesRepository>> {
        Ok(Arc::new(ResourcePolicySqlxDatabase::create().await?))
    }
}
