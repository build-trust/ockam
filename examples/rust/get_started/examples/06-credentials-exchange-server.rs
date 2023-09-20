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
    // I2c3b0ef15c12fe43d405497fcfc46318da46d0f5
    //
    // We're hard coding this specific identity because its public identifier is known
    // to the credential issuer as a member of the production cluster.
    let change_history = hex::decode("81a201583ba20101025835a40282018158201d387ce453816d91159740a55e9a62ad3b58be9ecf7ef08760c42c0d885b6c2e03f4041a64dd4074051a77a9437402820181584053de69d82c9c4b12476c889b437be1d9d33bd0041655c4836a3a57ac5a67703e7f500af5bacaed291cfd6783d255fe0f0606638577d087a5612bfb4671f2b70a").unwrap();
    let server = node.import_private_identity(&change_history, &secret).await?;

    let issuer_identity = "81a201583ba20101025835a4028201815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef403f4041a64dd404a051a77a9434a0282018158407754214545cda6e7ff49136f67c9c7973ec309ca4087360a9f844aac961f8afe3f579a72c0c9530f3ff210f02b7c5f56e96ce12ee256b01d7628519800723805";
    let issuer = node.import_identity_hex(issuer_identity).await?;

    // Connect with the credential issuer and authenticate using the latest private
    // key of this program's hardcoded identity.
    //
    // The credential issuer already knows the public identifier of this identity
    // as a member of the production cluster so it returns a signed credential
    // attesting to that knowledge.
    let authority_node = NodeManager::authority_node(
        &tcp,
        node.secure_channels().clone(),
        issuer.identifier(),
        &MultiAddr::try_from("/dnsaddr/localhost/tcp/5000/secure/api").unwrap(),
        server.identifier(),
    )
    .await?;
    let credential = authority_node.issue_credential(node.context()).await.unwrap();

    // Verify that the received credential has indeed be signed by the issuer.
    // The issuer identity must be provided out-of-band from a trusted source
    // and match the identity used to start the issuer node
    node.credentials()
        .credentials_verification()
        .verify_credential(Some(server.identifier()), &[issuer.identifier().clone()], &credential)
        .await?;

    // Create a trust context that will be used to authenticate credential exchanges
    let trust_context = TrustContext::new(
        "trust_context_id".to_string(),
        Some(AuthorityService::new(
            node.credentials(),
            issuer.identifier().clone(),
            None,
        )),
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
    node.create_secure_channel_listener(
        server.identifier(),
        DefaultAddress::SECURE_CHANNEL_LISTENER,
        sc_listener_options,
    )
    .await?;

    // Create a TCP listener and wait for incoming connections
    tcp.listen("127.0.0.1:4000", tcp_listener_options).await?;

    // Don't call node.stop() here so this node runs forever.
    println!("server started");
    Ok(())
}
