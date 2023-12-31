use std::str::FromStr;

use sqlx::sqlite::SqliteRow;
use sqlx::*;

use ockam::identity::Identifier;
use ockam::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_core::async_trait;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;

use crate::cli_state::NodesRepository;
use crate::config::lookup::InternetAddress;
use crate::NodeInfo;

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
        let query = query("INSERT OR REPLACE INTO node VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .bind(node_info.name().to_sql())
            .bind(node_info.identifier().to_sql())
            .bind(node_info.verbosity().to_sql())
            .bind(node_info.is_default().to_sql())
            .bind(node_info.is_authority_node().to_sql())
            .bind(
                node_info
                    .tcp_listener_address()
                    .as_ref()
                    .map(|a| a.to_string().to_sql()),
            )
            .bind(node_info.pid().map(|p| p.to_sql()));
        Ok(query.execute(&*self.database.pool).await.void()?)
    }

    async fn get_nodes(&self) -> Result<Vec<NodeInfo>> {
        let query = query_as("SELECT name, identifier, verbosity, is_default, is_authority, tcp_listener_address, pid FROM node");
        let rows: Vec<NodeRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        rows.iter().map(|r| r.node_info()).collect()
    }

    async fn get_node(&self, node_name: &str) -> Result<Option<NodeInfo>> {
        let query = query_as("SELECT name, identifier, verbosity, is_default, is_authority, tcp_listener_address, pid FROM node WHERE name = ?").bind(node_name.to_sql());
        let row: Option<NodeRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.node_info()).transpose()
    }

    async fn get_nodes_by_identifier(&self, identifier: &Identifier) -> Result<Vec<NodeInfo>> {
        let query = query_as("SELECT name, identifier, verbosity, is_default, is_authority, tcp_listener_address, pid FROM node WHERE identifier = ?").bind(identifier.to_sql());
        let rows: Vec<NodeRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        rows.iter().map(|r| r.node_info()).collect()
    }

    async fn get_default_node(&self) -> Result<Option<NodeInfo>> {
        let query = query_as("SELECT name, identifier, verbosity, is_default, is_authority, tcp_listener_address, pid FROM node WHERE is_default = ?").bind(true.to_sql());
        let row: Option<NodeRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.node_info()).transpose()
    }

    async fn is_default_node(&self, node_name: &str) -> Result<bool> {
        let query = query("SELECT is_default FROM node WHERE name = ?").bind(node_name.to_sql());
        let row: Option<SqliteRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.get(0)).unwrap_or(false))
    }

    async fn set_default_node(&self, node_name: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        // set the node as the default one
        let query1 = query("UPDATE node SET is_default = ? WHERE name = ?")
            .bind(true.to_sql())
            .bind(node_name.to_sql());
        query1.execute(&mut *transaction).await.void()?;

        // set all the others as non-default
        let query2 = query("UPDATE node SET is_default = ? WHERE name <> ?")
            .bind(false.to_sql())
            .bind(node_name.to_sql());
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()
    }

    async fn delete_node(&self, node_name: &str) -> Result<()> {
        let query = query("DELETE FROM node WHERE name=?").bind(node_name.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn set_tcp_listener_address(
        &self,
        node_name: &str,
        address: &InternetAddress,
    ) -> Result<()> {
        let query = query("UPDATE node SET tcp_listener_address = ? WHERE name = ?")
            .bind(address.to_string().to_sql())
            .bind(node_name.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn set_as_authority_node(&self, node_name: &str) -> Result<()> {
        let query = query("UPDATE node SET is_authority = ? WHERE name = ?")
            .bind(true.to_sql())
            .bind(node_name.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn get_tcp_listener_address(&self, node_name: &str) -> Result<Option<InternetAddress>> {
        Ok(self
            .get_node(node_name)
            .await?
            .and_then(|n| n.tcp_listener_address()))
    }

    async fn set_node_pid(&self, node_name: &str, pid: u32) -> Result<()> {
        let query = query("UPDATE node SET pid = ? WHERE name = ?")
            .bind(pid.to_sql())
            .bind(node_name.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn set_no_node_pid(&self, node_name: &str) -> Result<()> {
        let query = query("UPDATE node SET pid = NULL WHERE name = ?").bind(node_name.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn set_node_project_name(&self, node_name: &str, project_name: &str) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO node_project VALUES (?1, ?2)")
            .bind(node_name.to_sql())
            .bind(project_name.to_sql());
        Ok(query.execute(&*self.database.pool).await.void()?)
    }

    async fn get_node_project_name(&self, node_name: &str) -> Result<Option<String>> {
        let query = query("SELECT project_name FROM node_project WHERE node_name = ?")
            .bind(node_name.to_sql());
        let row: Option<SqliteRow> = query
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
    verbosity: u8,
    is_default: bool,
    is_authority: bool,
    tcp_listener_address: Option<String>,
    pid: Option<u32>,
}

impl NodeRow {
    pub(crate) fn node_info(&self) -> Result<NodeInfo> {
        let tcp_listener_address = match self.tcp_listener_address.clone() {
            None => None,
            Some(a) => Some(InternetAddress::new(a.as_str()).ok_or_else(|| {
                ockam_core::Error::new(
                    Origin::Api,
                    Kind::Serialization,
                    format!("cannot deserialize the tcp listener address {}", a),
                )
            })?),
        };

        Ok(NodeInfo::new(
            self.name.clone(),
            Identifier::from_str(&self.identifier.clone())?,
            self.verbosity,
            self.is_default,
            self.is_authority,
            tcp_listener_address,
            self.pid,
        ))
    }
}

#[cfg(test)]
mod test {
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
        )
    }
}
