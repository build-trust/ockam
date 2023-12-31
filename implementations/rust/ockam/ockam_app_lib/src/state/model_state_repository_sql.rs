use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use sqlx::*;
use tracing::debug;

use ockam::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_core::{async_trait, Address};

use crate::incoming_services::PersistentIncomingService;
use crate::local_service::PersistentLocalService;
use crate::state::model::ModelState;
use crate::state::model_state_repository::ModelStateRepository;
use crate::Result;

#[derive(Clone)]
pub struct ModelStateSqlxDatabase {
    database: Arc<SqlxDatabase>,
}

impl ModelStateSqlxDatabase {
    /// Create a new database
    pub fn new(database: Arc<SqlxDatabase>) -> Self {
        debug!("create a repository for model state");
        Self { database }
    }

    /// Create a database on the specified path
    pub async fn create_at<P: AsRef<Path>>(path: P) -> Result<Arc<Self>> {
        Ok(Arc::new(Self::new(Arc::new(
            SqlxDatabase::create(path).await?,
        ))))
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
        query("DELETE FROM local_service")
            .execute(&mut *transaction)
            .await
            .void()?;

        // re-insert the new state
        for local_service in &model_state.local_services {
            let query = query("INSERT OR REPLACE INTO local_service VALUES (?, ?, ?, ?)")
                .bind(local_service.alias.to_sql())
                .bind(local_service.socket_addr.to_sql())
                .bind(local_service.worker_addr.to_sql())
                .bind(local_service.scheme.as_ref().map(|p| p.to_sql()));
            query.execute(&mut *transaction).await.void()?;
        }

        // remove previous incoming_service state
        query("DELETE FROM incoming_service")
            .execute(&mut *transaction)
            .await
            .void()?;

        // re-insert the new state
        for incoming_service in &model_state.incoming_services {
            let query = query("INSERT OR REPLACE INTO incoming_service VALUES (?, ?, ?, ?, ?)")
                .bind(incoming_service.invitation_id.to_sql())
                .bind(incoming_service.enabled.to_sql())
                .bind(incoming_service.name.as_ref().map(|n| n.to_sql()))
                .bind(incoming_service.port.as_ref().map(|n| n.to_sql()))
                .bind(incoming_service.scheme.as_ref().map(|n| n.to_sql()));
            query.execute(&mut *transaction).await.void()?;
        }
        transaction.commit().await.void()?;

        Ok(())
    }

    async fn load(&self) -> Result<ModelState> {
        let query1 = query_as("SELECT alias, socket_addr, worker_addr, scheme FROM local_service");
        let result: Vec<PersistentLocalServiceRow> =
            query1.fetch_all(&self.database.pool).await.into_core()?;
        let local_services = result
            .into_iter()
            .map(|r| r.into_persistent_local_service())
            .collect::<Result<Vec<_>>>()?;

        let query2 =
            query_as("SELECT invitation_id, enabled, name, port, scheme FROM incoming_service");
        let result: Vec<PersistentIncomingServiceRow> =
            query2.fetch_all(&self.database.pool).await.into_core()?;
        let incoming_services = result
            .into_iter()
            .map(|r| r.into_persistent_incoming_service())
            .collect::<Result<Vec<_>>>()?;
        Ok(ModelState::new(local_services, incoming_services))
    }
}

// Database serialization / deserialization

/// Low-level representation of a row in the local_service table
#[derive(sqlx::FromRow)]
struct PersistentLocalServiceRow {
    alias: String,
    socket_addr: String,
    worker_addr: String,
    scheme: Option<String>,
}

impl PersistentLocalServiceRow {
    fn into_persistent_local_service(self) -> Result<PersistentLocalService> {
        let socket_addr = SocketAddr::from_str(&self.socket_addr)
            .map_err(|e| Error::new(Origin::Application, Kind::Serialization, e.to_string()))?;
        let worker_addr = Address::from_string(&self.worker_addr);
        Ok(PersistentLocalService {
            alias: self.alias,
            socket_addr,
            worker_addr,
            scheme: self.scheme,
        })
    }
}

/// Low-level representation of a row in the incoming_service table
#[derive(sqlx::FromRow)]
struct PersistentIncomingServiceRow {
    invitation_id: String,
    enabled: bool,
    name: Option<String>,
    port: Option<u16>,
    scheme: Option<String>,
}

impl PersistentIncomingServiceRow {
    fn into_persistent_incoming_service(self) -> Result<PersistentIncomingService> {
        Ok(PersistentIncomingService {
            invitation_id: self.invitation_id,
            enabled: self.enabled,
            name: self.name,
            port: self.port,
            scheme: self.scheme,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::Address;

    #[tokio::test]
    async fn store_and_load() -> Result<()> {
        let db = create_database().await?;
        let repository = create_repository(db.clone());

        let mut state = ModelState::default();
        repository.store(&state).await?;
        let loaded = repository.load().await?;
        assert!(state.local_services.is_empty());
        assert_eq!(state, loaded);

        // Add a local service
        state.add_local_service(PersistentLocalService {
            socket_addr: "127.0.0.1:1001".parse()?,
            worker_addr: Address::from_string("s1"),
            alias: "s1".to_string(),
            scheme: Some("http".to_string()),
        });
        // Add an incoming service
        state.add_incoming_service(PersistentIncomingService {
            invitation_id: "1235".to_string(),
            enabled: true,
            name: Some("aws".to_string()),
            port: Some(1022),
            scheme: Some("ssh".to_string()),
        });
        repository.store(&state).await?;
        let loaded = repository.load().await?;
        assert_eq!(state.local_services.len(), 1);
        assert_eq!(state.incoming_services.len(), 1);
        assert_eq!(state, loaded);

        // Add a few more
        for i in 2..=5 {
            state.add_local_service(PersistentLocalService {
                socket_addr: format!("127.0.0.1:100{i}").parse()?,
                worker_addr: Address::from_string(format!("s{i}")),
                alias: format!("s{i}"),
                scheme: None,
            });
            repository.store(&state).await.unwrap();
        }
        let loaded = repository.load().await?;
        assert_eq!(state.local_services.len(), 5);
        assert_eq!(state, loaded);

        // Reload from DB scratch to emulate an app restart
        let repository = create_repository(db);
        let loaded = repository.load().await?;
        assert_eq!(state.local_services.len(), 5);
        assert_eq!(state.incoming_services.len(), 1);
        assert_eq!(state, loaded);

        // Remove some values from the current state
        let _ = state.local_services.split_off(2);
        state.add_incoming_service(PersistentIncomingService {
            invitation_id: "4567".to_string(),
            enabled: true,
            name: Some("aws".to_string()),
            port: None,
            scheme: None,
        });

        repository.store(&state).await?;
        let loaded = repository.load().await?;

        assert_eq!(state.local_services.len(), 2);
        assert_eq!(state.incoming_services.len(), 2);
        assert_eq!(state, loaded);

        Ok(())
    }

    /// HELPERS
    fn create_repository(db: Arc<SqlxDatabase>) -> Arc<dyn ModelStateRepository> {
        Arc::new(ModelStateSqlxDatabase::new(db))
    }

    async fn create_database() -> Result<Arc<SqlxDatabase>> {
        Ok(SqlxDatabase::in_memory("enrollments-test").await?)
    }
}
