use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use sqlx::*;
use tracing::debug;

use crate::nodes::models::portal::{InletStatus, OutletStatus};
use ockam::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_core::{async_trait, Address};

use crate::storage::tcp_portals_repository::TcpPortalsRepository;
use crate::{ConnectionStatus, Result};

#[derive(Clone)]
pub struct TcpPortalsSqlxDatabase {
    database: SqlxDatabase,
}

impl TcpPortalsSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for tcp portals");
        Self { database }
    }

    /// Create a new in-memory database
    #[allow(unused)]
    pub async fn create() -> Result<Arc<Self>> {
        Ok(Arc::new(Self::new(
            SqlxDatabase::in_memory("tcp portals").await?,
        )))
    }
}

#[async_trait]
impl TcpPortalsRepository for TcpPortalsSqlxDatabase {
    async fn store_tcp_inlet(
        &self,
        node_name: &str,
        tcp_inlet_status: &InletStatus,
    ) -> ockam_core::Result<()> {
        let query = query("INSERT OR REPLACE INTO tcp_inlet_status VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(node_name.to_sql())
            .bind(tcp_inlet_status.bind_addr.to_sql())
            .bind(tcp_inlet_status.worker_addr.as_ref().map(|w| w.to_sql()))
            .bind(tcp_inlet_status.alias.to_sql())
            .bind(tcp_inlet_status.payload.as_ref().map(|p| p.to_sql()))
            .bind(tcp_inlet_status.outlet_route.as_ref().map(|o| o.to_sql()))
            .bind(tcp_inlet_status.outlet_addr.to_sql());
        query.execute(&*self.database.pool).await.void()?;
        Ok(())
    }

    async fn get_tcp_inlet(
        &self,
        node_name: &str,
        alias: &str,
    ) -> ockam_core::Result<Option<InletStatus>> {
        let query = query_as("SELECT bind_addr, worker_addr, alias, payload, outlet_route, outlet_addr FROM tcp_inlet_status WHERE node_name = ? AND alias = ?")
            .bind(node_name.to_sql())
            .bind(alias.to_sql());
        let result: Option<TcpInletStatusRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(result.map(|r| r.tcp_inlet_status()).transpose()?)
    }

    async fn delete_tcp_inlet(&self, node_name: &str, alias: &str) -> ockam_core::Result<()> {
        let query = query("DELETE FROM tcp_inlet_status WHERE node_name = ? AND alias = ?")
            .bind(node_name.to_sql())
            .bind(alias.to_sql());
        query.execute(&*self.database.pool).await.into_core()?;
        Ok(())
    }

    async fn store_tcp_outlet(
        &self,
        node_name: &str,
        tcp_outlet_status: &OutletStatus,
    ) -> ockam_core::Result<()> {
        let query = query("INSERT OR REPLACE INTO tcp_outlet_status VALUES (?, ?, ?, ?)")
            .bind(node_name.to_sql())
            .bind(tcp_outlet_status.socket_addr.to_sql())
            .bind(tcp_outlet_status.worker_addr.to_sql())
            .bind(tcp_outlet_status.payload.as_ref().map(|p| p.to_sql()));
        query.execute(&*self.database.pool).await.void()?;
        Ok(())
    }

    async fn get_tcp_outlet(
        &self,
        node_name: &str,
        worker_addr: &Address,
    ) -> ockam_core::Result<Option<OutletStatus>> {
        let query = query_as("SELECT socket_addr, worker_addr, payload FROM tcp_outlet_status WHERE node_name = ? AND worker_addr = ?")
            .bind(node_name.to_sql())
            .bind(worker_addr.to_sql());
        let result: Option<TcpOutletStatusRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(result.map(|r| r.tcp_outlet_status()).transpose()?)
    }

    async fn delete_tcp_outlet(
        &self,
        node_name: &str,
        worker_addr: &Address,
    ) -> ockam_core::Result<()> {
        let query = query("DELETE FROM tcp_outlet_status WHERE node_name = ? AND worker_addr = ?")
            .bind(node_name.to_sql())
            .bind(worker_addr.to_sql());
        query.execute(&*self.database.pool).await.into_core()?;
        Ok(())
    }

    async fn delete_tcp_portals_by_node(&self, node_name: &str) -> ockam_core::Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        let query1 =
            query("DELETE FROM tcp_inlet_status WHERE node_name = ?").bind(node_name.to_sql());
        query1.execute(&mut *transaction).await.void()?;

        let query2 =
            query("DELETE FROM tcp_outlet_status WHERE node_name = ?").bind(node_name.to_sql());
        query2.execute(&mut *transaction).await.void()?;

        transaction.commit().await.void()
    }
}

// Database serialization / deserialization

/// Low-level representation of a row in the tcp_outlet_status table
#[derive(sqlx::FromRow)]
struct TcpInletStatusRow {
    bind_addr: String,
    worker_addr: Option<String>,
    alias: String,
    payload: Option<String>,
    outlet_route: Option<String>,
    outlet_addr: String,
}

impl TcpInletStatusRow {
    fn tcp_inlet_status(&self) -> Result<InletStatus> {
        Ok(InletStatus {
            bind_addr: self.bind_addr.clone(),
            worker_addr: self.worker_addr.clone(),
            alias: self.alias.clone(),
            payload: self.payload.clone(),
            outlet_route: self.outlet_route.clone(),
            status: ConnectionStatus::Up,
            outlet_addr: self.outlet_addr.clone(),
        })
    }
}

/// Low-level representation of a row in the tcp_outlet_status table
#[derive(sqlx::FromRow)]
struct TcpOutletStatusRow {
    socket_addr: String,
    worker_addr: String,
    payload: Option<String>,
}

impl TcpOutletStatusRow {
    fn tcp_outlet_status(&self) -> Result<OutletStatus> {
        let socket_addr = SocketAddr::from_str(&self.socket_addr)
            .map_err(|e| Error::new(Origin::Application, Kind::Serialization, e.to_string()))?;
        let worker_addr = Address::from_string(&self.worker_addr);
        Ok(OutletStatus {
            socket_addr,
            worker_addr,
            payload: self.payload.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let db = create_database().await?;
        let repository = create_repository(db.clone());

        let tcp_inlet_status = InletStatus::new(
            "bind_addr",
            Some("worker_addr".to_string()),
            "alias",
            Some("payload".to_string()),
            Some("outlet_route".to_string()),
            ConnectionStatus::Up,
            "outlet_addr",
        );
        repository
            .store_tcp_inlet("node_name", &tcp_inlet_status)
            .await?;
        let actual = repository.get_tcp_inlet("node_name", "alias").await?;
        assert_eq!(actual, Some(tcp_inlet_status));

        repository.delete_tcp_inlet("node_name", "alias").await?;
        let actual = repository.get_tcp_inlet("node_name", "alias").await?;
        assert_eq!(actual, None);

        let worker_addr = Address::from_str("worker_addr").unwrap();
        let tcp_outlet_status = OutletStatus::new(
            SocketAddr::from_str("127.0.0.1:80").unwrap(),
            worker_addr.clone(),
            Some("payload".to_string()),
        );
        repository
            .store_tcp_outlet("node_name", &tcp_outlet_status)
            .await?;
        let actual = repository.get_tcp_outlet("node_name", &worker_addr).await?;
        assert_eq!(actual, Some(tcp_outlet_status));

        repository
            .delete_tcp_outlet("node_name", &worker_addr)
            .await?;
        let actual = repository.get_tcp_outlet("node_name", &worker_addr).await?;
        assert_eq!(actual, None);

        Ok(())
    }

    /// HELPERS
    fn create_repository(db: SqlxDatabase) -> Arc<dyn TcpPortalsRepository> {
        Arc::new(TcpPortalsSqlxDatabase::new(db))
    }

    async fn create_database() -> Result<SqlxDatabase> {
        Ok(SqlxDatabase::in_memory("test").await?)
    }
}
