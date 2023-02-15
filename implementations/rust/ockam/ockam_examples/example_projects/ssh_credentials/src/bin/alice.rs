use credential_example::{BOB_LISTENER_ADDRESS, BOB_TCP_ADDRESS, ECHOER};
use ockam::identity::{Identity, IdentityTrait, TrustEveryonePolicy};
use ockam::vault::{SecretAttributes, SecretPersistence, SecretType, SecretVault, Vault};
use ockam::{route, Context, Result, TcpTransport, TCP};
use std::{env, fs};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create();
    let alice = Identity::create(&ctx, &vault).await?;

    let secret_key_path = env::var("SECRET_KEY_PATH").unwrap();
    let secret_key = fs::read_to_string(secret_key_path).unwrap();

    let secret_key = ssh_key::PrivateKey::from_openssh(&secret_key)
        .unwrap()
        .key_data
        .ed25519()
        .unwrap()
        .private
        .clone();

    let secret_key = vault
        .secret_import(
            secret_key.as_ref(),
            SecretAttributes::new(SecretType::Ed25519, SecretPersistence::Ephemeral, 32),
        )
        .await?;

    alice.add_key("SSH".into(), &secret_key).await?;

    let channel = alice
        .create_secure_channel(
            route![(TCP, BOB_TCP_ADDRESS), BOB_LISTENER_ADDRESS],
            TrustEveryonePolicy,
        )
        .await?;

    ctx.send(
        route![channel, ECHOER],
        "Hello, Bob! I'm Alice from github".to_string(),
    )
    .await?;
    let msg = ctx.receive::<String>().await?.take().body();
    println!("Echo back: {}", &msg);

    ctx.stop().await?;

    Ok(())
}
