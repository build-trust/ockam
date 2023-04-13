use ockam::identity::{CredentialsIssuerClient, SecureChannelOptions};
use ockam::{node, route, Context, Result, TcpConnectionOptions};
use ockam_transport_tcp::TcpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx);
    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity representing the client
    // We preload the client vault with a change history and secret key corresponding to the identity identifier
    // Pe92f183eb4c324804ef4d62962dea94cf095a265d4d28500c34e1a4e0d5ef638
    // which is an identifier known to the credential issuer, with some preset attributes
    // We're hard coding this specific identity because its public identifier is known
    // to the credential issuer as a member of the production cluster.
    let change_history = "01dcf392551f796ef1bcb368177e53f9a5875a962f67279259207d24a01e690721000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020a0d205f09cab9a9467591fcee560429aab1215d8136e5c985a6b7dc729e6f08203010140b098463a727454c0e5292390d8f4cbd4dd0cae5db95606832f3d0a138936487e1da1489c40d8a0995fce71cc1948c6bcfd67186467cdd78eab7e95c080141505";
    let secret = "41b6873b20d95567bf958e6bab2808e9157720040882630b1bb37a72f4015cd2";
    let client = node.import_private_identity(change_history, secret).await?;

    // Connect with the credential issuer and authenticate using the latest private
    // key of this program's hardcoded identity.
    //
    // The credential issuer already knows the public identifier of this identity
    // as a member of the production cluster so it returns a signed credential
    // attesting to that knowledge.
    let issuer_connection = tcp.connect("127.0.0.1:5000", TcpConnectionOptions::new()).await?;
    let issuer_channel = node
        .create_secure_channel(
            &client.identifier(),
            route![issuer_connection, "secure"],
            SecureChannelOptions::new(),
        )
        .await?;

    let issuer_client = CredentialsIssuerClient::new(route![issuer_channel, "issuer"], node.context()).await?;
    let credential = issuer_client.credential().await?;
    println!("Credential:\n{credential}");

    // Verify that the received credential has indeed be signed by the issuer.
    // The issuer identity must be provided out-of-band from a trusted source
    // and match the identity used to start the issuer node
    let issuer_identity = "0180370b91c5d0aa4af34580a9ab4b8fb2a28351bed061525c96b4f07e75c0ee18000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020236f79490d3f683e0c3bf458a7381c366c99a8f2b2ac406db1ef8c130111f12703010140b23fddceb11cea25602aa681b6ef6abda036722c27a6dee291f1d6b2234a127af21cc79de2252201f27e7e34e0bf5064adbf3d01eb355aff4bf5c90b8f1fd80a";
    let issuer = node.import_identity_hex(issuer_identity).await?;
    node.credentials()
        .verify_credential(&client.identifier(), &[issuer.clone()], credential.clone())
        .await?;

    // Create a secure channel to the node that is running the Echoer service.
    let server_connection = tcp.connect("127.0.0.1:4000", TcpConnectionOptions::new()).await?;
    let channel = node
        .create_secure_channel(
            &client.identifier(),
            route![server_connection, "secure"],
            SecureChannelOptions::new(),
        )
        .await?;

    // Present credentials over the secure channel
    let r = route![channel.clone(), "credentials"];
    node.credentials_server()
        .present_credential_mutual(node.context(), r, &[issuer], credential)
        .await?;

    // Send a message to the worker at address "echoer".
    // Wait to receive a reply and print it.
    let reply = node
        .send_and_receive::<String>(route![channel, "echoer"], "Hello Ockam!".to_string())
        .await?;
    println!("Received: {}", reply); // should print "Hello Ockam!"

    node.stop().await
}
