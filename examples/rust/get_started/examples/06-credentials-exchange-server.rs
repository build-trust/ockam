// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.
use hello_ockam::Echoer;
use ockam::abac::{IncomingAbac, OutgoingAbac};
use ockam::identity::{SecureChannelListenerOptions, Vault};
use ockam::tcp::{TcpListenerOptions, TcpTransportExtension};
use ockam::vault::{EdDSACurve25519SecretKey, SigningSecret, SoftwareVaultForSigning};
use ockam::{Context, Node, Result};
use ockam_api::enroll::enrollment::Enrollment;
use ockam_api::nodes::NodeManager;
use ockam_api::DefaultAddress;
use ockam_multiaddr::MultiAddr;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let identity_vault = SoftwareVaultForSigning::create().await?;
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
    let mut vault = Vault::create().await?;
    vault.identity_vault = identity_vault;

    let node = Node::builder().await?.with_vault(vault).build(&ctx).await?;

    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity representing the server
    // Load an identity corresponding to the following public identifier
    // Ife42b412ecdb7fda4421bd5046e33c1017671ce7a320c3342814f0b99df9ab60
    //
    // We're hard coding this specific identity because its public identifier is known
    // to the credential issuer as a member of the production cluster.
    let change_history = hex::decode("81825837830101583285f682008158201d387ce453816d91159740a55e9a62ad3b58be9ecf7ef08760c42c0d885b6c2ef41a654cf9681a7818fc688200815840dc10ba498655dac0ebab81c6e1af45f465408ddd612842f10a6ced53c06d4562117e14d656be85685aa5bfbd5e5ede6f0ecf5eb41c19a5594e7a25b7a42c5c07").unwrap();
    let server = node.import_private_identity(None, &change_history, &secret).await?;

    let issuer_identity = "81825837830101583285f68200815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef4f41a654cf97d1a7818fc7d8200815840650c4c939b96142546559aed99c52b64aa8a2f7b242b46534f7f8d0c5cc083d2c97210b93e9bca990e9cb9301acc2b634ffb80be314025f9adc870713e6fde0d";
    let issuer = node.import_identity_hex(None, issuer_identity).await?;

    // Connect with the credential issuer and authenticate using the latest private
    // key of this program's hardcoded identity.
    //
    // The credential issuer already knows the public identifier of this identity
    // as a member of the production cluster so it returns a signed credential
    // attesting to that knowledge.
    let authority_node = NodeManager::authority_node_client(
        &tcp,
        node.secure_channels().clone(),
        &issuer,
        &MultiAddr::try_from("/dnsaddr/localhost/tcp/5000/secure/api").unwrap(),
        &server,
        None,
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

    // Start an echoer worker that will only accept incoming requests from
    // identities that have authenticated credentials issued by the above credential
    // issuer. These credentials must also attest that requesting identity is
    // a member of the production cluster.
    let tcp_listener_options = TcpListenerOptions::new();
    let sc_listener_options = SecureChannelListenerOptions::new()
        .with_authority(issuer.clone())
        .with_credential(credential)?
        .as_consumer(&tcp_listener_options.spawner_flow_control_id());

    node.flow_controls().add_consumer(
        DefaultAddress::ECHO_SERVICE,
        &sc_listener_options.spawner_flow_control_id(),
    );
    let allow_production_incoming = IncomingAbac::create_name_value(
        node.identities_attributes(),
        Some(issuer.clone()),
        "cluster",
        "production",
    );
    let allow_production_outgoing = OutgoingAbac::create_name_value(
        &ctx,
        node.identities_attributes(),
        Some(issuer),
        "cluster",
        "production",
    )
    .await?;
    node.start_worker_with_access_control(
        DefaultAddress::ECHO_SERVICE,
        Echoer,
        allow_production_incoming,
        allow_production_outgoing,
    )
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
