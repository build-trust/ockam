use ockam::identity::storage::{PurposeKeysRepository, PurposeKeysSqlxDatabase};
use ockam::identity::{
    ChangeHistoryRepository, ChangeHistorySqlxDatabase, IdentityAttributesRepository,
    IdentityAttributesSqlxDatabase,
};
use ockam_abac::{PoliciesRepository, PolicySqlxDatabase};
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
    pub(super) fn change_history_repository(&self) -> Arc<dyn ChangeHistoryRepository> {
        Arc::new(ChangeHistorySqlxDatabase::new(self.database()))
    }

    pub(super) fn identity_attributes_repository(&self) -> Arc<dyn IdentityAttributesRepository> {
        Arc::new(IdentityAttributesSqlxDatabase::new(self.database()))
    }

    pub(super) fn identities_repository(&self) -> Arc<dyn IdentitiesRepository> {
        Arc::new(IdentitiesSqlxDatabase::new(self.database()))
    }

    pub(super) fn purpose_keys_repository(&self) -> Arc<dyn PurposeKeysRepository> {
        Arc::new(PurposeKeysSqlxDatabase::new(self.database()))
    }

    pub(super) fn secrets_repository(&self) -> Arc<dyn SecretsRepository> {
        Arc::new(SecretsSqlxDatabase::new(self.database()))
    }

    pub(super) fn vaults_repository(&self) -> Arc<dyn VaultsRepository> {
        Arc::new(VaultsSqlxDatabase::new(self.database()))
    }

    pub(super) fn enrollment_repository(&self) -> Arc<dyn EnrollmentsRepository> {
        Arc::new(EnrollmentsSqlxDatabase::new(self.database()))
    }

    pub(super) fn nodes_repository(&self) -> Arc<dyn NodesRepository> {
        Arc::new(NodesSqlxDatabase::new(self.database()))
    }

    pub(super) fn policies_repository(&self) -> Arc<dyn PoliciesRepository> {
        Arc::new(PolicySqlxDatabase::new(self.database()))
    }

    pub(super) fn projects_repository(&self) -> Arc<dyn ProjectsRepository> {
        Arc::new(ProjectsSqlxDatabase::new(self.database()))
    }

    pub(super) fn spaces_repository(&self) -> Arc<dyn SpacesRepository> {
        Arc::new(SpacesSqlxDatabase::new(self.database()))
    }

    pub(super) fn users_repository(&self) -> Arc<dyn UsersRepository> {
        Arc::new(UsersSqlxDatabase::new(self.database()))
    }

    pub(super) fn user_journey_repository(&self) -> Arc<dyn JourneysRepository> {
        Arc::new(JourneysSqlxDatabase::new(self.application_database()))
    }
}
