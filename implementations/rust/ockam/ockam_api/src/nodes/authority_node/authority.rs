use crate::authenticator::direct::EnrollmentTokenAuthenticator;
use crate::bootstrapped_identities_store::BootstrapedIdentityStore;
use crate::echoer::Echoer;
use crate::lmdb::LmdbStorage;
use crate::nodes::authority_node::authority::EnrollerCheck::{AnyMember, EnrollerOnly};
use crate::nodes::authority_node::Configuration;
use crate::{actions, DefaultAddress};
use ockam::identity::{
    secure_channels, Identities, IdentitiesRepository, IdentitiesStorage, IdentitiesVault,
    Identity, IdentityAttributesWriter, SecureChannelListenerOptions, SecureChannels,
    TrustEveryonePolicy,
};
use ockam_abac::expr::{and, eq, ident, str};
use ockam_abac::{AbacAccessControl, Env};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::{FlowControlId, FlowControlPolicy, FlowControls};
use ockam_core::{Address, AllowAll, Error, Message, Result, Worker};
use ockam_identity::CredentialsIssuer;
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_tcp::{TcpListenerOptions, TcpTransport};
use ockam_vault::storage::FileStorage;
use ockam_vault::Vault;
use std::path::Path;
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
    secure_channels: Arc<SecureChannels>,
}

/// Public functions to:
///   - create an Authority
///   - start services
impl Authority {
    /// Return the public identity for this authority
    pub fn identity(&self) -> Identity {
        self.identity.clone()
    }

    /// Create an identity for an authority from the configured public identity and configured vault
    /// The list of trusted identities in the configuration is used to pre-populate an attributes storage
    /// In practice it contains the list of identities with the ockam-role attribute set as 'enroller'
    pub async fn create(configuration: &Configuration) -> Result<Authority> {
        info!("configuration {:?}", configuration);
        let vault = Self::create_secure_channels_vault(configuration).await?;
        let repository = Self::create_identities_repository(configuration).await?;
        let secure_channels = secure_channels::builder()
            .with_identities_vault(vault)
            .with_identities_repository(repository)
            .build();

        let identity = secure_channels
            .identities()
            .identities_creation()
            .import_identity(configuration.identity.export()?.as_slice())
            .await?;
        info!("retrieved the authority identity {}", identity.identifier());

        Ok(Authority {
            identity,
            secure_channels,
        })
    }

    /// Start the secure channel listener service, using TCP as a transport
    /// The TCP listener is connected to the secure channel listener so that it can only
    /// be used to create secure channels.
    pub async fn start_secure_channel_listener(
        &self,
        ctx: &Context,
        flow_controls: &FlowControls,
        configuration: &Configuration,
    ) -> Result<FlowControlId> {
        // Start a secure channel listener that only allows channels with
        // authenticated identities.
        let tcp_listener_flow_control_id = flow_controls.generate_id();
        let secure_channel_listener_flow_control_id = flow_controls.generate_id();

        let options = SecureChannelListenerOptions::as_spawner(
            flow_controls,
            &secure_channel_listener_flow_control_id,
        )
        .with_trust_policy(TrustEveryonePolicy)
        .as_consumer_with_flow_control_id(
            flow_controls,
            &tcp_listener_flow_control_id,
            FlowControlPolicy::SpawnerAllowOnlyOneMessage,
        );

        let listener_name = configuration.secure_channel_listener_name();
        self.secure_channels
            .create_secure_channel_listener(ctx, &self.identity, listener_name.clone(), options)
            .await?;
        info!("started a secure channel listener with name '{listener_name}'");

        // Create a TCP listener and wait for incoming connections
        let tcp = TcpTransport::create(ctx).await?;
        let tcp_listener_options =
            TcpListenerOptions::as_spawner(flow_controls, &tcp_listener_flow_control_id);

        let (address, _) = tcp
            .listen(configuration.tcp_listener_address(), tcp_listener_options)
            .await?;

        info!("started a TCP listener at {address:?}");
        Ok(secure_channel_listener_flow_control_id)
    }

    /// Start the authenticator service to enroll project members
    pub async fn start_direct_authenticator(
        &self,
        ctx: &Context,
        flow_controls: &FlowControls,
        secure_channel_flow_control_id: &FlowControlId,
        configuration: &Configuration,
    ) -> Result<()> {
        if configuration.no_direct_authentication {
            return Ok(());
        }

        let direct = crate::authenticator::direct::DirectAuthenticator::new(
            configuration.clone().trust_context_identifier(),
            self.attributes_writer(),
        )
        .await?;

        let name = configuration.clone().authenticator_name();
        flow_controls.add_consumer(
            &Address::from_string(name.clone()),
            secure_channel_flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );

        self.start(ctx, configuration, name.clone(), EnrollerOnly, direct)
            .await?;

        info!("started a direct authenticator at '{name}'");
        Ok(())
    }

    /// Start the enrollment services, to issue and accept tokens
    pub async fn start_enrollment_services(
        &self,
        ctx: &Context,
        flow_controls: &FlowControls,
        secure_channel_flow_control_id: &FlowControlId,
        configuration: &Configuration,
    ) -> Result<()> {
        if configuration.no_token_enrollment {
            return Ok(());
        }

        let (issuer, acceptor) = EnrollmentTokenAuthenticator::new_worker_pair(
            configuration.trust_context_identifier(),
            self.attributes_writer(),
        );

        // start an enrollment token issuer with an abac policy checking that
        // the caller is an enroller for the authority project
        let issuer_address: String = DefaultAddress::ENROLLMENT_TOKEN_ISSUER.into();
        flow_controls.add_consumer(
            &Address::from_string(issuer_address.clone()),
            secure_channel_flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );

        self.start(
            ctx,
            configuration,
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
        flow_controls.add_consumer(
            &Address::from_string(acceptor_address.clone()),
            secure_channel_flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );

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
        flow_controls: &FlowControls,
        secure_channel_flow_control_id: &FlowControlId,
        configuration: &Configuration,
    ) -> Result<()> {
        // create and start a credential issuer worker
        let issuer = CredentialsIssuer::new(
            self.identities(),
            self.identity.clone(),
            configuration.trust_context_identifier(),
        )
        .await?;

        let address = DefaultAddress::CREDENTIAL_ISSUER.to_string();
        flow_controls.add_consumer(
            &Address::from_string(address.clone()),
            secure_channel_flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );

        self.start(ctx, configuration, address.clone(), AnyMember, issuer)
            .await?;

        info!("started a credential issuer at '{address}'");
        Ok(())
    }

    /// Start the Okta service to retrieve attributes authenticated by Okta
    pub async fn start_okta(
        &self,
        ctx: &Context,
        flow_controls: &FlowControls,
        secure_channel_flow_control_id: &FlowControlId,
        configuration: &Configuration,
    ) -> Result<()> {
        if let Some(okta) = configuration.clone().okta {
            let okta_worker = crate::okta::Server::new(
                self.attributes_writer(),
                configuration.project_identifier(),
                okta.tenant_base_url(),
                okta.certificate(),
                okta.attributes().as_slice(),
            )?;

            flow_controls.add_consumer(
                &Address::from_string(okta.address.clone()),
                secure_channel_flow_control_id,
                FlowControlPolicy::SpawnerAllowMultipleMessages,
            );

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

    /// Start an echo service
    pub async fn start_echo_service(
        &self,
        ctx: &Context,
        flow_controls: &FlowControls,
        secure_channel_flow_control_id: &FlowControlId,
    ) -> Result<()> {
        let address = DefaultAddress::ECHO_SERVICE;

        flow_controls.add_consumer(
            &Address::from_string(address),
            secure_channel_flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );

        ctx.start_worker(address, Echoer, AllowAll, AllowAll).await
    }
}

/// Private Authority functions
impl Authority {
    /// Return the identities storage used by the authority
    fn identities(&self) -> Arc<Identities> {
        self.secure_channels.identities()
    }

    /// Return the identities repository used by the authority
    fn identities_repository(&self) -> Arc<dyn IdentitiesRepository> {
        self.identities().repository().clone()
    }

    /// Return the identities repository used by the authority
    fn attributes_writer(&self) -> Arc<dyn IdentityAttributesWriter> {
        self.identities_repository().as_attributes_writer().clone()
    }

    /// Create an identity vault backed by a FileStorage
    async fn create_secure_channels_vault(
        configuration: &Configuration,
    ) -> Result<Arc<dyn IdentitiesVault>> {
        let vault_path = &configuration.vault_path;
        Self::create_ockam_directory_if_necessary(vault_path)?;
        let mut file_storage = FileStorage::new(vault_path.clone());
        file_storage.init().await?;
        let vault = Arc::new(Vault::new(Some(Arc::new(file_storage))));
        Ok(vault)
    }

    /// Create an authenticated storage backed by a Lmdb database
    async fn create_identities_repository(
        configuration: &Configuration,
    ) -> Result<Arc<dyn IdentitiesRepository>> {
        let storage_path = &configuration.storage_path;
        Self::create_ockam_directory_if_necessary(storage_path)?;
        let storage = Arc::new(LmdbStorage::new(&storage_path).await?);
        let repository = Arc::new(IdentitiesStorage::new(storage));
        Ok(Self::bootstrap_repository(repository, configuration))
    }

    /// Create a directory to save storage files if they haven't been  created before
    fn create_ockam_directory_if_necessary(path: &Path) -> Result<()> {
        let parent = path.parent().unwrap();
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| Error::new(Origin::Node, Kind::Io, e))?;
        }
        Ok(())
    }

    /// Make an identities repository pre-populated with the attributes of some trusted
    /// identities. The values either come from the command line or are read directly from a file
    /// every time we try to retrieve some attributes
    fn bootstrap_repository(
        repository: Arc<dyn IdentitiesRepository>,
        configuration: &Configuration,
    ) -> Arc<dyn IdentitiesRepository> {
        let trusted_identities = &configuration.trusted_identities;
        Arc::new(BootstrapedIdentityStore::new(
            Arc::new(trusted_identities.clone()),
            repository.clone(),
        ))
    }

    /// Start a worker at a given address
    /// The configuration is used to create an Abac incoming policy checking that
    /// the sender can indeed call the authority services
    async fn start<M, W>(
        &self,
        ctx: &Context,
        configuration: &Configuration,
        address: String,
        enroller_check: EnrollerCheck,
        worker: W,
    ) -> Result<()>
    where
        M: Message + Send + 'static,
        W: Worker<Context = Context, Message = M>,
    {
        let abac = self.create_abac_policy(configuration, address.clone(), enroller_check);
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
                eq([ident("resource.project_id"), ident("subject.project_id")]), // TODO: DEPRECATE - Removing PROJECT_ID attribute in favor of TRUST_CONTEXT_ID
                eq([
                    ident("resource.trust_context_id"),
                    ident("subject.trust_context_id"),
                ]),
                eq([ident("subject.ockam-role"), str("enroller")]),
            ])
        } else {
            and([
                eq([ident("resource.project_id"), ident("subject.project_id")]), // TODO: DEPRECATE - Removing PROJECT_ID attribute in favor of TRUST_CONTEXT_ID
                eq([
                    ident("resource.trust_context_id"),
                    ident("subject.trust_context_id"),
                ]),
            ])
        };
        let mut env = Env::new();
        env.put("resource.id", str(address.as_str()));
        env.put("action.id", str(actions::HANDLE_MESSAGE.as_str()));
        env.put(
            "resource.project_id",
            str(configuration.clone().project_identifier),
        );
        env.put(
            "resource.trust_context_id",
            str(configuration.clone().trust_context_identifier),
        );
        let abac = Arc::new(AbacAccessControl::new(
            self.identities_repository(),
            rule,
            env,
        ));
        abac
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EnrollerCheck {
    EnrollerOnly,
    AnyMember,
}
