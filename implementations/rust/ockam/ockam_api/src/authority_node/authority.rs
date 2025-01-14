use std::collections::BTreeMap;
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
use ockam_core::flow_control::FlowControlId;
use ockam_core::Result;
use ockam_node::database::SqlxDatabase;
use ockam_node::Context;

use crate::authority_node::Configuration;
use crate::echoer::Echoer;
use crate::nodes::service::default_address::DefaultAddress;

/// This struct represents an Authority, which is an
/// Identity which other identities trust to authenticate attributes
///
/// An Authority is able to start a few services:
//   - a direct authenticator: can add and retrieve members.
//   - a credential issuer: return the attributes of a member as a time-limited credential.
//   - an enrollment token issuer: create a token attributed allowing an identity to acquire some specific attributes.
//   - an enrollment token acceptor: create or update a member, given a token.
#[derive(Clone)]
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
    pub async fn create(
        configuration: &Configuration,
        database: Option<SqlxDatabase>,
    ) -> Result<Self> {
        debug!(?configuration, "creating the authority");

        // create the database
        let node_name = "authority";
        let database = if let Some(database) = database {
            database
        } else {
            SqlxDatabase::create(&configuration.database_configuration).await?
        };

        let members = AuthorityMembersSqlxDatabase::make_repository(database.clone());
        let tokens = AuthorityEnrollmentTokenSqlxDatabase::make_repository(database.clone());
        let secure_channel_repository =
            SecureChannelSqlxDatabase::make_repository(database.clone());

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
        self.secure_channels.create_secure_channel_listener(
            ctx,
            &self.identifier(),
            listener_name.clone(),
            options,
        )?;
        info!("started a secure channel listener with name '{listener_name}'");

        // Create a TCP listener and wait for incoming connections
        let tcp = TcpTransport::create(ctx)?;

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
    pub fn start_direct_authenticator(
        &self,
        ctx: &Context,
        secure_channel_flow_control_id: &FlowControlId,
        configuration: &Configuration,
    ) -> Result<()> {
        if configuration.no_direct_authentication {
            return Ok(());
        }

        let direct = DirectAuthenticatorWorker::new(
            &self.identifier,
            self.members.clone(),
            self.secure_channels.identities().identities_attributes(),
            self.account_authority.clone(),
        );

        let name = configuration.authenticator_name();
        ctx.flow_controls()
            .add_consumer(&name.clone().into(), secure_channel_flow_control_id);

        ctx.start_worker(name.clone(), direct)?;

        info!("started a direct authenticator at '{name}'");
        Ok(())
    }

    /// Start the enrollment services, to issue and accept tokens
    pub fn start_enrollment_services(
        &self,
        ctx: &Context,
        secure_channel_flow_control_id: &FlowControlId,
        configuration: &Configuration,
    ) -> Result<()> {
        if configuration.no_token_enrollment {
            return Ok(());
        }

        let issuer = EnrollmentTokenIssuerWorker::new(
            &self.identifier,
            self.tokens.clone(),
            self.members.clone(),
            self.secure_channels.identities().identities_attributes(),
            self.account_authority.clone(),
        );
        let acceptor = EnrollmentTokenAcceptorWorker::new(
            &self.identifier,
            self.tokens.clone(),
            self.members.clone(),
        );

        // start an enrollment token issuer with an abac policy checking that
        // the caller is an enroller for the authority project
        let issuer_address: String = DefaultAddress::ENROLLMENT_TOKEN_ISSUER.into();
        ctx.flow_controls().add_consumer(
            &issuer_address.clone().into(),
            secure_channel_flow_control_id,
        );

        ctx.start_worker(issuer_address.clone(), issuer)?;

        // start an enrollment token acceptor allowing any incoming message as long as
        // it comes through a secure channel. We accept any message since the purpose of
        // that service is to access a one-time token stating that the sender of the message
        // is a project member
        let acceptor_address: String = DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR.into();
        ctx.flow_controls().add_consumer(
            &acceptor_address.clone().into(),
            secure_channel_flow_control_id,
        );

        ctx.start_worker(acceptor_address.clone(), acceptor)?;

        info!("started an enrollment token issuer at '{issuer_address}'");
        info!("started an enrollment token acceptor at '{acceptor_address}'");
        Ok(())
    }

    /// Start the credential issuer service to issue credentials for a identities
    /// known to the authority
    pub fn start_credential_issuer(
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
            .add_consumer(&address.clone().into(), secure_channel_flow_control_id);

        ctx.start_worker(address.clone(), issuer)?;

        info!("started a credential issuer at '{address}'");
        Ok(())
    }

    /// Start the Okta service to retrieve attributes authenticated by Okta
    pub fn start_okta(
        &self,
        ctx: &Context,
        secure_channel_flow_control_id: &FlowControlId,
        configuration: &Configuration,
    ) -> Result<()> {
        if let Some(okta) = &configuration.okta {
            let okta_worker = crate::okta::Server::new(
                &self.identifier,
                self.members.clone(),
                okta.tenant_base_url(),
                okta.certificate(),
                okta.attributes().as_slice(),
            )?;

            ctx.flow_controls()
                .add_consumer(&okta.address.clone().into(), secure_channel_flow_control_id);

            ctx.start_worker(okta.address.clone(), okta_worker)?;
        }
        Ok(())
    }

    /// Start an echo service
    pub fn start_echo_service(
        &self,
        ctx: &Context,
        secure_channel_flow_control_id: &FlowControlId,
    ) -> Result<()> {
        let address = DefaultAddress::ECHO_SERVICE;

        ctx.flow_controls()
            .add_consumer(&address.into(), secure_channel_flow_control_id);

        ctx.start_worker(address, Echoer)
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
            .add_member(
                &self.identifier,
                AuthorityMember::new(
                    identifier.clone(),
                    attrs,
                    self.identifier.clone(),
                    now()?,
                    false,
                ),
            )
            .await
    }
}

/// Private Authority functions
impl Authority {
    /// Make an identities repository pre-populated with the attributes of some trusted
    /// identities. The values either come from the command line or are read directly from a file
    /// every time we try to retrieve some attributes
    async fn bootstrap_repository(
        members: Arc<dyn AuthorityMembersRepository>,
        configuration: &Configuration,
    ) -> Result<()> {
        members
            .bootstrap_pre_trusted_members(
                &configuration.identifier,
                &configuration.trusted_identities,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authenticator::direct::{
        Members, OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY,
    };
    use crate::authenticator::enrollment_tokens::TokenIssuer;
    use crate::authenticator::one_time_code::OneTimeCode;
    use crate::authenticator::{PreTrustedIdentities, PreTrustedIdentity};
    use crate::authority_node;
    use crate::cloud::AuthorityNodeClient;
    use crate::config::lookup::InternetAddress;
    use crate::enroll::enrollment::{EnrollStatus, Enrollment};
    use crate::nodes::NodeManager;
    use ockam::identity::{identities, secure_channels, TimestampInSeconds};
    use ockam_core::TryClone;
    use ockam_multiaddr::MultiAddr;
    use ockam_node::database::{with_postgres, DatabaseConfiguration};
    use ockam_node::NodeBuilder;
    use std::future::Future;
    use std::net::TcpListener;
    use std::str::FromStr;
    use std::time::Duration;

    /// This test gets a reference to the postgres database and starts 2 authority nodes
    /// to make sure that they can work even when using the same database.
    ///
    /// We test:
    ///
    ///  - That adding a member works with trusted identities
    ///  - Issuing a credential works
    ///  - Issuing and accepting a token works
    #[test]
    fn test_create_two_authority_managed_nodes_using_the_same_postgres_database() {
        let result = execute_test(|db_base, ctx1_base, ctx2_base, ctx_client_base| {
            let db = db_base.clone();
            let ctx1 = ctx1_base.try_clone().unwrap();
            let ctx2 = ctx2_base.try_clone().unwrap();
            let ctx_client = ctx_client_base.try_clone().unwrap();
            async move {
                let port1 = random_port();
                let port2 = random_port();
                let secure_channels: Arc<SecureChannels> = secure_channels().await?;
                let identities_creation = secure_channels.identities().identities_creation();

                let enroller1 = identities_creation.create_identity().await?;
                let enroller2 = identities_creation.create_identity().await?;

                let authority1 = start_authority_node(
                    db.clone(),
                    &ctx1,
                    port1,
                    "authority-node-1",
                    &[enroller1.clone()],
                )
                .await?;
                let authority2 = start_authority_node(
                    db,
                    &ctx2,
                    port2,
                    "authority-node-2",
                    &[enroller2.clone()],
                )
                .await?;

                let client1 = make_authority_node_client(
                    &ctx_client,
                    secure_channels.clone(),
                    &authority1.identifier,
                    &MultiAddr::from_str(&format!("/dnsaddr/127.0.0.1/tcp/{}/secure/api", port1))?,
                    &enroller1,
                )
                .await?;
                let client2 = make_authority_node_client(
                    &ctx_client,
                    secure_channels.clone(),
                    &authority2.identifier,
                    &MultiAddr::from_str(&format!("/dnsaddr/127.0.0.1/tcp/{}/secure/api", port2))?,
                    &enroller2,
                )
                .await?;

                // adding members must work for both authorities
                let identities_creation = secure_channels.identities().identities_creation();
                let member1 = identities_creation.create_identity().await?;
                let member2 = identities_creation.create_identity().await?;

                add_member(&ctx_client, &client1, &member1, ("key1", "value1")).await?;
                add_member(&ctx_client, &client1, &member1, ("key1", "updated_value1")).await?;
                assert_eq!(
                    get_attribute_value(&ctx_client, &client1, &member1, "key1").await?,
                    Some("updated_value1".to_string())
                );

                add_member(&ctx_client, &client2, &member2, ("key1", "value1")).await?;
                add_member(&ctx_client, &client2, &member2, ("key2", "updated_value2")).await?;
                assert_eq!(
                    get_attribute_value(&ctx_client, &client2, &member2, "key2").await?,
                    Some("updated_value2".to_string())
                );

                // issuing credentials must work for both authorities
                issue_credential(&ctx_client, &client1, &member1).await?;
                issue_credential(&ctx_client, &client2, &member2).await?;

                // issuing a token must work for both authorities
                let token1 = create_token(&ctx_client, &client1, &enroller1).await?;
                let token2 = create_token(&ctx_client, &client2, &enroller2).await?;

                // accepting a token must work for both authorities
                let member3 = identities_creation.create_identity().await?;
                let member4 = identities_creation.create_identity().await?;
                let enroll_status1 = accept_token(&ctx_client, &client1, &member3, token1).await?;
                let enroll_status2 = accept_token(&ctx_client, &client2, &member4, token2).await?;

                assert_eq!(enroll_status1, EnrollStatus::EnrolledSuccessfully);
                assert_eq!(enroll_status2, EnrollStatus::EnrolledSuccessfully);
                Ok(())
            }
        });
        result.unwrap()
    }

    /// HELPERS

    /// Create an Authority configuration with:
    ///
    /// - The authority identifier
    /// - A port for the TCP listener (must not clash with another authority port)
    /// - An identity that should be trusted as an enroller
    fn create_configuration(
        authority: &Identifier,
        port: u16,
        trusted: &[Identifier],
    ) -> Result<Configuration> {
        let mut trusted_identities = BTreeMap::new();
        for t in trusted {
            let mut attributes = BTreeMap::new();
            attributes.insert(
                OCKAM_ROLE_ATTRIBUTE_KEY.as_bytes().to_vec(),
                OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.as_bytes().to_vec(),
            );

            trusted_identities.insert(
                t.clone(),
                PreTrustedIdentity::new(attributes, TimestampInSeconds(0), None, authority.clone()),
            );
        }
        Ok(Configuration {
            identifier: authority.clone(),
            database_configuration: DatabaseConfiguration::postgres()?.unwrap(),
            project_identifier: "123456".to_string(),
            tcp_listener_address: InternetAddress::new(&format!("127.0.0.1:{}", port)).unwrap(),
            secure_channel_listener_name: None,
            authenticator_name: None,
            trusted_identities: PreTrustedIdentities::new(trusted_identities),
            no_direct_authentication: false,
            no_token_enrollment: false,
            okta: None,
            account_authority: None,
            enforce_admin_checks: false,
            disable_trust_context_id: false,
        })
    }

    /// Make a client to access the services of an Authority
    async fn make_authority_node_client(
        ctx: &Context,
        secure_channels: Arc<SecureChannels>,
        authority_identifier: &Identifier,
        authority_route: &MultiAddr,
        caller: &Identifier,
    ) -> Result<AuthorityNodeClient> {
        let client = NodeManager::authority_node_client(
            &TcpTransport::create(ctx)?,
            secure_channels,
            authority_identifier,
            authority_route,
            caller,
            None,
        )
        .await?;
        Ok(client
            .with_secure_channel_timeout(&Duration::from_secs(1))
            .with_request_timeout(&Duration::from_secs(1)))
    }

    /// Create and start an authority node, with:
    ///  - A specific TCP listener port
    ///  - A specific node name
    ///  - An identifier for an enroller
    async fn start_authority_node(
        db: SqlxDatabase,
        ctx: &Context,
        port: u16,
        node_name: &str,
        trusted: &[Identifier],
    ) -> Result<Authority> {
        let identities = identities::create(db.clone(), node_name);
        let authority = identities.identities_creation().create_identity().await?;

        let configuration = create_configuration(&authority, port, trusted)?;
        let authority = Authority::create(&configuration, Some(db.clone())).await?;
        authority_node::start_node(ctx, &configuration, authority.clone()).await?;
        Ok(authority)
    }

    /// Add a member
    /// Add a member
    async fn add_member(
        ctx: &Context,
        client: &AuthorityNodeClient,
        member: &Identifier,
        attribute: (&str, &str),
    ) -> Result<()> {
        let mut attributes = BTreeMap::new();
        attributes.insert(attribute.0.to_string(), attribute.1.to_string());
        client
            .add_member(ctx, member.clone(), attributes)
            .await
            .unwrap();
        Ok(())
    }

    /// Get the value of a member attribute if present.
    async fn get_attribute_value(
        ctx: &Context,
        client: &AuthorityNodeClient,
        member: &Identifier,
        attribute_key: &str,
    ) -> Result<Option<String>> {
        let attributes_entry = client.show_member(ctx, member.clone()).await.unwrap();
        Ok(attributes_entry
            .attrs()
            .get(&attribute_key.as_bytes().to_vec())
            .to_owned()
            .map(|v| String::from_utf8(v.clone()).unwrap()))
    }

    /// Issue a credential for a given member
    async fn issue_credential(
        ctx: &Context,
        client: &AuthorityNodeClient,
        member: &Identifier,
    ) -> Result<()> {
        client
            .clone()
            .with_client_identifier(member)
            .issue_credential(ctx)
            .await
            .unwrap();
        Ok(())
    }

    /// Issue a token
    async fn create_token(
        ctx: &Context,
        client: &AuthorityNodeClient,
        enroller: &Identifier,
    ) -> Result<OneTimeCode> {
        let mut attributes = BTreeMap::new();
        attributes.insert("name".to_string(), "value".to_string());
        Ok(client
            .clone()
            .with_client_identifier(enroller)
            .create_token(ctx, attributes, None, None)
            .await
            .unwrap())
    }

    /// Accept a token
    async fn accept_token(
        ctx: &Context,
        client: &AuthorityNodeClient,
        member: &Identifier,
        token: OneTimeCode,
    ) -> Result<EnrollStatus> {
        let mut attributes = BTreeMap::new();
        attributes.insert("name".to_string(), "value".to_string());
        Ok(client
            .clone()
            .with_client_identifier(member)
            .present_token(ctx, &token)
            .await
            .unwrap())
    }

    /// Create 3 contexts representing 3 nodes: 2 authority nodes and a client node.
    /// Then execute some test code using the postgres database and the 3 contexts.
    fn execute_test<F, Fut>(f: F) -> Result<()>
    where
        F: Fn(&SqlxDatabase, &Context, &Context, &Context) -> Fut + Send + Sync + 'static + Clone,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        // skip everything if postgres is not available
        if DatabaseConfiguration::postgres()?.is_some() {
            return Ok(());
        };

        // set logging to true if debugging is needed
        let logging = false;

        // prepare the nodes
        let node_builder1 = NodeBuilder::new().with_logging(logging);
        let (ctx1, mut executor1) = node_builder1.build();
        let node_builder2 = NodeBuilder::new()
            .with_runtime(executor1.get_runtime())
            .with_logging(logging);
        let (ctx2, _executor2) = node_builder2.build();
        let client_node_builder = NodeBuilder::new()
            .with_runtime(executor1.get_runtime())
            .with_logging(logging);
        let (ctx_client, _executor_client) = client_node_builder.build();

        // run the code with the necessary contexts
        executor1.execute(async move {
            // we need to make separate clones to be able to close the contexts after the test.
            let ctx1_handle = ctx1.try_clone().unwrap();
            let ctx2_handle = ctx2.try_clone().unwrap();
            let ctx_client_handle = ctx_client.try_clone().unwrap();

            let result = with_postgres(move |db| {
                let f_clone = f.clone();
                let db_clone = db.clone();
                let ctx1_clone = ctx1.try_clone().unwrap();
                let ctx2_clone = ctx2.try_clone().unwrap();
                let ctx_client_clone = ctx_client.try_clone().unwrap();
                async move { f_clone(&db_clone, &ctx1_clone, &ctx2_clone, &ctx_client_clone).await }
            })
            .await;
            ctx1_handle.shutdown_node().await?;
            ctx2_handle.shutdown_node().await?;
            ctx_client_handle.shutdown_node().await?;
            result
        })?
    }

    fn random_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to address");
        let address = listener.local_addr().expect("Failed to get local address");
        address.port()
    }
}
