use crate::authenticator::direct::{CredentialIssuer, EnrollmentTokenAuthenticator};
use crate::bootstrapped_identities_store::{BootstrapedIdentityStore, PreTrustedIdentities};
use crate::lmdb::LmdbStorage;
use crate::nodes::authority_node::authority::EnrollerCheck::{AnyMember, EnrollerOnly};
use crate::nodes::authority_node::{Configuration, TrustedIdentity};
use crate::{actions, DefaultAddress};
use ockam_abac::expr::{and, eq, ident, str};
use ockam_abac::{AbacAccessControl, Env};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::sessions::{SessionPolicy, Sessions};
use ockam_core::{AllowAll, AsyncTryClone, Error, Message, Result, Worker};
use ockam_identity::authenticated_storage::{
    AttributesEntry, AuthenticatedAttributeStorage, AuthenticatedStorage, IdentityAttributeStorage,
};
use ockam_identity::{
    Identity, IdentityIdentifier, IdentityVault, PublicIdentity, SecureChannelListenerTrustOptions,
    SecureChannelRegistry, TrustMultiIdentifiersPolicy,
};
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_tcp::{TcpListenerTrustOptions, TcpTransport};
use ockam_vault::storage::FileStorage;
use ockam_vault::Vault;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::info;

/// This struct represents an Authority, which is an
/// Identity which other identities trust to authenticate attributes
/// An Authority is able to start a few services
//   - a direct authenticator
//   - a credential issuer
//   - an enrollment token issuer
//   - an enrollment token acceptor
pub struct Authority {
    identity: Identity,
    attributes_storage: Arc<dyn IdentityAttributeStorage>,
}

impl Authority {
    /// Create a new Authority with a given identity
    /// The list of trusted identities is used to pre-populate an attributes storage
    /// In practice it contains the list of identities with the ockam-role attribute set as 'enroller'
    pub(crate) fn new(identity: Identity, configuration: Configuration) -> Self {
        let attributes_storage = Self::make_attributes_storage(&identity, configuration);
        Self {
            identity,
            attributes_storage,
        }
    }
}

/// Public functions to:
///   - create an Authority
///   - start services
impl Authority {
    /// Return the public identity for this authority
    pub async fn public_identity(&self) -> Result<PublicIdentity> {
        self.identity.to_public().await
    }

    /// Create an identity for an authority if it has not been created before
    /// otherwise retrieve it from storage
    pub async fn create(ctx: &Context, configuration: Configuration) -> Result<Authority> {
        let vault = Self::create_identity_vault(&configuration).await?;
        let storage = Self::create_authenticated_storage(&configuration).await?;

        let identity = if let Some(authority_change_history) =
            storage.get("authority", "change_history").await?
        {
            let identity = Identity::import_ext(
                ctx,
                authority_change_history.as_slice(),
                storage.clone(),
                &SecureChannelRegistry::new(),
                vault,
            )
            .await?;
            info!("retrieved the authority identity");
            identity
        } else {
            let identity = Identity::create_ext(ctx, storage.clone(), vault).await?;
            // persist the identity for later retrieval
            storage
                .set(
                    "authority",
                    "change_history".to_string(),
                    identity.change_history().await.export()?,
                )
                .await?;

            info!("created the authority identity");
            identity
        };
        Ok(Authority::new(identity, configuration))
    }

    /// Start the secure channel listener service, using TCP as a transport
    /// The TCP listener is connected to the secure channel listener so that it can only
    /// be used to create secure channels.
    pub async fn start_secure_channel_listener(
        &self,
        ctx: &Context,
        configuration: Configuration,
    ) -> Result<()> {
        let sessions = Sessions::default();

        // Start a secure channel listener that only allows channels with
        // authenticated identities.
        let tcp_listener_session_id = sessions.generate_session_id();
        let secure_channel_listener_session_id = sessions.generate_session_id();

        let trust_options = SecureChannelListenerTrustOptions::new()
            .with_trust_policy(TrustMultiIdentifiersPolicy::new(
                configuration
                    .trusted_identities
                    .iter()
                    .map(|a| a.identifier())
                    .collect(),
            ))
            .as_consumer(
                &sessions,
                &tcp_listener_session_id,
                SessionPolicy::SpawnerAllowOnlyOneMessage,
            )
            .as_spawner(&sessions, &secure_channel_listener_session_id);

        let listener_name = configuration.secure_channel_listener_name();
        self.identity
            .create_secure_channel_listener(listener_name.clone(), trust_options)
            .await?;
        info!("started a secure channel listener with name '{listener_name}'");

        // Create a TCP listener and wait for incoming connections
        let tcp = TcpTransport::create(ctx).await?;
        let tcp_listener_trust_options =
            TcpListenerTrustOptions::new().as_spawner(&sessions, &tcp_listener_session_id);

        let (address, _) = tcp
            .listen(
                configuration.tcp_listener_address(),
                tcp_listener_trust_options,
            )
            .await?;

        info!("started a TCP listener at {address:?}");
        Ok(())
    }

    /// Start the authenticator service to enroll project members
    pub async fn start_direct_authenticator(
        &self,
        ctx: &Context,
        configuration: Configuration,
    ) -> Result<()> {
        let direct = crate::authenticator::direct::DirectAuthenticator::new(
            configuration.clone().project_identifier(),
            self.attributes_storage().clone(),
            self.identity.authenticated_storage().clone(),
        )
        .await?;

        let name = configuration.clone().authenticator_name();
        self.start(ctx, configuration, name.clone(), EnrollerOnly, direct)
            .await?;

        info!("started a direct authenticator at '{name}'");
        Ok(())
    }

    /// Start the enrollment services, to issue and accept tokens
    pub async fn start_enrollment_services(
        &self,
        ctx: &Context,
        configuration: Configuration,
    ) -> Result<()> {
        let (issuer, acceptor) = EnrollmentTokenAuthenticator::new_worker_pair(
            configuration.project_identifier(),
            self.attributes_storage(),
        );

        // start an enrollment token issuer with an abac policy checking that
        // the caller is an enroller for the authority project
        let issuer_address: String = DefaultAddress::ENROLLMENT_TOKEN_ISSUER.into();
        self.start(
            ctx,
            configuration.clone(),
            issuer_address.clone(),
            EnrollerOnly,
            issuer,
        )
        .await?;

        // start an enrollment token acceptor allowing any incoming message as long as
        // it comes through a secure channel. We accept any message since the purpose of
        // that service is to access a one-time token stating that the sender of the message
        // is a project member
        let acceptor_address: String = DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR.into();
        WorkerBuilder::with_access_control(
            Arc::new(AllowAll),
            Arc::new(AllowAll),
            acceptor_address.clone(),
            acceptor,
        )
        .start(ctx)
        .await?;

        info!("started an enrollment token issuer at '{issuer_address}'");
        info!("started an enrollment token acceptor at '{acceptor_address}'");
        Ok(())
    }

    /// Start the credential issuer service to issue credentials for a identities
    /// known to the authority
    pub async fn start_credential_issuer(
        &self,
        ctx: &Context,
        configuration: Configuration,
    ) -> Result<()> {
        // create and start a credential issuer worker
        let issuer = CredentialIssuer::new(
            configuration.project_identifier(),
            self.attributes_storage()
                .clone()
                .as_identity_attribute_storage_reader(),
            Arc::new(self.identity.async_try_clone().await?),
        )
        .await?;

        let address = DefaultAddress::CREDENTIAL_ISSUER.to_string();
        self.start(ctx, configuration, address.clone(), AnyMember, issuer)
            .await?;

        info!("started a credential issuer at '{address}'");
        Ok(())
    }

    /// Start the Okta service to retrieve attributes authenticated by Okta
    pub async fn start_okta(&self, ctx: &Context, configuration: Configuration) -> Result<()> {
        if let Some(okta) = configuration.clone().okta {
            let okta_worker = crate::okta::Server::new(
                configuration.project_identifier(),
                self.attributes_storage()
                    .as_identity_attribute_storage_writer(),
                okta.tenant_base_url(),
                okta.certificate(),
                okta.attributes().as_slice(),
            )?;

            ctx.start_worker(
                okta.address,
                okta_worker,
                AllowAll, // FIXME: @ac
                AllowAll,
            )
            .await?;
        }
        Ok(())
    }
}

/// Private Authority functions
impl Authority {
    /// Return the attribute storage used by the authority
    fn attributes_storage(&self) -> Arc<dyn IdentityAttributeStorage> {
        self.attributes_storage.clone()
    }

    /// Create an identity vault backed by a FileStorage
    async fn create_identity_vault(
        configuration: &Configuration,
    ) -> Result<Arc<dyn IdentityVault>> {
        let vault_path = PathBuf::from_str(configuration.vault_path.as_str()).unwrap();
        Self::create_ockam_directory_if_necessary(vault_path.clone())?;
        let mut file_storage = FileStorage::new(vault_path);
        file_storage.init().await?;
        let vault = Arc::new(Vault::new(Some(Arc::new(file_storage))));
        Ok(vault)
    }

    /// Create an authenticated storage backed by a Lmdb database
    async fn create_authenticated_storage(
        configuration: &Configuration,
    ) -> Result<Arc<dyn AuthenticatedStorage>> {
        let storage_path = PathBuf::from_str(configuration.storage_path.as_str()).unwrap();
        Self::create_ockam_directory_if_necessary(storage_path.clone())?;
        let storage = Arc::new(LmdbStorage::new(storage_path).await?);
        Ok(storage)
    }

    /// Create a directory to save storage files if they haven't been  created before
    fn create_ockam_directory_if_necessary(path: PathBuf) -> Result<()> {
        let parent = path.parent().unwrap();
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| Error::new(Origin::Node, Kind::Io, e))?;
        }
        Ok(())
    }

    /// Make an identity attributes storage pre-populated with the attributes of some trusted
    /// identities.
    /// Note that the trusted identities attributes are never persisted to disk so the only source
    /// of truth is always the content of the configuration file for the authority node
    fn make_attributes_storage(
        authority: &Identity,
        configuration: Configuration,
    ) -> Arc<dyn IdentityAttributeStorage> {
        let project_identifier = configuration.project_identifier;
        let trusted_identities = configuration.trusted_identities;

        let trusted: HashMap<IdentityIdentifier, AttributesEntry> =
            HashMap::from_iter(trusted_identities.iter().map(|t: &TrustedIdentity| {
                (
                    t.clone().identifier(),
                    t.attributes_entry(project_identifier.clone(), authority.identifier()),
                )
            }));
        let attributes_storage: Arc<dyn IdentityAttributeStorage> =
            Arc::new(BootstrapedIdentityStore::new(
                Arc::new(PreTrustedIdentities::new_from_hashmap(trusted)),
                Arc::new(AuthenticatedAttributeStorage::new(
                    authority.authenticated_storage(),
                )),
            ));
        attributes_storage
    }

    /// Start a worker at a given address
    /// The configuration is used to create an Abac incoming policy checking that
    /// the sender can indeed call the authority services
    async fn start<M, W>(
        &self,
        ctx: &Context,
        configuration: Configuration,
        address: String,
        enroller_check: EnrollerCheck,
        worker: W,
    ) -> Result<()>
    where
        M: Message + Send + 'static,
        W: Worker<Context = Context, Message = M>,
    {
        let abac = self.create_abac_policy(&configuration, address.clone(), enroller_check);
        WorkerBuilder::with_access_control(abac, Arc::new(AllowAll), address.clone(), worker)
            .start(ctx)
            .await
            .map(|_| ())
    }

    /// Return an Abac incoming policy checking that for the authority services
    /// The configuration is used to check that
    ///   - the service is accessed via a secure channel
    ///   - the sender has the correct project identifier (the same as the authority)
    ///   - if enroller_check == EnrollerOnly, the sender is an identity with 'enroller' as its 'ockam-role'
    fn create_abac_policy(
        &self,
        configuration: &Configuration,
        address: String,
        enroller_check: EnrollerCheck,
    ) -> Arc<AbacAccessControl> {
        // create an ABAC policy to only allow messages having
        // the same project id as the authority
        let rule = if enroller_check == EnrollerOnly {
            and([
                eq([ident("resource.project_id"), ident("subject.project_id")]),
                eq([ident("subject.ockam-role"), str("enroller")]),
            ])
        } else {
            eq([ident("resource.project_id"), ident("subject.project_id")])
        };
        let mut env = Env::new();
        env.put("resource.id", str(address.as_str()));
        env.put("action.id", str(actions::HANDLE_MESSAGE.as_str()));
        env.put(
            "resource.project_id",
            str(configuration.clone().project_identifier),
        );
        let abac = Arc::new(AbacAccessControl::new(self.attributes_storage(), rule, env));
        abac
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EnrollerCheck {
    EnrollerOnly,
    AnyMember,
}
