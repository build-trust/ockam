use ockam::access_control::AllowAll;
use ockam::access_control::IdentityIdAccessControl;
use ockam::identity::SecureChannelListenerOptions;
use ockam::identity::{CredentialsIssuer, Vault};
use ockam::{Context, Result, TcpListenerOptions};
use ockam::{Node, TcpTransportExtension};
use ockam_api::DefaultAddress;
use ockam_vault::{EdDSACurve25519SecretKey, SigningSecret, SoftwareVaultForSigning};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let identity_vault = SoftwareVaultForSigning::create();
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
    let mut vault = Vault::create();
    vault.identity_vault = identity_vault;

    let node = Node::builder().with_vault(vault).build(&ctx).await?;

    let issuer_identity = hex::decode("81a201583ba20101025835a4028201815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef403f4041a64dd404a051a77a9434a0282018158407754214545cda6e7ff49136f67c9c7973ec309ca4087360a9f844aac961f8afe3f579a72c0c9530f3ff210f02b7c5f56e96ce12ee256b01d7628519800723805").unwrap();
    let issuer = node.import_private_identity(&issuer_identity, &secret).await?;
    println!("issuer identifier {}", issuer.identifier());

    // Tell the credential issuer about a set of public identifiers that are
    // known, in advance, to be members of the production cluster.
    let known_identifiers = vec![
        "I6342c580429b9a0733880bea4fa18f8055871130".try_into()?, // Client Identifier
        "I2c3b0ef15c12fe43d405497fcfc46318da46d0f5".try_into()?, // Server Identifier
    ];

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
    let credential_issuer = CredentialsIssuer::new(
        node.identities().repository(),
        node.credentials(),
        issuer.identifier(),
        "trust_context".into(),
    );
    for identifier in known_identifiers.iter() {
        node.identities()
            .repository()
            .put_attribute_value(identifier, b"cluster".to_vec(), b"production".to_vec())
            .await?;
    }

    let tcp_listener_options = TcpListenerOptions::new();
    let sc_listener_options =
        SecureChannelListenerOptions::new().as_consumer(&tcp_listener_options.spawner_flow_control_id());
    let sc_listener_flow_control_id = sc_listener_options.spawner_flow_control_id();

    // Start a secure channel listener that only allows channels where the identity
    // at the other end of the channel can authenticate with the latest private key
    // corresponding to one of the above known public identifiers.
    node.create_secure_channel_listener(
        issuer.identifier(),
        DefaultAddress::SECURE_CHANNEL_LISTENER,
        sc_listener_options,
    )
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
