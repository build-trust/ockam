use ockam::identity::storage::{PurposeKeysRepository, PurposeKeysSqlxDatabase};
use ockam::identity::{
    ChangeHistoryRepository, ChangeHistorySqlxDatabase, CredentialRepository,
    CredentialSqlxDatabase,
};
use ockam_core::compat::sync::Arc;
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
        ChangeHistorySqlxDatabase::make_repository(self.database())
    }

    pub(super) fn identities_repository(&self) -> Arc<dyn IdentitiesRepository> {
        IdentitiesSqlxDatabase::make_repository(self.database())
    }

    pub(super) fn purpose_keys_repository(&self) -> Arc<dyn PurposeKeysRepository> {
        PurposeKeysSqlxDatabase::make_repository(self.database())
    }

    pub(super) fn secrets_repository(&self) -> Arc<dyn SecretsRepository> {
        SecretsSqlxDatabase::make_repository(self.database())
    }

    pub(super) fn vaults_repository(&self) -> Arc<dyn VaultsRepository> {
        VaultsSqlxDatabase::make_repository(self.database())
    }

    pub(super) fn enrollment_repository(&self) -> Arc<dyn EnrollmentsRepository> {
        EnrollmentsSqlxDatabase::make_repository(self.database())
    }

    pub(super) fn nodes_repository(&self) -> Arc<dyn NodesRepository> {
        NodesSqlxDatabase::make_repository(self.database())
    }

    pub(super) fn tcp_portals_repository(&self) -> Arc<dyn TcpPortalsRepository> {
        TcpPortalsSqlxDatabase::make_repository(self.database())
    }

    pub(super) fn projects_repository(&self) -> Arc<dyn ProjectsRepository> {
        ProjectsSqlxDatabase::make_repository(self.database())
    }

    pub(super) fn spaces_repository(&self) -> Arc<dyn SpacesRepository> {
        SpacesSqlxDatabase::make_repository(self.database())
    }

    pub(super) fn users_repository(&self) -> Arc<dyn UsersRepository> {
        UsersSqlxDatabase::make_repository(self.database())
    }

    pub(super) fn user_journey_repository(&self) -> Arc<dyn JourneysRepository> {
        JourneysSqlxDatabase::make_repository(self.application_database())
    }

    pub fn cached_credentials_repository(&self, node_name: &str) -> Arc<dyn CredentialRepository> {
        CredentialSqlxDatabase::make_repository(self.database(), node_name)
    }
}
