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
    // I23cd53431a11c85ce75892f575718041ff7d1f56
    // which is an identifier known to the credential issuer, with some preset attributes
    //
    // We're hard coding this specific identity because its public identifier is known
    // to the credential issuer as a member of the production cluster.
    let change_history = hex::decode("818258368201583285f68200815820530d1c2e9822433b679a66a60b9c2ed47c370cd0ce51cbe1a7ad847b5835a963f41a651d4a5c1a77e94d5c820081584022a2ec3a3c1b46c0dd90310bc0bed34ddf77dcb01fea0bf738f689fd195e45dfee58bf5e0d9a46f7f2bbb1a737858d4cb669728f0e63f98da87eecfc2cfe8a00").unwrap();
    let client = node.import_private_identity(None, &change_history, &secret).await?;
    println!("issuer identifier {}", client);

    // Connect to the authority node and ask that node to create a
    // credential for the client.
    let issuer_identity = "818258368201583285f68200815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef4f41a651d4a0e1a77e94d0e8200815840c2a890d8282a63f7145e6c931b179df88af6d5d3d055b48e5064921ad5812c740ead074e296ce401d74f71ba8b108e3953ad8b05e481da953be6cc2896575b01";
    let issuer = node.import_identity_hex(None, issuer_identity).await?;

    // The authority node already knows the public identifier of the client
    // as a member of the production cluster so it returns a signed credential
    // attesting to that knowledge.
    let authority_node = NodeManager::authority_node(
        &tcp,
        node.secure_channels().clone(),
        &issuer,
        &MultiAddr::try_from("/dnsaddr/localhost/tcp/5000/secure/api")?,
        &client,
    )
    .await?;
    let credential = authority_node.issue_credential(node.context()).await.unwrap();

    // Verify that the received credential has indeed be signed by the issuer.
    // The issuer identity must be provided out-of-band from a trusted source
    // and match the identity used to start the issuer node
    node.credentials()
        .credentials_verification()
        .verify_credential(Some(&client), &[issuer.clone()], &credential)
        .await?;

    // Create a trust context that will be used to authenticate credential exchanges
    let trust_context = TrustContext::new(
        "trust_context_id".to_string(),
        Some(AuthorityService::new(node.credentials(), issuer.clone(), None)),
    );

    // Create a secure channel to the node that is running the Echoer service.
    let server_connection = tcp.connect("127.0.0.1:4000", TcpConnectionOptions::new()).await?;
    let channel = node
        .create_secure_channel(
            &client,
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
