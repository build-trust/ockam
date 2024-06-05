use std::collections::BTreeMap;
use std::path::Path;

use tracing::info;

use crate::authenticator::credential_issuer::CredentialIssuerWorker;
use crate::authenticator::direct::{AccountAuthorityInfo, DirectAuthenticatorWorker};
use crate::authenticator::enrollment_tokens::{
    EnrollmentTokenAcceptorWorker, EnrollmentTokenIssuerWorker,
};
use crate::authenticator::{
    AuthorityEnrollmentTokenRepository, AuthorityEnrollmentTokenSqlxDatabase, AuthorityMember,
    AuthorityMembersRepository, AuthorityMembersSqlxDatabase,
};
use ockam::identity::utils::now;
use ockam::identity::{
    Identifier, Identities, SecureChannelListenerOptions, SecureChannelSqlxDatabase,
    SecureChannels, TrustEveryonePolicy,
};
use ockam::tcp::{TcpListenerOptions, TcpTransport};
use ockam_core::compat::sync::Arc;
use ockam_core::env::get_env;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Error, Result};
use ockam_node::database::SqlxDatabase;
use ockam_node::Context;

use crate::authority_node::Configuration;
use crate::echoer::Echoer;
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
    members: Arc<dyn AuthorityMembersRepository>,
    tokens: Arc<dyn AuthorityEnrollmentTokenRepository>,
    account_authority: Option<AccountAuthorityInfo>,
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
    pub async fn create(configuration: &Configuration) -> Result<Self> {
        debug!(?configuration, "creating the authority");

        // create the database
        let node_name = "authority";
        let database_path = &configuration.database_path;
        Self::create_ockam_directory_if_necessary(database_path)?;
        let database = SqlxDatabase::create(database_path).await?;
        let members = Arc::new(AuthorityMembersSqlxDatabase::new(database.clone()));
        let tokens = Arc::new(AuthorityEnrollmentTokenSqlxDatabase::new(database.clone()));
        let secure_channel_repository = Arc::new(SecureChannelSqlxDatabase::new(database.clone()));

        Self::bootstrap_repository(members.clone(), configuration).await?;

        let identities = Identities::create_with_node(database, node_name).build();

        let secure_channels =
            SecureChannels::from_identities(identities.clone(), secure_channel_repository);

        let identifier = configuration.identifier();
        info!(identifier=%identifier, "retrieved the authority identifier");
        let account_authority =
            if let Some(change_history) = configuration.account_authority.clone() {
                let acc_authority_identifier = identities
                    .identities_creation()
                    .identities_verification()
                    .import_from_change_history(None, change_history)
                    .await?;
                Some(AccountAuthorityInfo::new(
                    acc_authority_identifier,
                    configuration.project_identifier(),
                    configuration.enforce_admin_checks,
                ))
            } else {
                None
            };
        Ok(Self {
            identifier,
            secure_channels,
            members,
            tokens,
            account_authority,
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
        let options = if let Some(account_authority) = &self.account_authority {
            options.with_authority(account_authority.account_authority().clone())
        } else {
            options
        };
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

        let direct = DirectAuthenticatorWorker::new(
            self.members.clone(),
            self.secure_channels.identities().identities_attributes(),
            self.account_authority.clone(),
        );

        let name = configuration.authenticator_name();
        ctx.flow_controls()
            .add_consumer(name.clone(), secure_channel_flow_control_id);

        ctx.start_worker(name.clone(), direct).await?;

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

        let issuer = EnrollmentTokenIssuerWorker::new(
            self.tokens.clone(),
            self.members.clone(),
            self.secure_channels.identities().identities_attributes(),
            self.account_authority.clone(),
        );
        let acceptor =
            EnrollmentTokenAcceptorWorker::new(self.tokens.clone(), self.members.clone());

        // start an enrollment token issuer with an abac policy checking that
        // the caller is an enroller for the authority project
        let issuer_address: String = DefaultAddress::ENROLLMENT_TOKEN_ISSUER.into();
        ctx.flow_controls()
            .add_consumer(issuer_address.clone(), secure_channel_flow_control_id);

        ctx.start_worker(issuer_address.clone(), issuer).await?;

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
        let ttl = get_env("CREDENTIAL_TTL_SECS")?;

        // create and start a credential issuer worker
        let issuer = CredentialIssuerWorker::new(
            self.members.clone(),
            self.secure_channels.identities().identities_attributes(),
            self.secure_channels.identities().credentials(),
            &self.identifier,
            configuration.project_identifier(),
            ttl,
            self.account_authority.clone(),
            configuration.disable_trust_context_id,
        );

        let address = DefaultAddress::CREDENTIAL_ISSUER.to_string();
        ctx.flow_controls()
            .add_consumer(address.clone(), secure_channel_flow_control_id);

        ctx.start_worker(address.clone(), issuer).await?;

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
                self.members.clone(),
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

    /// Add a member directly to storage, without additional validation
    /// This is used during the authority start-up to add an identity for exporting traces
    pub async fn add_member(
        &self,
        identifier: &Identifier,
        attributes: &BTreeMap<String, String>,
    ) -> Result<()> {
        let attrs = attributes
            .iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
            .collect();

        self.members
            .add_member(AuthorityMember::new(
                identifier.clone(),
                attrs,
                self.identifier.clone(),
                now()?,
                false,
            ))
            .await
    }
}

/// Private Authority functions
impl Authority {
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
    async fn bootstrap_repository(
        members: Arc<dyn AuthorityMembersRepository>,
        configuration: &Configuration,
    ) -> Result<()> {
        members
            .bootstrap_pre_trusted_members(&configuration.trusted_identities)
            .await
    }
}
