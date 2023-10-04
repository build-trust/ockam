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

    let issuer_identity = hex::decode("818258368201583285f68200815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef4f41a651d4a0e1a77e94d0e8200815840c2a890d8282a63f7145e6c931b179df88af6d5d3d055b48e5064921ad5812c740ead074e296ce401d74f71ba8b108e3953ad8b05e481da953be6cc2896575b01").unwrap();
    let issuer = node.import_private_identity(None, &issuer_identity, &secret).await?;
    println!("issuer identifier {}", issuer);

    // Tell the credential issuer about a set of public identifiers that are
    // known, in advance, to be members of the production cluster.
    let known_identifiers = vec![
        "I23cd53431a11c85ce75892f575718041ff7d1f56".try_into()?, // Client Identifier
        "I4eecb209a3f9db547fb552c1e48d8e741d56ebfe".try_into()?, // Server Identifier
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
        &issuer,
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
