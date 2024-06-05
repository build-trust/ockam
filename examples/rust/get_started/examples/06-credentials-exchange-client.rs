use ockam::identity::{SecureChannelOptions, Vault};
use ockam::tcp::{TcpConnectionOptions, TcpTransportExtension};
use ockam::vault::{EdDSACurve25519SecretKey, SigningSecret, SoftwareVaultForSigning};
use ockam::{route, Context, Node, Result};
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
            hex::decode("31FF4E1CD55F17735A633FBAB4B838CF88D1252D164735CB3185A6E315438C2C")
                .unwrap()
                .try_into()
                .unwrap(),
        )))
        .await?;

    // Create a default Vault but use the signing vault with our secret in it
    let mut vault = Vault::create().await?;
    vault.identity_vault = identity_vault;

    let mut node = Node::builder().await?.with_vault(vault).build(&ctx).await?;
    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity representing the client
    // We preload the client vault with a change history and secret key corresponding to the identity identifier
    // Ie70dc5545d64724880257acb32b8851e7dd1dd57076838991bc343165df71bfe
    // which is an identifier known to the credential issuer, with some preset attributes
    //
    // We're hard coding this specific identity because its public identifier is known
    // to the credential issuer as a member of the production cluster.
    let change_history = hex::decode("81825837830101583285f68200815820530d1c2e9822433b679a66a60b9c2ed47c370cd0ce51cbe1a7ad847b5835a963f41a654cf98e1a7818fc8e820081584085054457d079a67778f235a90fa1b926d676bad4b1063cec3c1b869950beb01d22f930591897f761c2247938ce1d8871119488db35fb362727748407885a1608").unwrap();
    let client = node.import_private_identity(None, &change_history, &secret).await?;
    println!("issuer identifier {}", client);

    // Connect to the authority node and ask that node to create a
    // credential for the client.
    let issuer_identity = "81825837830101583285f68200815820afbca9cf5d440147450f9f0d0a038a337b3fe5c17086163f2c54509558b62ef4f41a654cf97d1a7818fc7d8200815840650c4c939b96142546559aed99c52b64aa8a2f7b242b46534f7f8d0c5cc083d2c97210b93e9bca990e9cb9301acc2b634ffb80be314025f9adc870713e6fde0d";
    let issuer = node.import_identity_hex(None, issuer_identity).await?;

    // The authority node already knows the public identifier of the client
    // as a member of the production cluster so it returns a signed credential
    // attesting to that knowledge.
    let authority_node = NodeManager::authority_node_client(
        &tcp,
        node.secure_channels().clone(),
        &issuer,
        &MultiAddr::try_from("/dnsaddr/localhost/tcp/5000/secure/api")?,
        &client,
        None,
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

    // Create a secure channel to the node that is running the Echoer service.
    let server_connection = tcp.connect("127.0.0.1:4000", TcpConnectionOptions::new()).await?;
    let channel = node
        .create_secure_channel(
            &client,
            route![server_connection, DefaultAddress::SECURE_CHANNEL_LISTENER],
            SecureChannelOptions::new()
                .with_authority(issuer.clone())
                .with_credential(credential)?,
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
