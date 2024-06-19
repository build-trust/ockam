use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use sqlx::*;
use tracing::debug;

use crate::nodes::models::portal::OutletStatus;
use ockam::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use ockam_core::{async_trait, Address};
use ockam_multiaddr::MultiAddr;
use ockam_node::HostnamePort;

use crate::cli_state::storage::tcp_portals_repository::TcpPortalsRepository;
use crate::cli_state::TcpInlet;

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
        tcp_inlet: &TcpInlet,
    ) -> ockam_core::Result<()> {
        let query = query("INSERT OR REPLACE INTO tcp_inlet VALUES (?, ?, ?, ?)")
            .bind(node_name.to_sql())
            .bind(tcp_inlet.bind_addr().to_string().to_sql())
            .bind(tcp_inlet.outlet_addr().to_string().to_sql())
            .bind(tcp_inlet.alias().to_sql());
        query.execute(&*self.database.pool).await.void()?;
        Ok(())
    }

    async fn get_tcp_inlet(
        &self,
        node_name: &str,
        alias: &str,
    ) -> ockam_core::Result<Option<TcpInlet>> {
        let query = query_as(
            "SELECT bind_addr, outlet_addr, alias FROM tcp_inlet WHERE node_name = ? AND alias = ?",
        )
        .bind(node_name.to_sql())
        .bind(alias.to_sql());
        let result: Option<TcpInletRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(result.map(|r| r.tcp_inlet()).transpose()?)
    }

    async fn delete_tcp_inlet(&self, node_name: &str, alias: &str) -> ockam_core::Result<()> {
        let query = query("DELETE FROM tcp_inlet WHERE node_name = ? AND alias = ?")
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
            .bind(tcp_outlet_status.to.to_sql())
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
}

// Database serialization / deserialization

/// Low-level representation of a row in the tcp_outlet_status table
#[derive(sqlx::FromRow)]
struct TcpInletRow {
    bind_addr: String,
    outlet_addr: String,
    alias: String,
}

impl TcpInletRow {
    fn bind_addr(&self) -> Result<SocketAddr> {
        SocketAddr::from_str(&self.bind_addr)
            .map_err(|e| ockam_core::Error::new(Origin::Api, Kind::Serialization, format!("{e:?}")))
    }

    fn outlet_addr(&self) -> Result<MultiAddr> {
        MultiAddr::from_str(&self.outlet_addr)
            .map_err(|e| ockam_core::Error::new(Origin::Api, Kind::Serialization, format!("{e:?}")))
    }

    fn tcp_inlet(&self) -> Result<TcpInlet> {
        Ok(TcpInlet::new(
            &self.bind_addr()?,
            &self.outlet_addr()?,
            &self.alias,
        ))
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
        let hostname_port = HostnamePort::from_str(&self.socket_addr)?;
        let worker_addr = Address::from_string(&self.worker_addr);
        Ok(OutletStatus {
            to: hostname_port,
            worker_addr,
            payload: self.payload.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_node::HostnamePort;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let db = create_database().await?;
        let repository = create_repository(db.clone());

        let tcp_inlet = TcpInlet::new(
            &SocketAddr::from_str("127.0.0.1:80").unwrap(),
            &MultiAddr::from_str("/node/outlet").unwrap(),
            "alias",
        );
        repository.store_tcp_inlet("node_name", &tcp_inlet).await?;
        let actual = repository.get_tcp_inlet("node_name", "alias").await?;
        assert_eq!(actual, Some(tcp_inlet.clone()));

        repository.delete_tcp_inlet("node_name", "alias").await?;
        let actual = repository.get_tcp_inlet("node_name", "alias").await?;
        assert_eq!(actual, None);

        let worker_addr = Address::from_str("worker_addr").unwrap();
        let tcp_outlet_status = OutletStatus::new(
            HostnamePort::new("127.0.0.1", 80),
            worker_addr.clone(),
            Some("payload".to_string()),
        );
        repository
            .store_tcp_outlet("node_name", &tcp_outlet_status)
            .await?;
        let actual = repository.get_tcp_outlet("node_name", &worker_addr).await?;
        assert_eq!(actual, Some(tcp_outlet_status.clone()));

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
        SqlxDatabase::in_memory("test").await
    }
}
