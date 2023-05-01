use std::sync::Arc;

use rand::random;

use ockam_core::flow_control::{FlowControlId, FlowControlPolicy};
use ockam_core::{route, Address, AllowAll, Result, Route};
use ockam_identity::{
    secure_channels, IdentityIdentifier, SecureChannelListenerOptions, SecureChannelOptions,
    SecureChannels,
};
use ockam_node::{Context, MessageReceiveOptions};

#[allow(dead_code)]
pub async fn message_should_pass(ctx: &Context, address: &Address) -> Result<()> {
    check_message_flow(ctx, route![address.clone()], true).await
}

#[allow(dead_code)]
pub async fn message_should_not_pass(ctx: &Context, address: &Address) -> Result<()> {
    check_message_flow(ctx, route![address.clone()], false).await
}

async fn check_message_flow(ctx: &Context, route: Route, should_pass: bool) -> Result<()> {
    let address = Address::random_local();
    let mut receiving_ctx = ctx
        .new_detached(address.clone(), AllowAll, AllowAll)
        .await?;

    let msg: [u8; 4] = random();
    let msg = hex::encode(msg);
    ctx.send(route![route, address], msg.clone()).await?;

    if should_pass {
        let msg_received = receiving_ctx.receive::<String>().await?.body();
        assert_eq!(msg_received, msg);
    } else {
        let res = receiving_ctx
            .receive_extended::<String>(MessageReceiveOptions::new().with_timeout_secs(1))
            .await;
        assert!(res.is_err(), "Messages should not pass for given route");
    }

    Ok(())
}

#[allow(dead_code)]
pub async fn message_should_pass_with_ctx(
    ctx: &Context,
    address: &Address,
    receiving_ctx: &mut Context,
) -> Result<()> {
    check_message_flow_with_ctx(ctx, address, receiving_ctx, true).await
}

#[allow(dead_code)]
pub async fn message_should_not_pass_with_ctx(
    ctx: &Context,
    address: &Address,
    receiving_ctx: &mut Context,
) -> Result<()> {
    check_message_flow_with_ctx(ctx, address, receiving_ctx, false).await
}

async fn check_message_flow_with_ctx(
    ctx: &Context,
    address: &Address,
    receiving_ctx: &mut Context,
    should_pass: bool,
) -> Result<()> {
    let msg: [u8; 4] = random();
    let msg = hex::encode(msg);
    ctx.send(
        route![address.clone(), receiving_ctx.address()],
        msg.clone(),
    )
    .await?;

    if should_pass {
        let msg_received = receiving_ctx.receive::<String>().await?.body();
        assert_eq!(msg_received, msg);
    } else {
        let res = receiving_ctx
            .receive_extended::<String>(MessageReceiveOptions::new().with_timeout_secs(1))
            .await;
        assert!(res.is_err(), "Messages should not pass for given route");
    }

    Ok(())
}

#[allow(dead_code)]
pub struct SecureChannelListenerInfo {
    pub identifier: IdentityIdentifier,
    pub secure_channels: Arc<SecureChannels>,
    pub flow_control_id: FlowControlId,
}

impl SecureChannelListenerInfo {
    #[allow(dead_code)]
    pub fn get_channel(&self) -> Address {
        self.secure_channels
            .secure_channel_registry()
            .get_channel_list()
            .first()
            .unwrap()
            .encryptor_messaging_address()
            .clone()
    }
}

#[allow(dead_code)]
pub async fn create_secure_channel_listener(
    ctx: &Context,
    flow_control_id: &FlowControlId,
    with_tcp_listener: bool,
) -> Result<SecureChannelListenerInfo> {
    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let identity = identities_creation.create_identity().await?;

    let options = SecureChannelListenerOptions::new();
    let policy = if with_tcp_listener {
        FlowControlPolicy::SpawnerAllowOnlyOneMessage
    } else {
        FlowControlPolicy::ProducerAllowMultiple
    };
    let options = options.as_consumer(flow_control_id, policy);
    let flow_control_id = options.spawner_flow_control_id();

    let identifier = identity.identifier();
    secure_channels
        .create_secure_channel_listener(ctx, &identifier, "listener", options)
        .await?;

    let info = SecureChannelListenerInfo {
        secure_channels,
        identifier,
        flow_control_id,
    };

    Ok(info)
}

#[allow(dead_code)]
pub struct SecureChannelInfo {
    pub secure_channels: Arc<SecureChannels>,
    pub identifier: IdentityIdentifier,
    pub address: Address,
}

#[allow(dead_code)]
pub async fn create_secure_channel(
    ctx: &Context,
    connection: &Address,
) -> Result<SecureChannelInfo> {
    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let identity = identities_creation.create_identity().await?;

    let identifier = identity.identifier();
    let address = secure_channels
        .create_secure_channel(
            ctx,
            &identifier,
            route![connection.clone(), "listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    let info = SecureChannelInfo {
        secure_channels,
        identifier,
        address,
    };

    Ok(info)
}
