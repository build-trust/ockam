use std::sync::Arc;

use rand::random;

use ockam_core::flow_control::FlowControlId;
use ockam_core::{route, Address, AllowAll, Result, Route};
use ockam_identity::models::Identifier;
use ockam_identity::{
    secure_channels, SecureChannelListenerOptions, SecureChannelOptions, SecureChannels,
};
use ockam_node::{Context, MessageReceiveOptions};

pub async fn message_should_pass(ctx: &Context, address: &Address) -> Result<()> {
    check_message_flow(ctx, route![address.clone()], true).await
}

pub async fn message_should_not_pass(ctx: &Context, address: &Address) -> Result<()> {
    check_message_flow(ctx, route![address.clone()], false).await
}

async fn check_message_flow(ctx: &Context, route: Route, should_pass: bool) -> Result<()> {
    let address = Address::random_local();
    let mut receiving_ctx = ctx.new_detached(address.clone(), AllowAll, AllowAll)?;

    let msg: [u8; 4] = random();
    let msg = hex::encode(msg);
    ctx.send(route + address, msg.clone()).await?;

    if should_pass {
        let msg_received = receiving_ctx.receive::<String>().await?.into_body()?;
        assert_eq!(msg_received, msg);
    } else {
        let res = receiving_ctx
            .receive_extended::<String>(MessageReceiveOptions::new().with_timeout_secs(1))
            .await;
        assert!(res.is_err(), "Messages should not pass for given route");
    }

    Ok(())
}

pub async fn message_should_pass_with_ctx(
    ctx: &Context,
    address: &Address,
    receiving_ctx: &mut Context,
) -> Result<()> {
    check_message_flow_with_ctx(ctx, address, receiving_ctx, true).await
}

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
        route![address.clone(), receiving_ctx.primary_address().clone()],
        msg.clone(),
    )
    .await?;

    if should_pass {
        let msg_received = receiving_ctx.receive::<String>().await?.into_body()?;
        assert_eq!(msg_received, msg);
    } else {
        let res = receiving_ctx
            .receive_extended::<String>(MessageReceiveOptions::new().with_timeout_secs(1))
            .await;
        assert!(res.is_err(), "Messages should not pass for given route");
    }

    Ok(())
}

pub struct SecureChannelListenerInfo {
    pub identifier: Identifier,
    pub secure_channels: Arc<SecureChannels>,
    pub flow_control_id: FlowControlId,
}

impl SecureChannelListenerInfo {
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

pub async fn create_secure_channel_listener(
    ctx: &Context,
    flow_control_id: &FlowControlId,
) -> Result<SecureChannelListenerInfo> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let identifier = identities_creation.create_identity().await?;
    let options = SecureChannelListenerOptions::new().as_consumer(flow_control_id);
    let listener =
        secure_channels.create_secure_channel_listener(ctx, &identifier, "listener", options)?;

    let info = SecureChannelListenerInfo {
        secure_channels,
        identifier,
        flow_control_id: listener.flow_control_id().clone(),
    };

    Ok(info)
}

pub struct SecureChannelInfo {
    pub secure_channels: Arc<SecureChannels>,
    pub identifier: Identifier,
    pub address: Address,
}

pub async fn create_secure_channel(
    ctx: &Context,
    connection: &Address,
) -> Result<SecureChannelInfo> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let identifier = identities_creation.create_identity().await?;
    let address = secure_channels
        .create_secure_channel(
            ctx,
            &identifier,
            route![connection.clone(), "listener"],
            SecureChannelOptions::new(),
        )
        .await?
        .encryptor_address()
        .clone();

    let info = SecureChannelInfo {
        secure_channels,
        identifier,
        address,
    };

    Ok(info)
}
