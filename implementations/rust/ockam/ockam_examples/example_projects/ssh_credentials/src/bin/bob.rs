use credentials_example::{BOB_LISTENER_ADDRESS, BOB_TCP_ADDRESS, ECHOER};
use ockam::{
    Context, Entity, EntityAccessControlBuilder, Result, Routed, SoftwareVault, TcpTransport,
    TrustPublicKeyPolicy, VaultSync, Worker,
};
use ockam_core::AsyncTryClone;
use ockam_vault::OpenSshKeys;
use std::{env, fs};

pub struct Echoer;

#[ockam::worker]
impl Worker for Echoer {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("Address: {}, Received: {}", ctx.address(), msg);

        // Echo the message body back on its return_route.
        ctx.send(msg.return_route(), msg.body()).await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let vault = VaultSync::create(&ctx, SoftwareVault::default()).await?;
    let vault_address = vault.address();

    let access_control = EntityAccessControlBuilder::new_with_any_id();
    ctx.start_worker_with_access_control(ECHOER, Echoer, access_control)
        .await?;

    let mut bob = Entity::create(&ctx, &vault_address).await?;

    let public_key_path = env::var("PUBLIC_KEY_PATH").unwrap();
    let public_key = fs::read_to_string(public_key_path).unwrap();

    let public_key = OpenSshKeys::extract_ed25519_public_key(&public_key).unwrap();

    let trust_policy =
        TrustPublicKeyPolicy::new(public_key, "SSH", bob.async_try_clone().await.unwrap());

    bob.create_secure_channel_listener(BOB_LISTENER_ADDRESS, trust_policy)
        .await?;

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(BOB_TCP_ADDRESS).await?;

    Ok(())
}
