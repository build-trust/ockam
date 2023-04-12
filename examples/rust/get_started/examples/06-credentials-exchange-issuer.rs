use ockam::access_control::AllowAll;
use ockam::access_control::IdentityIdAccessControl;
use ockam::identity::credential_issuer::CredentialIssuer;
use ockam::identity::SecureChannelListenerOptions;
use ockam::{Context, Result, TcpListenerOptions, TcpTransport};
use ockam_core::flow_control::{FlowControlPolicy, FlowControls};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let flow_controls = FlowControls::default();

    let issuer = CredentialIssuer::create(&ctx, &flow_controls).await?;
    let issuer_change_history = issuer.identity().change_history().await;
    let exported = issuer_change_history.export().unwrap();
    println!("Credential Issuer Identifier: {}", issuer.identity().identifier());
    println!("Credential Issuer Change History: {}", hex::encode(exported));

    // Tell this credential issuer about a set of public identifiers that are
    // known, in advance, to be members of the production cluster.
    let known_identifiers = vec![
        "Pe92f183eb4c324804ef4d62962dea94cf095a265d4d28500c34e1a4e0d5ef638".try_into()?,
        "Pada09e0f96e56580f6a0cb54f55ecbde6c973db6732e30dfb39b178760aed041".try_into()?,
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
    for identifier in known_identifiers.iter() {
        issuer.put_attribute_value(identifier, "cluster", "production").await?;
    }

    // Start a secure channel listener that only allows channels where the identity
    // at the other end of the channel can authenticate with the latest private key
    // corresponding to one of the above known public identifiers.
    let tcp_flow_control_id = flow_controls.generate_id();
    let secure_channel_flow_control_id = flow_controls.generate_id();
    issuer
        .identity()
        .create_secure_channel_listener(
            "secure",
            SecureChannelListenerOptions::as_spawner(&flow_controls, &secure_channel_flow_control_id)
                .as_consumer_with_flow_control_id(
                    &flow_controls,
                    &tcp_flow_control_id,
                    FlowControlPolicy::SpawnerAllowMultipleMessages,
                ),
        )
        .await?;

    // Start a credential issuer worker that will only accept incoming requests from
    // authenticated secure channels with our known public identifiers.
    let allow_known = IdentityIdAccessControl::new(known_identifiers);
    flow_controls.add_consumer(
        &"issuer".into(),
        &secure_channel_flow_control_id,
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );
    ctx.start_worker("issuer", issuer, allow_known, AllowAll).await?;

    // Initialize TCP Transport, create a TCP listener, and wait for connections.
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(
        "127.0.0.1:5000",
        TcpListenerOptions::as_spawner(&flow_controls, &tcp_flow_control_id),
    )
    .await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
