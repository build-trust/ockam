use sqlx::*;
use tracing::debug;

use ockam_core::async_trait;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, SqlxType, ToSqlxType, ToVoid};

use crate::{Action, Expr, PoliciesRepository, Policy, Resource};

#[derive(Clone)]
pub struct PolicySqlxDatabase {
    database: SqlxDatabase,
}

impl PolicySqlxDatabase {
    /// Create a new database for policies keys
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for policies");
        Self { database }
    }

    /// Create a new in-memory database for policies
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("policies").await?))
    }
}

#[async_trait]
impl PoliciesRepository for PolicySqlxDatabase {
    async fn get_policy(&self, resource: &Resource, action: &Action) -> Result<Option<Policy>> {
        let query = query_as(
            "SELECT resource, action, expression FROM policy WHERE resource=$1 and action=$2",
        )
        .bind(resource.to_sql())
        .bind(action.to_sql());
        let row: Option<PolicyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.policy()).transpose()?)
    }

    async fn set_policy(
        &self,
        resource: &Resource,
        action: &Action,
        policy: &Policy,
    ) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO policy VALUES (?, ?, ?)")
            .bind(resource.to_sql())
            .bind(action.to_sql())
            .bind(minicbor::to_vec(policy.expression())?.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn delete_policy(&self, resource: &Resource, action: &Action) -> Result<()> {
        let query = query("DELETE FROM policy WHERE resource = ? and action = ?")
            .bind(resource.to_sql())
            .bind(action.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_policies_by_resource(&self, resource: &Resource) -> Result<Vec<(Action, Policy)>> {
        let query = query_as("SELECT resource, action, expression FROM policy where resource = $1")
            .bind(resource.to_sql());
        let row: Vec<PolicyRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        row.into_iter()
            .map(|r| r.policy().map(|e| (r.action(), e)))
            .collect::<Result<Vec<(Action, Policy)>>>()
    }
}

// Database serialization / deserialization

impl ToSqlxType for Resource {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Text(self.as_str().to_string())
    }
}

impl ToSqlxType for Action {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Text(self.as_str().to_string())
    }
}

/// Low-level representation of a row in the policies table
#[derive(FromRow)]
pub(crate) struct PolicyRow {
    resource: String,
    action: String,
    expression: Vec<u8>,
}

impl PolicyRow {
    #[allow(dead_code)]
    pub(crate) fn resource(&self) -> Resource {
        Resource::from(self.resource.clone())
    }

    pub(crate) fn action(&self) -> Action {
        Action::from(self.action.clone())
    }

    pub(crate) fn expression(&self) -> Result<Expr> {
        Ok(minicbor::decode(self.expression.as_slice())?)
    }

    pub(crate) fn policy(&self) -> Result<Policy> {
        Ok(Policy::new(self.expression()?))
    }
}

#[cfg(test)]
mod test {
    use crate::expr::*;

    use super::*;

    use ockam_core::compat::sync::Arc;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let repository = create_repository().await?;

        // a policy can be associated to a resource and an action
        let r = Resource::from("outlet");
        let a = Action::from("create");
        let p = Policy::new(eq([ident("name"), str("me")]));
        repository.set_policy(&r, &a, &p).await?;
        assert_eq!(repository.get_policy(&r, &a).await?.unwrap(), p);

        // we can retrieve all the policies associated to a given resource
        let policies = repository.get_policies_by_resource(&r).await?;
        assert_eq!(policies.len(), 1);

        let a = Action::from("delete");
        repository.set_policy(&r, &a, &p).await?;
        let policies = repository.get_policies_by_resource(&r).await?;
        assert_eq!(policies.len(), 2);

        // we can delete a given policy
        // here we delete the policy for outlet/delete
        repository.delete_policy(&r, &a).await?;
        let policies = repository.get_policies_by_resource(&r).await?;
        assert_eq!(policies.len(), 1);
        assert_eq!(policies.first().unwrap().0, Action::from("create"));

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn PoliciesRepository>> {
        Ok(Arc::new(PolicySqlxDatabase::create().await?))
    }
}
