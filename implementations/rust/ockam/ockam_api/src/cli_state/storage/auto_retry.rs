use crate::cli_state::journeys::{Journey, ProjectJourney};
use crate::cli_state::{
    EnrollmentsRepository, IdentitiesRepository, IdentityEnrollment, JourneysRepository,
    NamedIdentity, NamedVault, NodeInfo, NodesRepository, ProjectsRepository, SpacesRepository,
    TcpInlet, TcpPortalsRepository, UsersRepository, VaultType, VaultsRepository,
};
use crate::cloud::email_address::EmailAddress;
use crate::cloud::enroll::auth0::UserInfo;
use crate::cloud::project::models::ProjectModel;
use crate::cloud::space::Space;
use crate::config::lookup::InternetAddress;
use crate::nodes::models::portal::OutletStatus;
use chrono::{DateTime, Utc};
use ockam::identity::models::{ChangeHistory, CredentialAndPurposeKey, PurposeKeyAttestation};
use ockam::identity::storage::PurposeKeysRepository;
use ockam::identity::{
    ChangeHistoryRepository, CredentialRepository, Identifier, Identity, Purpose,
    TimestampInSeconds,
};
use ockam_core::{async_trait, Address};
use ockam_vault::storage::SecretsRepository;
use ockam_vault::{
    AeadSecret, AeadSecretKeyHandle, SigningSecret, SigningSecretKeyHandle, X25519SecretKey,
    X25519SecretKeyHandle,
};

macro_rules! retry {
    ($async_function:expr) => {{
        let mut retries = 0;
        loop {
            match $async_function.await {
                Ok(result) => break Ok(result),
                Err(err) => {
                    if err.to_string().contains("database is locked") && retries < 100 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    } else {
                        break Err(err);
                    }
                    retries += 1;
                }
            }
        }
    }};
}

#[derive(Clone)]
pub(crate) struct AutoRetry<T: Sized + Send + Sync + 'static> {
    wrapped: T,
}

impl<T: Send + Sync + 'static> AutoRetry<T> {
    pub(crate) fn new(wrapped_trait: T) -> AutoRetry<T> {
        Self {
            wrapped: wrapped_trait,
        }
    }
}

#[async_trait]
impl<T: EnrollmentsRepository> EnrollmentsRepository for AutoRetry<T> {
    async fn set_as_enrolled(
        &self,
        identifier: &Identifier,
        email: &EmailAddress,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_as_enrolled(identifier, email))
    }

    async fn get_enrolled_identities(&self) -> ockam_core::Result<Vec<IdentityEnrollment>> {
        retry!(self.wrapped.get_enrolled_identities())
    }

    async fn get_all_identities_enrollments(&self) -> ockam_core::Result<Vec<IdentityEnrollment>> {
        retry!(self.wrapped.get_all_identities_enrollments())
    }

    async fn is_default_identity_enrolled(&self) -> ockam_core::Result<bool> {
        retry!(self.wrapped.is_default_identity_enrolled())
    }

    async fn is_identity_enrolled(&self, name: &str) -> ockam_core::Result<bool> {
        retry!(self.wrapped.is_identity_enrolled(name))
    }
}

#[async_trait]
impl<T: IdentitiesRepository + Send + Sync + 'static> IdentitiesRepository for AutoRetry<T> {
    async fn store_named_identity(
        &self,
        identifier: &Identifier,
        name: &str,
        vault_name: &str,
    ) -> ockam_core::Result<NamedIdentity> {
        retry!(self
            .wrapped
            .store_named_identity(identifier, name, vault_name))
    }

    async fn delete_identity(&self, name: &str) -> ockam_core::Result<Option<Identifier>> {
        retry!(self.wrapped.delete_identity(name))
    }

    async fn delete_identity_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> ockam_core::Result<Option<String>> {
        retry!(self.wrapped.delete_identity_by_identifier(identifier))
    }

    async fn get_identifier(&self, name: &str) -> ockam_core::Result<Option<Identifier>> {
        retry!(self.wrapped.get_identifier(name))
    }

    async fn get_identity_name_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> ockam_core::Result<Option<String>> {
        retry!(self.wrapped.get_identity_name_by_identifier(identifier))
    }

    async fn get_named_identity(&self, name: &str) -> ockam_core::Result<Option<NamedIdentity>> {
        retry!(self.wrapped.get_named_identity(name))
    }

    async fn get_named_identity_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> ockam_core::Result<Option<NamedIdentity>> {
        retry!(self.wrapped.get_named_identity_by_identifier(identifier))
    }

    async fn get_named_identities(&self) -> ockam_core::Result<Vec<NamedIdentity>> {
        retry!(self.wrapped.get_named_identities())
    }

    async fn get_named_identities_by_vault_name(
        &self,
        vault_name: &str,
    ) -> ockam_core::Result<Vec<NamedIdentity>> {
        retry!(self.wrapped.get_named_identities_by_vault_name(vault_name))
    }

    async fn set_as_default(&self, name: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_as_default(name))
    }

    async fn set_as_default_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_as_default_by_identifier(identifier))
    }

    async fn get_default_named_identity(&self) -> ockam_core::Result<Option<NamedIdentity>> {
        retry!(self.wrapped.get_default_named_identity())
    }
}

#[async_trait]
impl<T: JourneysRepository> JourneysRepository for AutoRetry<T> {
    async fn store_project_journey(
        &self,
        project_journey: ProjectJourney,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_project_journey(project_journey.clone()))
    }

    async fn get_project_journey(
        &self,
        project_id: &str,
        now: DateTime<Utc>,
    ) -> ockam_core::Result<Option<ProjectJourney>> {
        retry!(self.wrapped.get_project_journey(project_id, now))
    }

    async fn delete_project_journeys(&self, project_id: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_project_journeys(project_id))
    }

    async fn store_host_journey(&self, host_journey: Journey) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_host_journey(host_journey.clone()))
    }

    async fn get_host_journey(&self, now: DateTime<Utc>) -> ockam_core::Result<Option<Journey>> {
        retry!(self.wrapped.get_host_journey(now))
    }
}

#[async_trait]
impl<T: NodesRepository> NodesRepository for AutoRetry<T> {
    async fn store_node(&self, node_info: &NodeInfo) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_node(node_info))
    }

    async fn get_nodes(&self) -> ockam_core::Result<Vec<NodeInfo>> {
        retry!(self.wrapped.get_nodes())
    }

    async fn get_node(&self, node_name: &str) -> ockam_core::Result<Option<NodeInfo>> {
        retry!(self.wrapped.get_node(node_name))
    }

    async fn get_nodes_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> ockam_core::Result<Vec<NodeInfo>> {
        retry!(self.wrapped.get_nodes_by_identifier(identifier))
    }

    async fn get_default_node(&self) -> ockam_core::Result<Option<NodeInfo>> {
        retry!(self.wrapped.get_default_node())
    }

    async fn set_default_node(&self, node_name: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_default_node(node_name))
    }

    async fn is_default_node(&self, node_name: &str) -> ockam_core::Result<bool> {
        retry!(self.wrapped.is_default_node(node_name))
    }

    async fn delete_node(&self, node_name: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_node(node_name))
    }

    async fn set_tcp_listener_address(
        &self,
        node_name: &str,
        address: &InternetAddress,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_tcp_listener_address(node_name, address))
    }

    async fn set_status_endpoint_address(
        &self,
        node_name: &str,
        address: &InternetAddress,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_status_endpoint_address(node_name, address))
    }

    async fn set_as_authority_node(&self, node_name: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_as_authority_node(node_name))
    }

    async fn get_tcp_listener_address(
        &self,
        node_name: &str,
    ) -> ockam_core::Result<Option<InternetAddress>> {
        retry!(self.wrapped.get_tcp_listener_address(node_name))
    }

    async fn get_status_endpoint_address(
        &self,
        node_name: &str,
    ) -> ockam_core::Result<Option<InternetAddress>> {
        retry!(self.wrapped.get_status_endpoint_address(node_name))
    }

    async fn set_node_pid(&self, node_name: &str, pid: u32) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_node_pid(node_name, pid))
    }

    async fn set_no_node_pid(&self, node_name: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_no_node_pid(node_name))
    }

    async fn set_node_project_name(
        &self,
        node_name: &str,
        project_name: &str,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_node_project_name(node_name, project_name))
    }

    async fn get_node_project_name(&self, node_name: &str) -> ockam_core::Result<Option<String>> {
        retry!(self.wrapped.get_node_project_name(node_name))
    }
}

#[async_trait]
impl<T: ProjectsRepository> ProjectsRepository for AutoRetry<T> {
    async fn store_project(&self, project: &ProjectModel) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_project(project))
    }

    async fn get_project(&self, project_id: &str) -> ockam_core::Result<Option<ProjectModel>> {
        retry!(self.wrapped.get_project(project_id))
    }

    async fn get_project_by_name(&self, name: &str) -> ockam_core::Result<Option<ProjectModel>> {
        retry!(self.wrapped.get_project_by_name(name))
    }

    async fn get_projects(&self) -> ockam_core::Result<Vec<ProjectModel>> {
        retry!(self.wrapped.get_projects())
    }

    async fn get_default_project(&self) -> ockam_core::Result<Option<ProjectModel>> {
        retry!(self.wrapped.get_default_project())
    }

    async fn set_default_project(&self, project_id: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_default_project(project_id))
    }

    async fn delete_project(&self, project_id: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_project(project_id))
    }
}

#[async_trait]
impl<T: SpacesRepository> SpacesRepository for AutoRetry<T> {
    async fn store_space(&self, space: &Space) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_space(space))
    }

    async fn get_space(&self, space_id: &str) -> ockam_core::Result<Option<Space>> {
        retry!(self.wrapped.get_space(space_id))
    }

    async fn get_space_by_name(&self, name: &str) -> ockam_core::Result<Option<Space>> {
        retry!(self.wrapped.get_space_by_name(name))
    }

    async fn get_spaces(&self) -> ockam_core::Result<Vec<Space>> {
        retry!(self.wrapped.get_spaces())
    }

    async fn get_default_space(&self) -> ockam_core::Result<Option<Space>> {
        retry!(self.wrapped.get_default_space())
    }

    async fn set_default_space(&self, space_id: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_default_space(space_id))
    }

    async fn delete_space(&self, space_id: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_space(space_id))
    }
}

#[async_trait]
impl<T: TcpPortalsRepository> TcpPortalsRepository for AutoRetry<T> {
    async fn store_tcp_inlet(
        &self,
        node_name: &str,
        tcp_inlet: &TcpInlet,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_tcp_inlet(node_name, tcp_inlet))
    }

    async fn get_tcp_inlet(
        &self,
        node_name: &str,
        alias: &str,
    ) -> ockam_core::Result<Option<TcpInlet>> {
        retry!(self.wrapped.get_tcp_inlet(node_name, alias))
    }

    async fn delete_tcp_inlet(&self, node_name: &str, alias: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_tcp_inlet(node_name, alias))
    }

    async fn store_tcp_outlet(
        &self,
        node_name: &str,
        tcp_outlet_status: &OutletStatus,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_tcp_outlet(node_name, tcp_outlet_status))
    }

    async fn get_tcp_outlet(
        &self,
        node_name: &str,
        worker_addr: &Address,
    ) -> ockam_core::Result<Option<OutletStatus>> {
        retry!(self.wrapped.get_tcp_outlet(node_name, worker_addr))
    }

    async fn delete_tcp_outlet(
        &self,
        node_name: &str,
        worker_addr: &Address,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_tcp_outlet(node_name, worker_addr))
    }
}

#[async_trait]
impl<T: UsersRepository> UsersRepository for AutoRetry<T> {
    async fn store_user(&self, user: &UserInfo) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_user(user))
    }

    async fn get_default_user(&self) -> ockam_core::Result<Option<UserInfo>> {
        retry!(self.wrapped.get_default_user())
    }

    async fn set_default_user(&self, email: &EmailAddress) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_default_user(email))
    }

    async fn get_user(&self, email: &EmailAddress) -> ockam_core::Result<Option<UserInfo>> {
        retry!(self.wrapped.get_user(email))
    }

    async fn get_users(&self) -> ockam_core::Result<Vec<UserInfo>> {
        retry!(self.wrapped.get_users())
    }

    async fn delete_user(&self, email: &EmailAddress) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_user(email))
    }
}

#[async_trait]
impl<T: VaultsRepository> VaultsRepository for AutoRetry<T> {
    async fn store_vault(
        &self,
        name: &str,
        vault_type: VaultType,
    ) -> ockam_core::Result<NamedVault> {
        retry!(self.wrapped.store_vault(name, vault_type.clone()))
    }

    async fn update_vault(&self, name: &str, vault_type: VaultType) -> ockam_core::Result<()> {
        retry!(self.wrapped.update_vault(name, vault_type.clone()))
    }

    async fn delete_named_vault(&self, name: &str) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_named_vault(name))
    }

    async fn get_database_vault(&self) -> ockam_core::Result<Option<NamedVault>> {
        retry!(self.wrapped.get_database_vault())
    }

    async fn get_named_vault(&self, name: &str) -> ockam_core::Result<Option<NamedVault>> {
        retry!(self.wrapped.get_named_vault(name))
    }

    async fn get_named_vaults(&self) -> ockam_core::Result<Vec<NamedVault>> {
        retry!(self.wrapped.get_named_vaults())
    }
}

#[async_trait]
impl<T: ChangeHistoryRepository> ChangeHistoryRepository for AutoRetry<T> {
    async fn update_identity(
        &self,
        identity: &Identity,
        ignore_older: bool,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.update_identity(identity, ignore_older))
    }

    async fn store_change_history(
        &self,
        identifier: &Identifier,
        change_history: ChangeHistory,
    ) -> ockam_core::Result<()> {
        retry!(self
            .wrapped
            .store_change_history(identifier, change_history.clone()))
    }

    async fn delete_change_history(&self, identifier: &Identifier) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_change_history(identifier))
    }

    async fn get_change_history(
        &self,
        identifier: &Identifier,
    ) -> ockam_core::Result<Option<ChangeHistory>> {
        retry!(self.wrapped.get_change_history(identifier))
    }

    async fn get_change_histories(&self) -> ockam_core::Result<Vec<ChangeHistory>> {
        retry!(self.wrapped.get_change_histories())
    }
}

#[async_trait]
impl<T: PurposeKeysRepository> PurposeKeysRepository for AutoRetry<T> {
    async fn set_purpose_key(
        &self,
        subject: &Identifier,
        purpose: Purpose,
        purpose_key_attestation: &PurposeKeyAttestation,
    ) -> ockam_core::Result<()> {
        retry!(self
            .wrapped
            .set_purpose_key(subject, purpose, purpose_key_attestation))
    }

    async fn delete_purpose_key(
        &self,
        subject: &Identifier,
        purpose: Purpose,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_purpose_key(subject, purpose))
    }

    async fn get_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> ockam_core::Result<Option<PurposeKeyAttestation>> {
        retry!(self.wrapped.get_purpose_key(identifier, purpose))
    }

    async fn delete_all(&self) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_all())
    }
}

#[async_trait]
impl<T: CredentialRepository> CredentialRepository for AutoRetry<T> {
    async fn get(
        &self,
        subject: &Identifier,
        issuer: &Identifier,
        scope: &str,
    ) -> ockam_core::Result<Option<CredentialAndPurposeKey>> {
        retry!(self.wrapped.get(subject, issuer, scope))
    }

    async fn put(
        &self,
        subject: &Identifier,
        issuer: &Identifier,
        scope: &str,
        expires_at: TimestampInSeconds,
        credential: CredentialAndPurposeKey,
    ) -> ockam_core::Result<()> {
        retry!(self
            .wrapped
            .put(subject, issuer, scope, expires_at, credential.clone()))
    }

    async fn delete(
        &self,
        subject: &Identifier,
        issuer: &Identifier,
        scope: &str,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete(subject, issuer, scope))
    }
}

#[async_trait]
impl<T: SecretsRepository> SecretsRepository for AutoRetry<T> {
    async fn store_signing_secret(
        &self,
        handle: &SigningSecretKeyHandle,
        secret: SigningSecret,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_signing_secret(handle, secret.clone()))
    }

    async fn delete_signing_secret(
        &self,
        handle: &SigningSecretKeyHandle,
    ) -> ockam_core::Result<bool> {
        retry!(self.wrapped.delete_signing_secret(handle))
    }

    async fn get_signing_secret(
        &self,
        handle: &SigningSecretKeyHandle,
    ) -> ockam_core::Result<Option<SigningSecret>> {
        retry!(self.wrapped.get_signing_secret(handle))
    }

    async fn get_signing_secret_handles(&self) -> ockam_core::Result<Vec<SigningSecretKeyHandle>> {
        retry!(self.wrapped.get_signing_secret_handles())
    }

    async fn store_x25519_secret(
        &self,
        handle: &X25519SecretKeyHandle,
        secret: X25519SecretKey,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_x25519_secret(handle, secret.clone()))
    }

    async fn delete_x25519_secret(
        &self,
        handle: &X25519SecretKeyHandle,
    ) -> ockam_core::Result<bool> {
        retry!(self.wrapped.delete_x25519_secret(handle))
    }

    async fn get_x25519_secret(
        &self,
        handle: &X25519SecretKeyHandle,
    ) -> ockam_core::Result<Option<X25519SecretKey>> {
        retry!(self.wrapped.get_x25519_secret(handle))
    }

    async fn get_x25519_secret_handles(&self) -> ockam_core::Result<Vec<X25519SecretKeyHandle>> {
        retry!(self.wrapped.get_x25519_secret_handles())
    }

    async fn store_aead_secret(
        &self,
        handle: &AeadSecretKeyHandle,
        secret: AeadSecret,
    ) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_aead_secret(handle, secret.clone()))
    }

    async fn delete_aead_secret(&self, handle: &AeadSecretKeyHandle) -> ockam_core::Result<bool> {
        retry!(self.wrapped.delete_aead_secret(handle))
    }

    async fn get_aead_secret(
        &self,
        handle: &AeadSecretKeyHandle,
    ) -> ockam_core::Result<Option<AeadSecret>> {
        retry!(self.wrapped.get_aead_secret(handle))
    }

    async fn delete_all(&self) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_all())
    }
}
