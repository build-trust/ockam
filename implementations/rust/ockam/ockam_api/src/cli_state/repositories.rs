use ockam::identity::storage::{PurposeKeysRepository, PurposeKeysSqlxDatabase};
use ockam::identity::{
    ChangeHistoryRepository, ChangeHistorySqlxDatabase, CredentialRepository,
    CredentialSqlxDatabase,
};
use ockam_core::compat::sync::Arc;
use ockam_node::database::DatabaseConfiguration;
use ockam_vault::storage::{SecretsRepository, SecretsSqlxDatabase};

use crate::cli_state::storage::*;
use crate::cli_state::CliState;
use crate::cli_state::{EnrollmentsRepository, EnrollmentsSqlxDatabase};
use crate::cli_state::{ProjectsRepository, ProjectsSqlxDatabase};
use crate::cli_state::{SpacesRepository, SpacesSqlxDatabase};
use crate::cli_state::{UsersRepository, UsersSqlxDatabase};

/// These functions create repository implementations to access data
/// stored in the database
impl CliState {
    pub fn change_history_repository(&self) -> Arc<dyn ChangeHistoryRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => {
                Arc::new(AutoRetry::new(ChangeHistorySqlxDatabase::new(database)))
            }
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(ChangeHistorySqlxDatabase::new(database))
            }
        }
    }

    pub(super) fn identities_repository(&self) -> Arc<dyn IdentitiesRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => {
                Arc::new(AutoRetry::new(IdentitiesSqlxDatabase::new(self.database())))
            }
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(IdentitiesSqlxDatabase::new(self.database()))
            }
        }
    }

    pub(super) fn purpose_keys_repository(&self) -> Arc<dyn PurposeKeysRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => Arc::new(AutoRetry::new(
                PurposeKeysSqlxDatabase::new(self.database()),
            )),
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(PurposeKeysSqlxDatabase::new(self.database()))
            }
        }
    }

    pub(super) fn secrets_repository(&self) -> Arc<dyn SecretsRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => {
                Arc::new(AutoRetry::new(SecretsSqlxDatabase::new(self.database())))
            }
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(SecretsSqlxDatabase::new(self.database()))
            }
        }
    }

    pub(super) fn vaults_repository(&self) -> Arc<dyn VaultsRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => {
                Arc::new(AutoRetry::new(VaultsSqlxDatabase::new(self.database())))
            }
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(VaultsSqlxDatabase::new(self.database()))
            }
        }
    }

    pub(super) fn enrollment_repository(&self) -> Arc<dyn EnrollmentsRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => Arc::new(AutoRetry::new(
                EnrollmentsSqlxDatabase::new(self.database()),
            )),
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(EnrollmentsSqlxDatabase::new(self.database()))
            }
        }
    }

    pub(super) fn nodes_repository(&self) -> Arc<dyn NodesRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => {
                Arc::new(AutoRetry::new(NodesSqlxDatabase::new(self.database())))
            }
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(NodesSqlxDatabase::new(self.database()))
            }
        }
    }

    pub(super) fn tcp_portals_repository(&self) -> Arc<dyn TcpPortalsRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => {
                Arc::new(AutoRetry::new(TcpPortalsSqlxDatabase::new(self.database())))
            }
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(TcpPortalsSqlxDatabase::new(self.database()))
            }
        }
    }

    pub(super) fn projects_repository(&self) -> Arc<dyn ProjectsRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => {
                Arc::new(AutoRetry::new(ProjectsSqlxDatabase::new(self.database())))
            }
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(ProjectsSqlxDatabase::new(self.database()))
            }
        }
    }

    pub(super) fn spaces_repository(&self) -> Arc<dyn SpacesRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => {
                Arc::new(AutoRetry::new(SpacesSqlxDatabase::new(self.database())))
            }
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(SpacesSqlxDatabase::new(self.database()))
            }
        }
    }

    pub(super) fn users_repository(&self) -> Arc<dyn UsersRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => {
                Arc::new(AutoRetry::new(UsersSqlxDatabase::new(self.database())))
            }
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(UsersSqlxDatabase::new(self.database()))
            }
        }
    }

    pub(super) fn user_journey_repository(&self) -> Arc<dyn JourneysRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => Arc::new(AutoRetry::new(
                JourneysSqlxDatabase::new(self.application_database()),
            )),
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(JourneysSqlxDatabase::new(self.application_database()))
            }
        }
    }

    pub fn cached_credentials_repository(&self, node_name: &str) -> Arc<dyn CredentialRepository> {
        let database = self.database();
        match database.configuration {
            DatabaseConfiguration::SqlitePersistent { .. }
            | DatabaseConfiguration::SqliteInMemory { .. } => Arc::new(AutoRetry::new(
                CredentialSqlxDatabase::new(self.database(), node_name),
            )),
            DatabaseConfiguration::Postgres { .. } => {
                Arc::new(CredentialSqlxDatabase::new(self.database(), node_name))
            }
        }
    }
}
