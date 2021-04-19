use ockam::{Context, Result, Route, SecureChannel};
use ockam_get_started::Echoer;
use ockam_vault::SoftwareVault;
use ockam_vault_sync_core::VaultWorker;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start the echoer worker.
    ctx.start_worker("echoer", Echoer).await?;

    let vault_address = VaultWorker::start(&ctx, SoftwareVault::default()).await?;

    SecureChannel::create_listener(&mut ctx, "secure_channel_listener", vault_address.clone())
        .await?;

    let channel = SecureChannel::create(&mut ctx, "secure_channel_listener", vault_address).await?;

    // Send a message to the echoer worker via the channel.
    ctx.send(
        Route::new().append(channel.address()).append("echoer"),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    ctx.stop().await
}
