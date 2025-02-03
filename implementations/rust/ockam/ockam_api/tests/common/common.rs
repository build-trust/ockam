use core::time::Duration;

use ockam::identity::models::CredentialSchemaIdentifier;
use ockam::identity::utils::AttributesBuilder;
use ockam::identity::{
    CredentialRetrieverCreator, Identifier, MemoryCredentialRetrieverCreator, SecureChannels,
    SecureClient,
};
use ockam_api::authority_node;
use ockam_api::authority_node::{Authority, Configuration};
use ockam_api::config::lookup::InternetAddress;
use ockam_api::nodes::NodeManager;
use ockam_api::orchestrator::{AuthorityNodeClient, HasSecureClient};
use ockam_core::Result;
use ockam_multiaddr::MultiAddr;
use ockam_node::database::DatabaseConfiguration;
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use tempfile::NamedTempFile;
use tracing::debug;

// Default Configuration with fake TrustedIdentifier (which can be changed after the call),
// with freshly created Authority Identifier and temporary files for storage and vault
pub async fn default_configuration() -> Result<Configuration> {
    let database_path = NamedTempFile::new().unwrap().keep().unwrap().1;
    let database_configuration = DatabaseConfiguration::sqlite(database_path.as_path());
    let port = thread_rng().gen_range(10000..65535);

    let mut configuration = create_configuration(
        "I4dba4b2e53b2ed95967b3bab350b6c9ad9c624e5a1b2c3d4e5f6a6b5c4d3e2f1",
        port,
        &database_configuration,
    )?;

    // Hack to create Authority Identity using the same vault and storage
    let authority_sc_temp = Authority::create(&configuration, None)
        .await?
        .secure_channels();

    let authority_identifier = authority_sc_temp
        .identities()
        .identities_creation()
        .create_identity()
        .await?;

    configuration.identifier = authority_identifier;

    Ok(configuration)
}

pub fn create_configuration(
    identifier: &str,
    port: u16,
    database_configuration: &DatabaseConfiguration,
) -> Result<Configuration> {
    Ok(Configuration {
        identifier: identifier.try_into()?,
        database_configuration: database_configuration.clone(),
        project_identifier: "123456".to_string(),
        tcp_listener_address: InternetAddress::new(&format!("127.0.0.1:{}", port)).unwrap(),
        secure_channel_listener_name: None,
        authenticator_name: None,
        trusted_identities: Default::default(),
        no_direct_authentication: true,
        no_token_enrollment: true,
        okta: None,
        account_authority: None,
        enforce_admin_checks: false,
        disable_trust_context_id: false,
    })
}

pub struct AuthorityClient {
    pub identifier: Identifier,
    pub client: AuthorityNodeClient,
}

pub struct AuthorityInfo {
    pub authority_identifier: Identifier,
    pub admins: Vec<AuthorityClient>,
}

// Start an Authority with given number of freshly generated Admins, also instantiate a Client for
// each of the admins
pub async fn start_authority(
    ctx: &Context,
    secure_channels: Arc<SecureChannels>,
    number_of_admins: usize,
) -> Result<AuthorityInfo> {
    let mut configuration = default_configuration().await?;

    let account_authority = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;

    let account_authority_identity = secure_channels
        .identities()
        .get_identity(&account_authority)
        .await?;

    let credentials_creation = secure_channels
        .identities()
        .credentials()
        .credentials_creation();
    let admin_attrs = AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
        .with_attribute("project", configuration.project_identifier.clone())
        .build();
    let mut admins = vec![];
    for _ in 0..number_of_admins {
        let admin = secure_channels
            .identities()
            .identities_creation()
            .create_identity()
            .await?;
        let cred = credentials_creation
            .issue_credential(
                &account_authority,
                &admin,
                admin_attrs.clone(),
                Duration::from_secs(300),
            )
            .await?;

        let authority_node_client = NodeManager::authority_node_client(
            &TcpTransport::create(ctx)?,
            secure_channels.clone(),
            &configuration.identifier,
            &MultiAddr::try_from("/secure/api")?,
            &admin,
            Some(Arc::new(MemoryCredentialRetrieverCreator::new(cred))),
        )
        .await?;
        admins.push(AuthorityClient {
            identifier: admin,
            client: authority_node_client,
        });
    }

    configuration.account_authority = Some(account_authority_identity.change_history().clone());
    configuration.no_direct_authentication = false;
    configuration.no_token_enrollment = false;

    debug!(
        "common.rs about to call authority::start_node with {:?}",
        configuration.account_authority.is_some()
    );
    start_authority_node(ctx, &configuration).await?;

    Ok(AuthorityInfo {
        authority_identifier: configuration.identifier.clone(),
        admins,
    })
}

pub fn change_client_identifier(
    client: &AuthorityNodeClient,
    new_identifier: &Identifier,
    new_credential_retriever_creator: Option<Arc<dyn CredentialRetrieverCreator>>,
) -> AuthorityNodeClient {
    let client = client.get_secure_client();
    let client = SecureClient::new(
        client.secure_channels(),
        new_credential_retriever_creator,
        client.transport(),
        client.secure_route().clone(),
        client.server_identifier(),
        new_identifier,
        client.secure_channel_timeout(),
        client.request_timeout(),
    );
    AuthorityNodeClient::new(client)
}

pub async fn start_authority_node(ctx: &Context, configuration: &Configuration) -> Result<()> {
    let authority = Authority::create(configuration, None).await?;
    authority_node::start_node(ctx, configuration, authority).await
}
