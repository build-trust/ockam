use std::str::FromStr;

use sqlx::any::AnyRow;
use sqlx::*;

use ockam::identity::Identifier;
use ockam::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_core::async_trait;

use ockam_core::Result;
use ockam_node::database::{Boolean, Nullable};

use crate::cli_state::{NodeInfo, NodesRepository};
use crate::config::lookup::InternetAddress;

#[derive(Clone)]
pub struct NodesSqlxDatabase {
    database: SqlxDatabase,
}

impl NodesSqlxDatabase {
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for nodes");
        Self { database }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("nodes").await?))
    }
}

#[async_trait]
impl NodesRepository for NodesSqlxDatabase {
    async fn store_node(&self, node_info: &NodeInfo) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO node VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")
            .bind(node_info.name())
            .bind(node_info.identifier().to_sql())
            .bind(node_info.verbosity().to_sql())
            .bind(node_info.is_default())
            .bind(node_info.is_authority_node())
            .bind(
                node_info
                    .tcp_listener_address()
                    .as_ref()
                    .map(|a| a.to_string()),
            )
            .bind(node_info.pid().map(|p| p.to_sql()))
            .bind(
                node_info
                    .http_server_address()
                    .as_ref()
                    .map(|a| a.to_string()),
            );
        Ok(query.execute(&*self.database.pool).await.void()?)
    }

    async fn get_nodes(&self) -> Result<Vec<NodeInfo>> {
        let query = query_as("SELECT name, identifier, verbosity, is_default, is_authority, tcp_listener_address, pid, http_server_address FROM node");
        let rows: Vec<NodeRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        rows.iter().map(|r| r.node_info()).collect()
    }

    async fn get_node(&self, node_name: &str) -> Result<Option<NodeInfo>> {
        let query = query_as("SELECT name, identifier, verbosity, is_default, is_authority, tcp_listener_address, pid, http_server_address FROM node WHERE name = ?").bind(node_name);
        let row: Option<NodeRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.node_info()).transpose()
    }

    async fn get_nodes_by_identifier(&self, identifier: &Identifier) -> Result<Vec<NodeInfo>> {
        let query = query_as("SELECT name, identifier, verbosity, is_default, is_authority, tcp_listener_address, pid, http_server_address FROM node WHERE identifier = ?").bind(identifier.to_sql());
        let rows: Vec<NodeRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        rows.iter().map(|r| r.node_info()).collect()
    }

    async fn get_default_node(&self) -> Result<Option<NodeInfo>> {
        let query = query_as("SELECT name, identifier, verbosity, is_default, is_authority, tcp_listener_address, pid, http_server_address FROM node WHERE is_default = ?").bind(true);
        let row: Option<NodeRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.node_info()).transpose()
    }

    async fn is_default_node(&self, node_name: &str) -> Result<bool> {
        let query = query("SELECT is_default FROM node WHERE name = ?").bind(node_name);
        let row: Option<AnyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row
            .map(|r| r.get::<Boolean, usize>(0).to_bool())
            .unwrap_or(false))
    }

    async fn set_default_node(&self, node_name: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        // set the node as the default one
        let query1 = query("UPDATE node SET is_default = ? WHERE name = ?")
            .bind(true)
            .bind(node_name);
        query1.execute(&mut *transaction).await.void()?;

        // set all the others as non-default
        let query2 = query("UPDATE node SET is_default = ? WHERE name <> ?")
            .bind(false)
            .bind(node_name);
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()
    }

    async fn delete_node(&self, node_name: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        let query = query("DELETE FROM node WHERE name=?").bind(node_name);
        query.execute(&mut *transaction).await.void()?;

        let query = sqlx::query("DELETE FROM credential WHERE node_name=?").bind(node_name);
        query.execute(&mut *transaction).await.void()?;

        let query = sqlx::query("DELETE FROM resource WHERE node_name=?").bind(node_name);
        query.execute(&mut *transaction).await.void()?;

        let query = sqlx::query("DELETE FROM resource_policy WHERE node_name=?").bind(node_name);
        query.execute(&mut *transaction).await.void()?;

        let query =
            sqlx::query("DELETE FROM resource_type_policy WHERE node_name=?").bind(node_name);
        query.execute(&mut *transaction).await.void()?;

        let query =
            sqlx::query("DELETE FROM identity_attributes WHERE node_name=?").bind(node_name);
        query.execute(&mut *transaction).await.void()?;

        let query = sqlx::query("DELETE FROM tcp_inlet WHERE node_name=?").bind(node_name);
        query.execute(&mut *transaction).await.void()?;

        let query = sqlx::query("DELETE FROM tcp_outlet_status WHERE node_name=?").bind(node_name);
        query.execute(&mut *transaction).await.void()?;

        let query = sqlx::query("DELETE FROM node_project WHERE node_name=?").bind(node_name);
        query.execute(&mut *transaction).await.void()?;

        transaction.commit().await.void()
    }

    async fn set_tcp_listener_address(
        &self,
        node_name: &str,
        address: &InternetAddress,
    ) -> Result<()> {
        let query = query("UPDATE node SET tcp_listener_address = ? WHERE name = ?")
            .bind(address.to_string())
            .bind(node_name);
        query.execute(&*self.database.pool).await.void()
    }

    async fn set_http_server_address(
        &self,
        node_name: &str,
        address: &InternetAddress,
    ) -> Result<()> {
        let query = query("UPDATE node SET http_server_address = ? WHERE name = ?")
            .bind(address.to_string())
            .bind(node_name);
        query.execute(&*self.database.pool).await.void()
    }

    async fn set_as_authority_node(&self, node_name: &str) -> Result<()> {
        let query = query("UPDATE node SET is_authority = ? WHERE name = ?")
            .bind(true)
            .bind(node_name);
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_tcp_listener_address(&self, node_name: &str) -> Result<Option<InternetAddress>> {
        Ok(self
            .get_node(node_name)
            .await?
            .and_then(|n| n.tcp_listener_address()))
    }

    async fn get_http_server_address(&self, node_name: &str) -> Result<Option<InternetAddress>> {
        Ok(self
            .get_node(node_name)
            .await?
            .and_then(|n| n.http_server_address()))
    }

    async fn set_node_pid(&self, node_name: &str, pid: u32) -> Result<()> {
        let query = query("UPDATE node SET pid = ? WHERE name = ?")
            .bind(pid.to_sql())
            .bind(node_name);
        query.execute(&*self.database.pool).await.void()
    }

    async fn set_no_node_pid(&self, node_name: &str) -> Result<()> {
        let query = query("UPDATE node SET pid = NULL WHERE name = ?").bind(node_name);
        query.execute(&*self.database.pool).await.void()
    }

    async fn set_node_project_name(&self, node_name: &str, project_name: &str) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO node_project VALUES (?1, ?2)")
            .bind(node_name)
            .bind(project_name);
        Ok(query.execute(&*self.database.pool).await.void()?)
    }

    async fn get_node_project_name(&self, node_name: &str) -> Result<Option<String>> {
        let query =
            query("SELECT project_name FROM node_project WHERE node_name = ?").bind(node_name);
        let row: Option<AnyRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        let project_name: Option<String> = row.map(|r| r.get(0));
        Ok(project_name)
    }
}

// Database serialization / deserialization

#[derive(FromRow)]
pub(crate) struct NodeRow {
    name: String,
    identifier: String,
    verbosity: i64,
    is_default: Boolean,
    is_authority: Boolean,
    tcp_listener_address: Nullable<String>,
    pid: Nullable<i64>,
    http_server_address: Nullable<String>,
}

impl NodeRow {
    pub(crate) fn node_info(&self) -> Result<NodeInfo> {
        let tcp_listener_address = self
            .tcp_listener_address
            .to_option()
            .and_then(|a| InternetAddress::new(&a));
        let http_server_address = self
            .http_server_address
            .to_option()
            .and_then(|a| InternetAddress::new(&a));

        Ok(NodeInfo::new(
            self.name.clone(),
            Identifier::from_str(&self.identifier.clone())?,
            self.verbosity as u8,
            self.is_default.to_bool(),
            self.is_authority.to_bool(),
            tcp_listener_address,
            self.pid.to_option().map(|p| p as u32),
            http_server_address,
        ))
    }
}

#[cfg(test)]
mod test {
    use crate::cli_state::NodeInfo;
    use ockam::identity::identities;
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let repository = create_repository().await?;
        let identifier = create_identity().await?;

        // The information about a node can be stored
        let node_info1 = NodeInfo::new(
            "node1".to_string(),
            identifier.clone(),
            0,
            false,
            false,
            InternetAddress::new("127.0.0.1:51591"),
            Some(1234),
            InternetAddress::new("127.0.0.1:51592"),
        );

        repository.store_node(&node_info1).await?;

        // get the node by name
        let result = repository.get_node("node1").await?;
        assert_eq!(result, Some(node_info1.clone()));

        // get the node by identifier
        let result = repository.get_nodes_by_identifier(&identifier).await?;
        assert_eq!(result, vec![node_info1.clone()]);

        // the list of all the nodes can be retrieved
        let node_info2 = NodeInfo::new(
            "node2".to_string(),
            identifier.clone(),
            0,
            false,
            false,
            None,
            Some(5678),
            None,
        );

        repository.store_node(&node_info2).await?;
        let result = repository.get_nodes().await?;
        assert_eq!(result, vec![node_info1.clone(), node_info2.clone()]);

        // a node can be set as the default
        repository.set_default_node("node2").await?;
        let result = repository.get_default_node().await?;
        assert_eq!(result, Some(node_info2.set_as_default()));

        // a node can be deleted
        repository.delete_node("node2").await?;
        let result = repository.get_nodes().await?;
        assert_eq!(result, vec![node_info1.clone()]);

        // in that case there is no more default node
        let result = repository.get_default_node().await?;
        assert!(result.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_an_identity_used_by_two_nodes() -> Result<()> {
        let repository = create_repository().await?;
        let identifier1 = create_identity().await?;
        let identifier2 = create_identity().await?;

        // Create 3 nodes: 2 with the same identifier, 1 with a different identifier
        let node_info1 = create_node("node1", &identifier1);
        repository.store_node(&node_info1).await?;

        let node_info2 = create_node("node2", &identifier1);
        repository.store_node(&node_info2).await?;

        let node_info3 = create_node("node3", &identifier2);
        repository.store_node(&node_info3).await?;

        // get the nodes for identifier1
        let result = repository.get_nodes_by_identifier(&identifier1).await?;
        assert_eq!(result, vec![node_info1.clone(), node_info2.clone()]);
        Ok(())
    }

    #[tokio::test]
    async fn test_node_project() -> Result<()> {
        let repository = create_repository().await?;

        // a node can be associated to a project name
        repository
            .set_node_project_name("node_name", "project1")
            .await?;
        let result = repository.get_node_project_name("node_name").await?;
        assert_eq!(result, Some("project1".into()));

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn NodesRepository>> {
        Ok(Arc::new(NodesSqlxDatabase::create().await?))
    }

    async fn create_identity() -> Result<Identifier> {
        let identities = identities().await?;
        identities.identities_creation().create_identity().await
    }

    fn create_node(node_name: &str, identifier: &Identifier) -> NodeInfo {
        NodeInfo::new(
            node_name.to_string(),
            identifier.clone(),
            0,
            false,
            false,
            InternetAddress::new("127.0.0.1:51591"),
            Some(1234),
            InternetAddress::new("127.0.0.1:51592"),
        )
    }
}
