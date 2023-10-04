// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.
use hello_ockam::Echoer;
use ockam::abac::AbacAccessControl;
use ockam::access_control::AllowAll;
use ockam::identity::{AuthorityService, SecureChannelListenerOptions, TrustContext, Vault};
use ockam::{Context, Result, TcpListenerOptions};
use ockam::{Node, TcpTransportExtension};
use ockam_api::enroll::enrollment::Enrollment;
use ockam_api::nodes::NodeManager;
use ockam_api::DefaultAddress;
use ockam_multiaddr::MultiAddr;
use ockam_vault::{EdDSACurve25519SecretKey, SigningSecret, SoftwareVaultForSigning};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let identity_vault = SoftwareVaultForSigning::create();
    // Import the signing secret key to the Vault
    let secret = identity_vault
        .import_key(SigningSecret::EdDSACurve25519(EdDSACurve25519SecretKey::new(
            hex::decode("5FB3663DF8405379981462BABED7507E3D53A8D061188105E3ADBD70E0A74B8A")
                .unwrap()
                .try_into()
                .unwrap(),
        )))
        .await?;

    // Create a default Vault but use the signing vault with our secret in it
    let mut vault = Vault::create();
    vault.identity_vault = identity_vault;

    let node = Node::builder().with_vault(vault).build(&ctx).await?;

    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity representing the server
    // Load an identity corresponding to the following public identifier
    // I4eecb209a3f9db547fb552c1e48d8e741d56ebfe
    //
    // We're hard coding this specific identity because its public identifier is known
    // to the credential issuer as a member of the production cluster.
    let change_history = hex::decode("818258368201583285f682008158201d387ce453816d91159740a55e9a62ad3b58be9ecf7ef08760c42c0d885b6c2ef41a651d4a3b1a77e94d3b8200815840b2d831d21875fe428c600e071feaa6781393bcba6d0a769d9fe7b1ff0c961399d39f517508662de5806fa57ec1fd5c13e03bd27cd9c0cf047402271edc945808").unwrap();
    let server = node.import_private_identity(None, &change_history, &secret).await?;

    let issuer_identity = "818258368201583285f68200815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef4f41a651d4a0e1a77e94d0e8200815840c2a890d8282a63f7145e6c931b179df88af6d5d3d055b48e5064921ad5812c740ead074e296ce401d74f71ba8b108e3953ad8b05e481da953be6cc2896575b01";
    let issuer = node.import_identity_hex(None, issuer_identity).await?;

    // Connect with the credential issuer and authenticate using the latest private
    // key of this program's hardcoded identity.
    //
    // The credential issuer already knows the public identifier of this identity
    // as a member of the production cluster so it returns a signed credential
    // attesting to that knowledge.
    let authority_node = NodeManager::authority_node(
        &tcp,
        node.secure_channels().clone(),
        &issuer,
        &MultiAddr::try_from("/dnsaddr/localhost/tcp/5000/secure/api").unwrap(),
        &server,
    )
    .await?;
    let credential = authority_node.issue_credential(node.context()).await.unwrap();

    // Verify that the received credential has indeed be signed by the issuer.
    // The issuer identity must be provided out-of-band from a trusted source
    // and match the identity used to start the issuer node
    node.credentials()
        .credentials_verification()
        .verify_credential(Some(&server), &[issuer.clone()], &credential)
        .await?;

    // Create a trust context that will be used to authenticate credential exchanges
    let trust_context = TrustContext::new(
        "trust_context_id".to_string(),
        Some(AuthorityService::new(node.credentials(), issuer.clone(), None)),
    );

    // Start an echoer worker that will only accept incoming requests from
    // identities that have authenticated credentials issued by the above credential
    // issuer. These credentials must also attest that requesting identity is
    // a member of the production cluster.
    let tcp_listener_options = TcpListenerOptions::new();
    let sc_listener_options = SecureChannelListenerOptions::new()
        .with_trust_context(trust_context)
        .with_credential(credential)
        .as_consumer(&tcp_listener_options.spawner_flow_control_id());

    node.flow_controls().add_consumer(
        DefaultAddress::ECHO_SERVICE,
        &sc_listener_options.spawner_flow_control_id(),
    );
    let allow_production = AbacAccessControl::create(node.identities_repository(), "cluster", "production");
    node.start_worker_with_access_control(DefaultAddress::ECHO_SERVICE, Echoer, allow_production, AllowAll)
        .await?;

    // Start a secure channel listener that only allows channels with
    // authenticated identities.
    node.create_secure_channel_listener(&server, DefaultAddress::SECURE_CHANNEL_LISTENER, sc_listener_options)
        .await?;

    // Create a TCP listener and wait for incoming connections
    tcp.listen("127.0.0.1:4000", tcp_listener_options).await?;

    // Don't call node.stop() here so this node runs forever.
    println!("server started");
    Ok(())
}
