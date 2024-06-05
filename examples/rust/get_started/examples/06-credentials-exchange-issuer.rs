use ockam::access_control::AllowAll;
use ockam::access_control::IdentityIdAccessControl;
use ockam::compat::collections::BTreeMap;
use ockam::compat::sync::Arc;
use ockam::identity::utils::now;
use ockam::identity::SecureChannelListenerOptions;
use ockam::identity::{Identifier, Vault};
use ockam::tcp::{TcpListenerOptions, TcpTransportExtension};
use ockam::vault::{EdDSACurve25519SecretKey, SigningSecret, SoftwareVaultForSigning};
use ockam::{Context, Node, Result};
use ockam_api::authenticator::credential_issuer::CredentialIssuerWorker;
use ockam_api::authenticator::{AuthorityMembersRepository, AuthorityMembersSqlxDatabase, PreTrustedIdentity};
use ockam_api::DefaultAddress;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let identity_vault = SoftwareVaultForSigning::create().await?;
    // Import the signing secret key to the Vault
    let secret = identity_vault
        .import_key(SigningSecret::EdDSACurve25519(EdDSACurve25519SecretKey::new(
            hex::decode("0127359911708ef4de9adaaf27c357501473c4a10a5326a69c1f7f874a0cd82e")
                .unwrap()
                .try_into()
                .unwrap(),
        )))
        .await?;

    // Create a default Vault but use the signing vault with our secret in it
    let mut vault = Vault::create().await?;
    vault.identity_vault = identity_vault;

    let node = Node::builder().await?.with_vault(vault).build(&ctx).await?;

    let issuer_identity = hex::decode("81825837830101583285f68200815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef4f41a654cf97d1a7818fc7d8200815840650c4c939b96142546559aed99c52b64aa8a2f7b242b46534f7f8d0c5cc083d2c97210b93e9bca990e9cb9301acc2b634ffb80be314025f9adc870713e6fde0d").unwrap();
    let issuer = node.import_private_identity(None, &issuer_identity, &secret).await?;
    println!("issuer identifier {}", issuer);

    // Tell the credential issuer about a set of public identifiers that are
    // known, in advance, to be members of the production cluster.
    let known_identifiers = vec![
        Identifier::try_from("Ie70dc5545d64724880257acb32b8851e7dd1dd57076838991bc343165df71bfe")?, // Client Identifier
        Identifier::try_from("Ife42b412ecdb7fda4421bd5046e33c1017671ce7a320c3342814f0b99df9ab60")?, // Server Identifier
    ];

    let members = Arc::new(AuthorityMembersSqlxDatabase::create().await?);

    // Tell this credential issuer about the attributes to include in credentials
    // that will be issued to each of the above known_identifiers, after and only
    // if, they authenticate with their corresponding latest private key.
    //
    // Since this issuer knows that the above identifiers are for members of the
    // production cluster, it will issue a credential that attests to the attribute
    // set: [{cluster, production}] for all identifiers in the above list.
    //
    // For a different application this attested attribute set can be different and
    // distinct for each identifier, but for this example we'll keep things simple.
    let credential_issuer = CredentialIssuerWorker::new(
        members.clone(),
        node.identities_attributes(),
        node.credentials(),
        &issuer,
        "test".to_string(),
        None,
        None,
        true,
    );

    let mut pre_trusted_identities = BTreeMap::<Identifier, PreTrustedIdentity>::new();
    let attributes = PreTrustedIdentity::new(
        [(b"cluster".to_vec(), b"production".to_vec())].into(),
        now()?,
        None,
        issuer.clone(),
    );
    for identifier in &known_identifiers {
        pre_trusted_identities.insert(identifier.clone(), attributes.clone());
    }
    members
        .bootstrap_pre_trusted_members(&pre_trusted_identities.into())
        .await?;

    let tcp_listener_options = TcpListenerOptions::new();
    let sc_listener_options =
        SecureChannelListenerOptions::new().as_consumer(&tcp_listener_options.spawner_flow_control_id());
    let sc_listener_flow_control_id = sc_listener_options.spawner_flow_control_id();

    // Start a secure channel listener that only allows channels where the identity
    // at the other end of the channel can authenticate with the latest private key
    // corresponding to one of the above known public identifiers.
    node.create_secure_channel_listener(&issuer, DefaultAddress::SECURE_CHANNEL_LISTENER, sc_listener_options)
        .await?;

    // Start a credential issuer worker that will only accept incoming requests from
    // authenticated secure channels with our known public identifiers.
    let allow_known = IdentityIdAccessControl::new(known_identifiers);
    node.flow_controls()
        .add_consumer(DefaultAddress::CREDENTIAL_ISSUER, &sc_listener_flow_control_id);
    node.start_worker_with_access_control(
        DefaultAddress::CREDENTIAL_ISSUER,
        credential_issuer,
        allow_known,
        AllowAll,
    )
    .await?;

    // Initialize TCP Transport, create a TCP listener, and wait for connections.
    let tcp = node.create_tcp_transport().await?;
    tcp.listen("127.0.0.1:5000", tcp_listener_options).await?;

    // Don't call node.stop() here so this node runs forever.
    println!("issuer started");
    Ok(())
}
