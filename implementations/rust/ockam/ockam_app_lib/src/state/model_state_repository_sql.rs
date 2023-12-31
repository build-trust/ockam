use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use sqlx::*;
use tracing::debug;

use ockam::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_core::{async_trait, Address};

use crate::incoming_services::PersistentIncomingService;
use crate::state::model::ModelState;
use crate::state::model_state_repository::ModelStateRepository;
use crate::Result;

#[derive(Clone)]
pub struct ModelStateSqlxDatabase {
    database: SqlxDatabase,
}

impl ModelStateSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for model state");
        Self { database }
    }

    /// Create a new in-memory database
    #[allow(unused)]
    pub async fn create() -> Result<Arc<Self>> {
        Ok(Arc::new(Self::new(
            SqlxDatabase::in_memory("model state").await?,
        )))
    }
}

/// The implementation simply serializes / deserializes the ModelState as JSON
#[async_trait]
impl ModelStateRepository for ModelStateSqlxDatabase {
    async fn store(&self, model_state: &ModelState) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        // remove previous tcp_outlet_status state
        query("DELETE FROM tcp_outlet_status")
            .execute(&mut *transaction)
            .await
            .void()?;

        // re-insert the new state
        for tcp_outlet_status in &model_state.tcp_outlets {
            let query = query("INSERT OR REPLACE INTO tcp_outlet_status VALUES (?, ?, ?, ?)")
                .bind(tcp_outlet_status.alias.to_sql())
                .bind(tcp_outlet_status.socket_addr.to_sql())
                .bind(tcp_outlet_status.worker_addr.to_sql())
                .bind(tcp_outlet_status.payload.as_ref().map(|p| p.to_sql()));
            query.execute(&mut *transaction).await.void()?;
        }

        // remove previous incoming_service state
        query("DELETE FROM incoming_service")
            .execute(&mut *transaction)
            .await
            .void()?;

        // re-insert the new state
        for incoming_service in &model_state.incoming_services {
            let query = query("INSERT OR REPLACE INTO incoming_service VALUES (?, ?, ?)")
                .bind(incoming_service.invitation_id.to_sql())
                .bind(incoming_service.enabled.to_sql())
                .bind(incoming_service.name.as_ref().map(|n| n.to_sql()));
            query.execute(&mut *transaction).await.void()?;
        }
        transaction.commit().await.void()?;

        Ok(())
    }

    async fn load(&self) -> Result<ModelState> {
        let query1 =
            query_as("SELECT alias, socket_addr, worker_addr, payload FROM tcp_outlet_status");
        let result: Vec<TcpOutletStatusRow> =
            query1.fetch_all(&*self.database.pool).await.into_core()?;
        let tcp_outlets = result
            .into_iter()
            .map(|r| r.tcp_outlet_status())
            .collect::<Result<Vec<_>>>()?;

        let query2 = query_as("SELECT invitation_id, enabled, name FROM incoming_service");
        let result: Vec<PersistentIncomingServiceRow> =
            query2.fetch_all(&*self.database.pool).await.into_core()?;
        let incoming_services = result
            .into_iter()
            .map(|r| r.persistent_incoming_service())
            .collect::<Result<Vec<_>>>()?;
        Ok(ModelState::new(tcp_outlets, incoming_services))
    }
}

// Database serialization / deserialization

/// Low-level representation of a row in the tcp_outlet_status table
#[derive(sqlx::FromRow)]
struct TcpOutletStatusRow {
    alias: String,
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
            alias: self.alias.clone(),
            socket_addr,
            worker_addr,
            payload: self.payload.clone(),
        })
    }
}

/// Low-level representation of a row in the incoming_service table
#[derive(sqlx::FromRow)]
struct PersistentIncomingServiceRow {
    invitation_id: String,
    enabled: bool,
    name: Option<String>,
}

impl PersistentIncomingServiceRow {
    fn persistent_incoming_service(&self) -> Result<PersistentIncomingService> {
        Ok(PersistentIncomingService {
            invitation_id: self.invitation_id.clone(),
            enabled: self.enabled,
            name: self.name.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use ockam_api::nodes::models::portal::OutletStatus;
    use ockam_core::Address;

    use super::*;

    #[tokio::test]
    async fn store_and_load() -> Result<()> {
        let db = create_database().await?;
        let repository = create_repository(db.clone());

        let mut state = ModelState::default();
        repository.store(&state).await?;
        let loaded = repository.load().await?;
        assert!(state.tcp_outlets.is_empty());
        assert_eq!(state, loaded);

        // Add a tcp outlet
        state.add_tcp_outlet(OutletStatus::new(
            "127.0.0.1:1001".parse()?,
            Address::from_string("s1"),
            "s1",
            None,
        ));
        // Add an incoming service
        state.add_incoming_service(PersistentIncomingService {
            invitation_id: "1235".to_string(),
            enabled: true,
            name: Some("aws".to_string()),
        });
        repository.store(&state).await?;
        let loaded = repository.load().await?;
        assert_eq!(state.tcp_outlets.len(), 1);
        assert_eq!(state.incoming_services.len(), 1);
        assert_eq!(state, loaded);

        // Add a few more
        for i in 2..=5 {
            state.add_tcp_outlet(OutletStatus::new(
                format!("127.0.0.1:100{i}").parse().unwrap(),
                Address::from_string(format!("s{i}")),
                &format!("s{i}"),
                None,
            ));
            repository.store(&state).await.unwrap();
        }
        let loaded = repository.load().await?;
        assert_eq!(state.tcp_outlets.len(), 5);
        assert_eq!(state, loaded);

        // Reload from DB scratch to emulate an app restart
        let repository = create_repository(db);
        let loaded = repository.load().await?;
        assert_eq!(state.tcp_outlets.len(), 5);
        assert_eq!(state.incoming_services.len(), 1);
        assert_eq!(state, loaded);

        // Remove some values from the current state
        let _ = state.tcp_outlets.split_off(2);
        state.add_incoming_service(PersistentIncomingService {
            invitation_id: "4567".to_string(),
            enabled: true,
            name: Some("aws".to_string()),
        });

        repository.store(&state).await?;
        let loaded = repository.load().await?;

        assert_eq!(state.tcp_outlets.len(), 2);
        assert_eq!(state.incoming_services.len(), 2);
        assert_eq!(state, loaded);

        Ok(())
    }

    /// HELPERS
    fn create_repository(db: SqlxDatabase) -> Arc<dyn ModelStateRepository> {
        Arc::new(ModelStateSqlxDatabase::new(db))
    }

    async fn create_database() -> Result<SqlxDatabase> {
        Ok(SqlxDatabase::in_memory("enrollments-test").await?)
    }
}
