use ockam_channel::SecureChannel;
use ockam_core::compat::string::{String, ToString};
use ockam_core::compat::sync::Arc;
use ockam_core::{route, AsyncTryClone, LocalDestinationOnly, Mailboxes, Result, Routed, Worker};
use ockam_key_exchange_core::NewKeyExchanger;
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::access_control::LocalOriginOnly;
use ockam_node::{Context, WorkerBuilder};
use ockam_vault::Vault;

pub struct Echoer;

#[ockam_core::worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        ctx.send(msg.return_route(), msg.body()).await
    }
}

#[ockam_macros::test]
async fn simplest_channel(ctx: &mut Context) -> Result<()> {
    WorkerBuilder::with_mailboxes(
        Mailboxes::main(
            "echoer",
            Arc::new(LocalOriginOnly),
            Arc::new(LocalDestinationOnly),
        ),
        Echoer,
    )
    .start(ctx)
    .await?;

    let vault = Vault::create();
    let new_key_exchanger = XXNewKeyExchanger::new(vault.async_try_clone().await?);
    SecureChannel::create_listener_extended(
        ctx,
        "secure_channel_listener".to_string(),
        new_key_exchanger.async_try_clone().await?,
        vault.async_try_clone().await?,
    )
    .await?;
    let initiator = SecureChannel::create_extended(
        ctx,
        route!["secure_channel_listener"],
        None,
        new_key_exchanger.initiator().await?,
        vault,
    )
    .await?;

    let test_msg = "Hello, channel".to_string();
    let reply: String = ctx
        .send_and_receive(route![initiator.address(), "echoer"], test_msg.clone())
        .await?;
    assert_eq!(reply, test_msg);
    ctx.stop().await
}
