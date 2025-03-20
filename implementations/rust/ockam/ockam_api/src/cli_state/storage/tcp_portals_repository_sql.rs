use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use sqlx::*;
use tracing::debug;

use crate::cli_state::storage::tcp_portals_repository::TcpPortalsRepository;
use crate::cli_state::TcpInlet;
use crate::nodes::models::portal::TcpOutletInfo;
use crate::nodes::service::tcp_outlets::TcpOutletParameters;
use ockam::{FromSqlxError, SqlxDatabase, ToVoid};
use ockam_abac::PolicyExpression;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_core::Result;
use ockam_core::{async_trait, Address};
use ockam_multiaddr::MultiAddr;
use ockam_node::database::AutoRetry;
use ockam_node::database::Boolean;
use ockam_transport_core::HostnamePort;

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

    /// Create a repository
    pub fn make_repository(database: SqlxDatabase) -> Arc<dyn TcpPortalsRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database)))
        } else {
            Arc::new(Self::new(database))
        }
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
        let query = query(
            r#"
            INSERT INTO tcp_inlet (node_name, bind_addr, outlet_addr, alias, privileged)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT DO NOTHING"#,
        )
        .bind(node_name)
        .bind(tcp_inlet.bind_addr().to_string())
        .bind(tcp_inlet.outlet_addr().to_string())
        .bind(tcp_inlet.alias())
        .bind(tcp_inlet.privileged());
        query.execute(&*self.database.pool).await.void()?;
        Ok(())
    }

    async fn get_tcp_inlet(
        &self,
        node_name: &str,
        alias: &str,
    ) -> ockam_core::Result<Option<TcpInlet>> {
        let query = query_as(
            "SELECT bind_addr, outlet_addr, alias, privileged FROM tcp_inlet WHERE node_name = $1 AND alias = $2",
        )
        .bind(node_name)
        .bind(alias);
        let result: Option<TcpInletRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(result.map(|r| r.tcp_inlet()).transpose()?)
    }

    async fn delete_tcp_inlet(&self, node_name: &str, alias: &str) -> ockam_core::Result<()> {
        let query = query("DELETE FROM tcp_inlet WHERE node_name = $1 AND alias = $2")
            .bind(node_name)
            .bind(alias);
        query.execute(&*self.database.pool).await.into_core()?;
        Ok(())
    }

    async fn list_tcp_inlets(&self, node_name: &str) -> Result<Vec<TcpInlet>> {
        let query = query_as(
            "SELECT bind_addr, outlet_addr, alias, privileged FROM tcp_inlet WHERE node_name = $1",
        )
        .bind(node_name);
        let result: Vec<TcpInletRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        Ok(result
            .into_iter()
            .map(|r| r.tcp_inlet())
            .collect::<Result<Vec<_>>>()?)
    }

    async fn store_tcp_outlet(
        &self,
        node_name: &str,
        tcp_outlet_status: &TcpOutletInfo,
    ) -> Result<()> {
        let query = query(
            r#"
            INSERT INTO tcp_outlet (
               node_name, "to", worker_address, policy_expression, tls, privileged, skip_handshake, enable_nagle
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT DO NOTHING"#,
        )
        .bind(node_name)
        .bind(tcp_outlet_status.parameters.to.to_string())
        .bind(tcp_outlet_status.worker_address.address())
        .bind(tcp_outlet_status.parameters.policy_expression.as_ref().map(|e| e.to_string()))
        .bind(tcp_outlet_status.parameters.tls)
        .bind(tcp_outlet_status.parameters.privileged)
        .bind(tcp_outlet_status.parameters.skip_handshake)
        .bind(tcp_outlet_status.parameters.enable_nagle);
        query.execute(&*self.database.pool).await.void()?;
        Ok(())
    }

    async fn get_tcp_outlet(
        &self,
        node_name: &str,
        worker_address: &Address,
    ) -> ockam_core::Result<Option<TcpOutletInfo>> {
        let query = query_as(
            r#"
            SELECT "to", worker_address, policy_expression, tls, privileged, skip_handshake, enable_nagle
            FROM tcp_outlet WHERE node_name = $1 AND worker_address = $2
            "#
            )
            .bind(node_name)
            .bind(worker_address.address());
        let result: Option<TcpOutletStatusRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(result.map(|r| r.tcp_outlet()).transpose()?)
    }

    async fn delete_tcp_outlet(
        &self,
        node_name: &str,
        worker_address: &Address,
    ) -> ockam_core::Result<()> {
        let query = query("DELETE FROM tcp_outlet WHERE node_name = $1 AND worker_address = $2")
            .bind(node_name)
            .bind(worker_address.address());
        query.execute(&*self.database.pool).await.into_core()?;
        Ok(())
    }

    async fn list_tcp_outlets(&self, node_name: &str) -> Result<Vec<TcpOutletInfo>> {
        let query = query_as(
            r#"
            SELECT "to", worker_address, policy_expression, tls, privileged, skip_handshake, enable_nagle
            FROM tcp_outlet WHERE node_name = $1
        "#,
        )
        .bind(node_name);
        let result: Vec<TcpOutletStatusRow> =
            query.fetch_all(&*self.database.pool).await.into_core()?;
        Ok(result
            .into_iter()
            .map(|r| r.tcp_outlet())
            .collect::<Result<Vec<_>>>()?)
    }
}

// Database serialization / deserialization

/// Low-level representation of a row in the tcp_outlet_status table
#[derive(sqlx::FromRow)]
struct TcpInletRow {
    bind_addr: String,
    outlet_addr: String,
    alias: String,
    privileged: Boolean,
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
            self.privileged.to_bool(),
        ))
    }
}

/// Low-level representation of a row in the tcp_outlet_status table
#[derive(sqlx::FromRow)]
struct TcpOutletStatusRow {
    to: String,
    policy_expression: Option<String>,
    worker_address: String,
    tls: Boolean,
    privileged: Boolean,
    skip_handshake: Boolean,
    enable_nagle: Boolean,
}

impl TcpOutletStatusRow {
    fn tcp_outlet(&self) -> Result<TcpOutletInfo> {
        let to = HostnamePort::from_str(&self.to)
            .map_err(|e| Error::new(Origin::Application, Kind::Serialization, e.to_string()))?;
        let worker_address = Address::from_string(&self.worker_address);
        let policy_expression = if let Some(expression) = &self.policy_expression {
            Some(PolicyExpression::from_str(expression)?)
        } else {
            None
        };

        Ok(TcpOutletInfo {
            parameters: TcpOutletParameters {
                to,
                policy_expression,
                worker_address: Some(worker_address.clone()),
                tls: self.tls.to_bool(),
                privileged: self.privileged.to_bool(),
                skip_handshake: self.skip_handshake.to_bool(),
                enable_nagle: self.enable_nagle.to_bool(),
            },
            worker_address,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::service::tcp_outlets::TcpOutletParameters;
    use ockam_node::database::with_dbs;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        with_dbs(|db| async move {
            let repository: Arc<dyn TcpPortalsRepository> =
                Arc::new(TcpPortalsSqlxDatabase::new(db));

            let tcp_inlet = TcpInlet::new(
                &SocketAddr::from_str("127.0.0.1:80").unwrap(),
                &MultiAddr::from_str("/node/outlet").unwrap(),
                "alias",
                true,
            );
            repository.store_tcp_inlet("node_name", &tcp_inlet).await?;
            let actual = repository.get_tcp_inlet("node_name", "alias").await?;
            assert_eq!(actual, Some(tcp_inlet.clone()));

            let inlets = repository.list_tcp_inlets("node_name").await?;
            assert_eq!(inlets, vec![tcp_inlet.clone()]);

            repository.delete_tcp_inlet("node_name", "alias").await?;
            let actual = repository.get_tcp_inlet("node_name", "alias").await?;
            assert_eq!(actual, None);

            let worker_addr = Address::from_str("worker_addr").unwrap();
            let tcp_outlet_status = TcpOutletInfo::new(
                TcpOutletParameters::new(HostnamePort::from_str("127.0.0.1:80").unwrap())
                    .with_worker_address(worker_addr.clone())
                    .with_privileged(true),
                worker_addr.clone(),
            );
            repository
                .store_tcp_outlet("node_name", &tcp_outlet_status)
                .await?;
            let actual = repository.get_tcp_outlet("node_name", &worker_addr).await?;
            assert_eq!(actual, Some(tcp_outlet_status.clone()));

            let outlets = repository.list_tcp_outlets("node_name").await?;
            assert_eq!(outlets, vec![tcp_outlet_status.clone()]);

            repository
                .delete_tcp_outlet("node_name", &worker_addr)
                .await?;
            let actual = repository.get_tcp_outlet("node_name", &worker_addr).await?;
            assert_eq!(actual, None);

            Ok(())
        })
        .await
    }
}
