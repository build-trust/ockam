use ockam::identity::{AuthorityService, SecureChannelOptions, TrustContext, Vault};
use ockam::{route, Context, Result, TcpConnectionOptions};
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
            hex::decode("31FF4E1CD55F17735A633FBAB4B838CF88D1252D164735CB3185A6E315438C2C")
                .unwrap()
                .try_into()
                .unwrap(),
        )))
        .await?;

    // Create a default Vault but use the signing vault with our secret in it
    let mut vault = Vault::create();
    vault.identity_vault = identity_vault;

    let mut node = Node::builder().with_vault(vault).build(&ctx).await?;
    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity representing the client
    // We preload the client vault with a change history and secret key corresponding to the identity identifier
    // I6342c580429b9a0733880bea4fa18f8055871130
    // which is an identifier known to the credential issuer, with some preset attributes
    //
    // We're hard coding this specific identity because its public identifier is known
    // to the credential issuer as a member of the production cluster.
    let change_history = hex::decode("81a201583ba20101025835a4028201815820530d1c2e9822433b679a66a60b9c2ed47c370cd0ce51cbe1a7ad847b5835a96303f4041a64dd4060051a77a94360028201815840042fff8f6c80603fb1cec4a3cf1ff169ee36889d3ed76184fe1dfbd4b692b02892df9525c61c2f1286b829586d13d5abf7d18973141f734d71c1840520d40a0e").unwrap();
    let client = node.import_private_identity(&change_history, &secret).await?;
    println!("issuer identifier {}", client.identifier());

    // Connect to the authority node and ask that node to create a
    // credential for the client.
    let issuer_identity = "81a201583ba20101025835a4028201815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef403f4041a64dd404a051a77a9434a0282018158407754214545cda6e7ff49136f67c9c7973ec309ca4087360a9f844aac961f8afe3f579a72c0c9530f3ff210f02b7c5f56e96ce12ee256b01d7628519800723805";
    let issuer = node.import_identity_hex(issuer_identity).await?;

    // The authority node already knows the public identifier of the client
    // as a member of the production cluster so it returns a signed credential
    // attesting to that knowledge.
    let authority_node = NodeManager::authority_node(
        &tcp,
        node.secure_channels().clone(),
        issuer.identifier(),
        &MultiAddr::try_from("/dnsaddr/localhost/tcp/5000/secure/api")?,
        client.identifier(),
    )
    .await?;
    let credential = authority_node.issue_credential(node.context()).await.unwrap();

    // Verify that the received credential has indeed be signed by the issuer.
    // The issuer identity must be provided out-of-band from a trusted source
    // and match the identity used to start the issuer node
    node.credentials()
        .credentials_verification()
        .verify_credential(Some(client.identifier()), &[issuer.identifier().clone()], &credential)
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

    // Create a secure channel to the node that is running the Echoer service.
    let server_connection = tcp.connect("127.0.0.1:4000", TcpConnectionOptions::new()).await?;
    let channel = node
        .create_secure_channel(
            client.identifier(),
            route![server_connection, DefaultAddress::SECURE_CHANNEL_LISTENER],
            SecureChannelOptions::new()
                .with_trust_context(trust_context)
                .with_credential(credential),
        )
        .await?;

    // Send a message to the worker at address "echoer".
    // Wait to receive a reply and print it.
    let reply = node
        .send_and_receive::<String>(
            route![channel, DefaultAddress::ECHO_SERVICE],
            "Hello Ockam!".to_string(),
        )
        .await?;
    println!("Received: {}", reply); // should print "Hello Ockam!"

    node.stop().await
}
