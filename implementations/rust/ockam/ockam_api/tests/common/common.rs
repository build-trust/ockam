use ockam::identity::utils::now;
use ockam::identity::{Identifier, SecureChannels, SecureClient};
use ockam_api::authenticator::direct::{
    OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY,
};
use ockam_api::authenticator::PreTrustedIdentity;
use ockam_api::authority_node;
use ockam_api::authority_node::{Authority, Configuration};
use ockam_api::cloud::{AuthorityNodeClient, HasSecureClient};
use ockam_api::config::lookup::InternetAddress;
use ockam_api::nodes::NodeManager;
use ockam_core::Result;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use tempfile::NamedTempFile;

// Default Configuration with fake TrustedIdentifier (which can be changed after the call),
// with freshly created Authority Identifier and temporary files for storage and vault
pub async fn default_configuration() -> Result<Configuration> {
    let database_path = NamedTempFile::new().unwrap().keep().unwrap().1;

    let port = thread_rng().gen_range(10000..65535);

    let mut configuration = authority_node::Configuration {
        identifier: "I4dba4b2e53b2ed95967b3bab350b6c9ad9c624e5a1b2c3d4e5f6a6b5c4d3e2f1"
            .try_into()?,
        database_path,
        project_identifier: "123456".to_string(),
        tcp_listener_address: InternetAddress::new(&format!("127.0.0.1:{}", port)).unwrap(),
        secure_channel_listener_name: None,
        authenticator_name: None,
        trusted_identities: Default::default(),
        no_direct_authentication: true,
        no_token_enrollment: true,
        okta: None,
    };

    // Hack to create Authority Identity using the same vault and storage
    let authority_sc_temp = Authority::create(&configuration).await?.secure_channels();

    let authority_identifier = authority_sc_temp
        .identities()
        .identities_creation()
        .create_identity()
        .await?;

    configuration.identifier = authority_identifier;

    Ok(configuration)
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
    use ockam_core::compat::collections::BTreeMap;
    let now = now()?;

    let mut admin_ids = vec![];

    let mut trusted_identities = BTreeMap::<Identifier, PreTrustedIdentity>::new();

    let mut attrs = BTreeMap::<Vec<u8>, Vec<u8>>::new();
    attrs.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.as_bytes().to_vec(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.as_bytes().to_vec(),
    );

    let mut configuration = default_configuration().await?;

    for _ in 0..number_of_admins {
        let admin = secure_channels
            .identities()
            .identities_creation()
            .create_identity()
            .await?;

        let entry =
            PreTrustedIdentity::new(attrs.clone(), now, None, configuration.identifier.clone());
        trusted_identities.insert(admin.clone(), entry);

        admin_ids.push(admin);
    }

    configuration.no_direct_authentication = false;
    configuration.no_token_enrollment = false;

    configuration.trusted_identities = trusted_identities.into();

    authority_node::start_node(ctx, &configuration).await?;

    let mut admins = vec![];
    for admin_id in admin_ids {
        let authority_node_client = NodeManager::authority_node_client(
            &TcpTransport::create(ctx).await?,
            secure_channels.clone(),
            &configuration.identifier,
            &MultiAddr::try_from("/secure/api")?,
            &admin_id,
        )
        .await?;

        admins.push(AuthorityClient {
            identifier: admin_id,
            client: authority_node_client,
        });
    }

    Ok(AuthorityInfo {
        authority_identifier: configuration.identifier.clone(),
        admins,
    })
}

pub fn change_client_identifier(
    client: &AuthorityNodeClient,
    new_identifier: &Identifier,
) -> AuthorityNodeClient {
    let client = client.get_secure_client();
    let client = SecureClient::new(
        client.secure_channels(),
        client.credential_retriever_creator(),
        client.transport(),
        client.secure_route().clone(),
        client.server_identifier(),
        new_identifier,
        client.secure_channel_timeout(),
        client.request_timeout(),
    );
    AuthorityNodeClient::new(client)
}
