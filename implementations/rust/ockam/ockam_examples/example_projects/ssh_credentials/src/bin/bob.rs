use credential_example::{BOB_LISTENER_ADDRESS, BOB_TCP_ADDRESS, ECHOER};
use ockam::identity::access_control::IdentityAccessControlBuilder;
use ockam::identity::{Identity, TrustPublicKeyPolicy};
use ockam::vault::{PublicKey, SecretType, Vault};
use ockam::{Context, Result, Routed, TcpTransport, Worker};
use ockam_core::AsyncTryClone;
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
    let vault = Vault::create();

    let access_control = IdentityAccessControlBuilder::new_with_any_id();
    WorkerBuilder::with_access_control(access_control, ECHOER, Echoer)
        .start(ctx)
        .await?;

    let bob = Identity::create(&ctx, vault).await?;

    let public_key_path = env::var("PUBLIC_KEY_PATH").unwrap();
    let public_key = fs::read_to_string(public_key_path).unwrap();

    let public_key = *ssh_key::PublicKey::from_openssh(&public_key)
        .unwrap()
        .key_data
        .ed25519()
        .unwrap();

    let public_key = PublicKey::new(public_key.as_ref().to_vec(), SecretType::Ed25519);

    let trust_policy =
        TrustPublicKeyPolicy::new(public_key, "SSH", bob.async_try_clone().await.unwrap());

    bob.create_secure_channel_listener(BOB_LISTENER_ADDRESS, trust_policy)
        .await?;

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(BOB_TCP_ADDRESS).await?;

    Ok(())
}
