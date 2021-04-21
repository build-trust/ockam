use ockam::{Context, Result, Route, SecureChannel, Vault};
use ockam_get_started::{Echoer, Hop};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an echoer worker.
    ctx.start_worker("echoer", Echoer).await?;

    // Start hop workers - hop1, hop2, hop3.
    ctx.start_worker("hop1", Hop).await?;
    ctx.start_worker("hop2", Hop).await?;
    ctx.start_worker("hop3", Hop).await?;

    let vault = Vault::create(&ctx).await?;

    SecureChannel::create_listener(&mut ctx, "secure_channel_listener", &vault.clone())
        .await?;

    let route_to_listener = Route::new()
        .append("hop1")
        .append("hop2")
        .append("hop3")
        .append("secure_channel_listener");

    let channel = SecureChannel::create(&mut ctx, route_to_listener, &vault).await?;

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
