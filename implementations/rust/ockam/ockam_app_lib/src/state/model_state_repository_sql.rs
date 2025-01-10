use std::str::FromStr;
use std::sync::Arc;

use sqlx::*;
use tracing::debug;

use ockam::transport::HostnamePort;
use ockam::{Boolean, FromSqlxError, Nullable, SqlxDatabase, ToVoid};
use ockam_api::nodes::models::portal::OutletStatus;
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
    async fn store(&self, node_name: &str, model_state: &ModelState) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        // remove previous tcp_outlet_status state
        query("DELETE FROM tcp_outlet_status where node_name = $1")
            .bind(node_name)
            .execute(&mut *transaction)
            .await
            .void()?;

        // re-insert the new state
        for tcp_outlet_status in &model_state.tcp_outlets {
            let query = query(
                r#"
                 INSERT INTO tcp_outlet_status (node_name, socket_addr, worker_addr, payload, privileged)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT DO NOTHING"#,
            )
            .bind(node_name)
            .bind(tcp_outlet_status.to.to_string())
            .bind(tcp_outlet_status.worker_addr.to_string())
            .bind(tcp_outlet_status.payload.as_ref())
            .bind(tcp_outlet_status.privileged);
            query.execute(&mut *transaction).await.void()?;
        }

        // remove previous incoming_service state
        query("DELETE FROM incoming_service")
            .execute(&mut *transaction)
            .await
            .void()?;

        // re-insert the new state
        for incoming_service in &model_state.incoming_services {
            let query = query(
                r#"
                 INSERT INTO incoming_service (invitation_id, enabled, name)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (invitation_id)
                 DO UPDATE SET enabled = $2, name = $3"#,
            )
            .bind(&incoming_service.invitation_id)
            .bind(incoming_service.enabled)
            .bind(incoming_service.name.as_ref());
            query.execute(&mut *transaction).await.void()?;
        }
        transaction.commit().await.void()?;

        Ok(())
    }

    async fn load(&self, node_name: &str) -> Result<ModelState> {
        let query1 = query_as(
            "SELECT socket_addr, worker_addr, payload, privileged FROM tcp_outlet_status WHERE node_name = $1",
        )
        .bind(node_name);
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
    socket_addr: String,
    worker_addr: String,
    payload: Nullable<String>,
    privileged: Boolean,
}

impl TcpOutletStatusRow {
    fn tcp_outlet_status(&self) -> Result<OutletStatus> {
        let to = HostnamePort::from_str(&self.socket_addr)?;
        let worker_addr = Address::from_string(&self.worker_addr);
        Ok(OutletStatus {
            to,
            worker_addr,
            payload: self.payload.to_option(),
            privileged: self.privileged.to_bool(),
        })
    }
}

/// Low-level representation of a row in the incoming_service table
#[derive(sqlx::FromRow)]
struct PersistentIncomingServiceRow {
    invitation_id: String,
    enabled: Boolean,
    name: Nullable<String>,
}

impl PersistentIncomingServiceRow {
    fn persistent_incoming_service(&self) -> Result<PersistentIncomingService> {
        Ok(PersistentIncomingService {
            invitation_id: self.invitation_id.clone(),
            enabled: self.enabled.to_bool(),
            name: self.name.to_option(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_api::nodes::models::portal::OutletStatus;
    use ockam_core::Address;
    use ockam_node::database::with_sqlite_dbs;

    #[tokio::test]
    async fn store_and_load() -> ockam_core::Result<()> {
        with_sqlite_dbs(|db| async move {
            let repository: Arc<dyn ModelStateRepository> =
                Arc::new(ModelStateSqlxDatabase::new(db.clone()));

            let node_name = "node";

            let mut state = ModelState::default();
            repository.store(node_name, &state).await.unwrap();
            let loaded = repository.load(node_name).await.unwrap();
            assert!(state.tcp_outlets.is_empty());
            assert_eq!(state, loaded);

            // Add a tcp outlet
            state.add_tcp_outlet(OutletStatus::new(
                "127.0.0.1:1001".parse().unwrap(),
                Address::from_string("s1"),
                None,
                true,
            ));
            // Add an incoming service
            state.add_incoming_service(PersistentIncomingService {
                invitation_id: "1235".to_string(),
                enabled: true,
                name: Some("aws".to_string()),
            });
            repository.store(node_name, &state).await.unwrap();
            let loaded = repository.load(node_name).await.unwrap();
            assert_eq!(state.tcp_outlets.len(), 1);
            assert_eq!(state.incoming_services.len(), 1);
            assert_eq!(state, loaded);

            // Add a few more
            for i in 2..=5 {
                state.add_tcp_outlet(OutletStatus::new(
                    format!("127.0.0.1:100{i}").parse().unwrap(),
                    Address::from_string(format!("s{i}")),
                    None,
                    true,
                ));
                repository.store(node_name, &state).await.unwrap();
            }
            let loaded = repository.load(node_name).await.unwrap();
            assert_eq!(state.tcp_outlets.len(), 5);
            assert_eq!(state, loaded);

            // Reload from DB scratch to emulate an app restart
            let repository: Arc<dyn ModelStateRepository> =
                Arc::new(ModelStateSqlxDatabase::new(db));
            let loaded = repository.load(node_name).await.unwrap();
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

            repository.store(node_name, &state).await.unwrap();
            let loaded = repository.load(node_name).await.unwrap();

            assert_eq!(state.tcp_outlets.len(), 2);
            assert_eq!(state.incoming_services.len(), 2);
            assert_eq!(state, loaded);

            Ok(())
        })
        .await
    }
}
