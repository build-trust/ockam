use std::path::Path;

use tracing::info;

use ockam::identity::{
    CredentialsIssuer, Identifier, Identities, IdentityAttributesRepository,
    IdentityAttributesSqlxDatabase, SecureChannelListenerOptions, SecureChannels,
    TrustEveryonePolicy,
};
use ockam_abac::expr::{and, eq, ident, str};
use ockam_abac::{AbacAccessControl, Env, Policy};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Error, Result, Worker};
use ockam_node::database::SqlxDatabase;
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_tcp::{TcpListenerOptions, TcpTransport};

use crate::authenticator::enrollment_tokens::EnrollmentTokenAuthenticator;
use crate::authority_node::authority::EnrollerCheck::{AnyMember, EnrollerOnly};
use crate::authority_node::Configuration;
use crate::bootstrapped_identities_store::BootstrapedIdentityAttributesStore;
use crate::echoer::Echoer;
use crate::nodes::service::actions;
use crate::nodes::service::default_address::DefaultAddress;

/// This struct represents an Authority, which is an
/// Identity which other identities trust to authenticate attributes
/// An Authority is able to start a few services
//   - a direct authenticator
//   - a credential issuer
//   - an enrollment token issuer
//   - an enrollment token acceptor
pub struct Authority {
    identifier: Identifier,
    secure_channels: Arc<SecureChannels>,
}

/// Public functions to:
///   - create an Authority
///   - start services
impl Authority {
    /// Return the identity identifier for this authority
    pub fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }

    /// SecureChannels getter
    pub fn secure_channels(&self) -> Arc<SecureChannels> {
        self.secure_channels.clone()
    }

    /// Create an identity for an authority from the configured public identity and configured vault
    /// The list of trusted identities in the configuration is used to pre-populate an attributes storage
    /// In practice it contains the list of identities with the ockam-role attribute set as 'enroller'
    pub async fn create(configuration: &Configuration) -> Result<Authority> {
        debug!(?configuration, "creating the authority");

        // create the database
        let database_path = &configuration.database_path;
        Self::create_ockam_directory_if_necessary(database_path)?;
        let database = Arc::new(SqlxDatabase::create(database_path).await?);

        // create the bootstrapped identity attributes repository
        let identity_attributes_repository = Self::bootstrap_repository(
            Arc::new(IdentityAttributesSqlxDatabase::new(database.clone())),
            configuration,
        );

        let identities = Identities::create(database.clone())
            .with_identity_attributes_repository(identity_attributes_repository)
            .build();

        let secure_channels = SecureChannels::builder()
            .await?
            .with_identities(identities)
            .build();

        let identifier = configuration.identifier();
        info!(identifier=%identifier, "retrieved the authority identifier");

        Ok(Authority {
            identifier,
            secure_channels,
        })
    }

    /// Start the secure channel listener service, using TCP as a transport
    /// The TCP listener is connected to the secure channel listener so that it can only
    /// be used to create secure channels.
    pub async fn start_secure_channel_listener(
        &self,
        ctx: &Context,
        configuration: &Configuration,
    ) -> Result<FlowControlId> {
        // Start a secure channel listener that only allows channels with
        // authenticated identities.
        let tcp_listener_options = TcpListenerOptions::new();
        let tcp_listener_flow_control_id = tcp_listener_options.spawner_flow_control_id().clone();

        let options = SecureChannelListenerOptions::new()
            .with_trust_policy(TrustEveryonePolicy)
            .as_consumer(&tcp_listener_flow_control_id);
        let secure_channel_listener_flow_control_id = options.spawner_flow_control_id().clone();

        let listener_name = configuration.secure_channel_listener_name();
        self.secure_channels
            .create_secure_channel_listener(ctx, &self.identifier(), listener_name.clone(), options)
            .await?;
        info!("started a secure channel listener with name '{listener_name}'");

        // Create a TCP listener and wait for incoming connections
        let tcp = TcpTransport::create(ctx).await?;

        let listener = tcp
            .listen(
                configuration.tcp_listener_address().to_string(),
                tcp_listener_options,
            )
            .await?;

        info!("started a TCP listener at {listener:?}");
        Ok(secure_channel_listener_flow_control_id)
    }

    /// Start the authenticator service to enroll project members
    pub async fn start_direct_authenticator(
        &self,
        ctx: &Context,
        secure_channel_flow_control_id: &FlowControlId,
        configuration: &Configuration,
    ) -> Result<()> {
        if configuration.no_direct_authentication {
            return Ok(());
        }

        let direct = crate::authenticator::direct::DirectAuthenticator::new(
            configuration.project_identifier(),
            self.identity_attributes_repository(),
        )
        .await?;

        let name = configuration.authenticator_name();
        ctx.flow_controls()
            .add_consumer(name.clone(), secure_channel_flow_control_id);

        self.start(ctx, configuration, name.clone(), EnrollerOnly, direct)
            .await?;

        info!("started a direct authenticator at '{name}'");
        Ok(())
    }

    /// Start the enrollment services, to issue and accept tokens
    pub async fn start_enrollment_services(
        &self,
        ctx: &Context,
        secure_channel_flow_control_id: &FlowControlId,
        configuration: &Configuration,
    ) -> Result<()> {
        if configuration.no_token_enrollment {
            return Ok(());
        }

        let (issuer, acceptor) = EnrollmentTokenAuthenticator::new_worker_pair(
            configuration.project_identifier(),
            self.identity_attributes_repository(),
        );

        // start an enrollment token issuer with an abac policy checking that
        // the caller is an enroller for the authority project
        let issuer_address: String = DefaultAddress::ENROLLMENT_TOKEN_ISSUER.into();
        ctx.flow_controls()
            .add_consumer(issuer_address.clone(), secure_channel_flow_control_id);

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
        ctx.flow_controls()
            .add_consumer(acceptor_address.clone(), secure_channel_flow_control_id);

        ctx.start_worker(acceptor_address.clone(), acceptor).await?;

        info!("started an enrollment token issuer at '{issuer_address}'");
        info!("started an enrollment token acceptor at '{acceptor_address}'");
        Ok(())
    }

    /// Start the credential issuer service to issue credentials for a identities
    /// known to the authority
    pub async fn start_credential_issuer(
        &self,
        ctx: &Context,
        secure_channel_flow_control_id: &FlowControlId,
        configuration: &Configuration,
    ) -> Result<()> {
        // create and start a credential issuer worker
        let issuer = CredentialsIssuer::new(
            self.secure_channels
                .identities()
                .identity_attributes_repository(),
            self.secure_channels.identities().credentials(),
            &self.identifier,
            configuration.project_identifier(),
        );

        let address = DefaultAddress::CREDENTIAL_ISSUER.to_string();
        ctx.flow_controls()
            .add_consumer(address.clone(), secure_channel_flow_control_id);

        self.start(ctx, configuration, address.clone(), AnyMember, issuer)
            .await?;

        info!("started a credential issuer at '{address}'");
        Ok(())
    }

    /// Start the Okta service to retrieve attributes authenticated by Okta
    pub async fn start_okta(
        &self,
        ctx: &Context,
        secure_channel_flow_control_id: &FlowControlId,
        configuration: &Configuration,
    ) -> Result<()> {
        if let Some(okta) = &configuration.okta {
            let okta_worker = crate::okta::Server::new(
                self.identity_attributes_repository(),
                configuration.project_identifier(),
                okta.tenant_base_url(),
                okta.certificate(),
                okta.attributes().as_slice(),
            )?;

            ctx.flow_controls()
                .add_consumer(okta.address.clone(), secure_channel_flow_control_id);

            ctx.start_worker(okta.address.clone(), okta_worker).await?;
        }
        Ok(())
    }

    /// Start an echo service
    pub async fn start_echo_service(
        &self,
        ctx: &Context,
        secure_channel_flow_control_id: &FlowControlId,
    ) -> Result<()> {
        let address = DefaultAddress::ECHO_SERVICE;

        ctx.flow_controls()
            .add_consumer(address, secure_channel_flow_control_id);

        ctx.start_worker(address, Echoer).await
    }
}

/// Private Authority functions
impl Authority {
    /// Return the identities storage used by the authority
    fn identities(&self) -> Arc<Identities> {
        self.secure_channels.identities()
    }

    /// Return the identity attributes repository used by the authority
    fn identity_attributes_repository(&self) -> Arc<dyn IdentityAttributesRepository> {
        self.identities().identity_attributes_repository().clone()
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
        repository: Arc<dyn IdentityAttributesRepository>,
        configuration: &Configuration,
    ) -> Arc<dyn IdentityAttributesRepository> {
        let trusted_identities = &configuration.trusted_identities;
        Arc::new(BootstrapedIdentityAttributesStore::new(
            Arc::new(trusted_identities.clone()),
            repository.clone(),
        ))
    }

    /// Start a worker at a given address
    /// The configuration is used to create an Abac incoming policy checking that
    /// the sender can indeed call the authority services
    async fn start<W>(
        &self,
        ctx: &Context,
        configuration: &Configuration,
        address: String,
        enroller_check: EnrollerCheck,
        worker: W,
    ) -> Result<()>
    where
        W: Worker<Context = Context>,
    {
        let abac = self.create_abac_policy(configuration, address.clone(), enroller_check);
        WorkerBuilder::new(worker)
            .with_address(address)
            .with_incoming_access_control_arc(abac)
            .start(ctx)
            .await
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
        let rule = match enroller_check {
            EnrollerOnly => and([
                eq([
                    ident("resource.trust_context_id"),
                    ident("subject.trust_context_id"),
                ]),
                eq([ident("subject.ockam-role"), str("enroller")]),
            ]),
            AnyMember => eq([
                ident("resource.trust_context_id"),
                ident("subject.trust_context_id"),
            ]),
        };

        let mut env = Env::new();
        env.put("resource.id", str(address.as_str()));
        env.put("action.id", str(actions::HANDLE_MESSAGE.as_str()));
        env.put(
            "resource.trust_context_id",
            str(configuration.project_identifier.clone()),
        );
        let abac = Arc::new(AbacAccessControl::new(
            self.identity_attributes_repository(),
            Policy::new(rule),
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
