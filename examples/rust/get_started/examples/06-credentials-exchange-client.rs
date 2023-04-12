use ockam::authenticated_storage::AuthenticatedAttributeStorage;
use ockam::identity::credential_issuer::{CredentialIssuerApi, CredentialIssuerClient};
use ockam::identity::{Identity, SecureChannelOptions, TrustEveryonePolicy};
use ockam::{route, vault::Vault, Context, MessageSendReceiveOptions, Result, TcpConnectionOptions, TcpTransport};
use ockam_core::flow_control::FlowControls;
use std::sync::Arc;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let flow_controls = FlowControls::default();
    // Initialize the TCP Transport
    let tcp = TcpTransport::create(&ctx).await?;

    // Create an Identity representing the client
    // We preload the client vault with a change history and secret key corresponding to the identity identifier
    // Pe92f183eb4c324804ef4d62962dea94cf095a265d4d28500c34e1a4e0d5ef638
    // which is an identifier known to the credential issuer, with some preset attributes
    let vault = Vault::create();

    // Create an Identity representing the server
    // Load an identity corresponding to the following public identifier
    // Pada09e0f96e56580f6a0cb54f55ecbde6c973db6732e30dfb39b178760aed041
    //
    // We're hard coding this specific identity because its public identifier is known
    // to the credential issuer as a member of the production cluster.
    let change_history = "01dcf392551f796ef1bcb368177e53f9a5875a962f67279259207d24a01e690721000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020a0d205f09cab9a9467591fcee560429aab1215d8136e5c985a6b7dc729e6f08203010140b098463a727454c0e5292390d8f4cbd4dd0cae5db95606832f3d0a138936487e1da1489c40d8a0995fce71cc1948c6bcfd67186467cdd78eab7e95c080141505";
    let secret = "41b6873b20d95567bf958e6bab2808e9157720040882630b1bb37a72f4015cd2";
    let client = Identity::create_identity_with_change_history(&ctx, vault, change_history, secret).await?;
    let store = client.authenticated_storage();

    // Connect with the credential issuer and authenticate using the latest private
    // key of this program's hardcoded identity.
    //
    // The credential issuer already knows the public identifier of this identity
    // as a member of the production cluster so it returns a signed credential
    // attesting to that knowledge.
    let tcp_flow_control_id = flow_controls.generate_id();
    let issuer_tcp_options = TcpConnectionOptions::as_producer(&flow_controls, &tcp_flow_control_id);
    let issuer_connection = tcp.connect("127.0.0.1:5000", issuer_tcp_options).await?;
    let secure_channel_flow_control_id = flow_controls.generate_id();
    let issuer_options = SecureChannelOptions::as_producer(&flow_controls, &secure_channel_flow_control_id)
        .as_consumer(&flow_controls)
        .with_trust_policy(TrustEveryonePolicy);
    let issuer_channel = client
        .create_secure_channel(route![issuer_connection, "secure"], issuer_options)
        .await?;
    let issuer_client = CredentialIssuerClient::new(&ctx, route![issuer_channel]).await?;
    let credential = issuer_client
        .get_credential(
            client.identifier(),
            MessageSendReceiveOptions::new().with_flow_control(&flow_controls),
        )
        .await?
        .unwrap();
    println!("Credential:\n{credential}");

    // Create a secure channel to the node that is running the Echoer service.
    let server_tcp_options = TcpConnectionOptions::as_producer(&flow_controls, &flow_controls.generate_id());
    let server_connection = tcp.connect("127.0.0.1:4000", server_tcp_options).await?;
    let channel_options = SecureChannelOptions::as_producer(&flow_controls, &flow_controls.generate_id())
        .as_consumer(&flow_controls)
        .with_trust_policy(TrustEveryonePolicy);
    let channel = client
        .create_secure_channel(route![server_connection, "secure"], channel_options)
        .await?;

    // Present credentials over the secure channel
    let storage = Arc::new(AuthenticatedAttributeStorage::new(store.clone()));
    let issuer = issuer_client
        .public_identity(MessageSendReceiveOptions::new().with_flow_control(&flow_controls))
        .await?;
    let r = route![channel.clone(), "credentials"];
    client
        .present_credential_mutual(
            r,
            &[issuer],
            storage,
            &credential,
            MessageSendReceiveOptions::new().with_flow_control(&flow_controls),
        )
        .await?;

    // Send a message to the worker at address "echoer".
    // Wait to receive a reply and print it.
    let reply = ctx
        .send_and_receive_extended::<String>(
            route![channel, "echoer"],
            "Hello Ockam!".to_string(),
            MessageSendReceiveOptions::new().with_flow_control(&flow_controls),
        )
        .await?;
    println!("Received: {}", reply); // should print "Hello Ockam!"

    ctx.stop().await
}
