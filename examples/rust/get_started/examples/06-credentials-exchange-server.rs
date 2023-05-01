// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.
use hello_ockam::Echoer;
use ockam::abac::AbacAccessControl;
use ockam::access_control::AllowAll;
use ockam::flow_control::FlowControlPolicy;
use ockam::identity::{
    AuthorityService, CredentialsIssuerClient, CredentialsMemoryRetriever, SecureChannelListenerOptions,
    SecureChannelOptions, TrustContext,
};
use ockam::{node, route, Context, Result, TcpConnectionOptions, TcpListenerOptions};
use ockam_transport_tcp::TcpTransportExtension;
use std::sync::Arc;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx);
    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity representing the server
    // Load an identity corresponding to the following public identifier
    // Pe92f183eb4c324804ef4d62962dea94cf095a265d4d28500c34e1a4e0d5ef638
    //
    // We're hard coding this specific identity because its public identifier is known
    // to the credential issuer as a member of the production cluster.
    let change_history = "01ed8a5b1303f975c1296c990d1bd3c1946cfef328de20531e3511ec5604ce0dd9000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020e8c328bc0cc07a374762091d037e69c36fdd4d2e1a651abd4d43a1362d3f800503010140a349968063d7337d0c965969fa9c640824c01a6d37fe130d4ab963b0271b9d5bbf0923faa5e27f15359554f94f08676df01b99d997944e4feaf0caaa1189480e";
    let secret = "5b2b3f2abbd1787704d8f8b363529f8e2d8f423b6dd4b96a2c462e4f0e04ee18";
    let server = node.import_private_identity(change_history, secret).await?;

    // Connect with the credential issuer and authenticate using the latest private
    // key of this program's hardcoded identity.
    //
    // The credential issuer already knows the public identifier of this identity
    // as a member of the production cluster so it returns a signed credential
    // attesting to that knowledge.
    let issuer_connection = tcp.connect("127.0.0.1:5000", TcpConnectionOptions::new()).await?;
    let issuer_channel = node
        .create_secure_channel(
            &server.identifier(),
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
        .verify_credential(&server.identifier(), &[issuer.clone()], credential.clone())
        .await?;

    // Create a trust context that will be used to authenticate credential exchanges
    let trust_context = TrustContext::new(
        "trust_context_id".to_string(),
        Some(AuthorityService::new(
            node.identities().identities_reader(),
            node.credentials(),
            issuer.identifier(),
            Some(Arc::new(CredentialsMemoryRetriever::new(credential))),
        )),
    );

    // Start an echoer worker that will only accept incoming requests from
    // identities that have authenticated credentials issued by the above credential
    // issuer. These credentials must also attest that requesting identity is
    // a member of the production cluster.
    let tcp_listener_options = TcpListenerOptions::new();
    let sc_listener_options = SecureChannelListenerOptions::new().as_consumer(
        &tcp_listener_options.spawner_flow_control_id(),
        FlowControlPolicy::SpawnerAllowOnlyOneMessage,
    );

    node.flow_controls().add_consumer(
        "echoer",
        &sc_listener_options.spawner_flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );
    let allow_production = AbacAccessControl::create(node.repository(), "cluster", "production");
    node.start_worker("echoer", Echoer, allow_production, AllowAll).await?;

    // Start a worker which will receive credentials sent by the client and issued by the issuer node
    node.flow_controls().add_consumer(
        "credentials",
        &sc_listener_options.spawner_flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );
    node.credentials_server()
        .start(
            node.context(),
            trust_context,
            server.identifier(),
            "credentials".into(),
            true,
        )
        .await?;

    // Start a secure channel listener that only allows channels with
    // authenticated identities.
    node.create_secure_channel_listener(&server.identifier(), "secure", sc_listener_options)
        .await?;

    // Create a TCP listener and wait for incoming connections
    tcp.listen("127.0.0.1:4000", tcp_listener_options).await?;

    // Don't call node.stop() here so this node runs forever.
    println!("server started");
    Ok(())
}
