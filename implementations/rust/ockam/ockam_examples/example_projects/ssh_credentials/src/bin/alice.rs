use credentials_example::{BOB_LISTENER_ADDRESS, BOB_TCP_ADDRESS, ECHOER};
use ockam::{
    route, Context, Entity, Identity, Result, SoftwareVault, TcpTransport, TrustEveryonePolicy,
    VaultSync, TCP,
};
use ockam_vault::OpenSshKeys;
use ockam_core::vault::{SecretAttributes, SecretPersistence, SecretType, SecretVault};
use std::{env, fs};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let mut vault = VaultSync::create(&ctx, SoftwareVault::default()).await?;
    let vault_address = vault.address();
    let mut alice = Entity::create(&ctx, &vault_address).await?;

    let secret_key_path = env::var("SECRET_KEY_PATH").unwrap();
    let secret_key = fs::read_to_string(secret_key_path).unwrap();

    let secret_key = OpenSshKeys::extract_raw_ed25519_secret_key(&secret_key)?;
    let secret_key = vault
        .secret_import(
            &secret_key,
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
