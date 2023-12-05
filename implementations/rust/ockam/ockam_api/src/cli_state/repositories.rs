use ockam::identity::{
    ChangeHistoryRepository, ChangeHistorySqlxDatabase, IdentityAttributesRepository,
    IdentityAttributesSqlxDatabase,
};
use ockam_abac::{PoliciesRepository, PolicySqlxDatabase};
use ockam_core::compat::sync::Arc;

use crate::cli_state::error::Result;
use crate::cli_state::storage::*;
use crate::cli_state::CliState;
use crate::cli_state::{EnrollmentsRepository, EnrollmentsSqlxDatabase};
use crate::cli_state::{ProjectsRepository, ProjectsSqlxDatabase};
use crate::cli_state::{SpacesRepository, SpacesSqlxDatabase};
use crate::cli_state::{TrustContextsRepository, TrustContextsSqlxDatabase};
use crate::cli_state::{UsersRepository, UsersSqlxDatabase};

/// These functions create repository implementations to access data
/// stored in the database
impl CliState {
    pub(super) async fn change_history_repository(
        &self,
    ) -> Result<Arc<dyn ChangeHistoryRepository>> {
        Ok(Arc::new(ChangeHistorySqlxDatabase::new(self.database())))
    }

    pub(super) async fn identity_attributes_repository(
        &self,
    ) -> Result<Arc<dyn IdentityAttributesRepository>> {
        Ok(Arc::new(IdentityAttributesSqlxDatabase::new(
            self.database(),
        )))
    }

    pub(super) async fn identities_repository(&self) -> Result<Arc<dyn IdentitiesRepository>> {
        Ok(Arc::new(IdentitiesSqlxDatabase::new(self.database())))
    }

    pub(super) async fn vaults_repository(&self) -> Result<Arc<dyn VaultsRepository>> {
        Ok(Arc::new(VaultsSqlxDatabase::new(self.database())))
    }

    pub(super) async fn enrollment_repository(&self) -> Result<Arc<dyn EnrollmentsRepository>> {
        Ok(Arc::new(EnrollmentsSqlxDatabase::new(self.database())))
    }

    pub(super) async fn nodes_repository(&self) -> Result<Arc<dyn NodesRepository>> {
        Ok(Arc::new(NodesSqlxDatabase::new(self.database())))
    }

    pub(super) async fn policies_repository(&self) -> Result<Arc<dyn PoliciesRepository>> {
        Ok(Arc::new(PolicySqlxDatabase::new(self.database())))
    }

    pub(super) async fn projects_repository(&self) -> Result<Arc<dyn ProjectsRepository>> {
        Ok(Arc::new(ProjectsSqlxDatabase::new(self.database())))
    }

    pub(super) async fn spaces_repository(&self) -> Result<Arc<dyn SpacesRepository>> {
        Ok(Arc::new(SpacesSqlxDatabase::new(self.database())))
    }

    pub(super) async fn users_repository(&self) -> Result<Arc<dyn UsersRepository>> {
        Ok(Arc::new(UsersSqlxDatabase::new(self.database())))
    }

    pub(super) async fn credentials_repository(&self) -> Result<Arc<dyn CredentialsRepository>> {
        Ok(Arc::new(CredentialsSqlxDatabase::new(self.database())))
    }

    pub(super) async fn trust_contexts_repository(
        &self,
    ) -> Result<Arc<dyn TrustContextsRepository>> {
        Ok(Arc::new(TrustContextsSqlxDatabase::new(self.database())))
    }
}
