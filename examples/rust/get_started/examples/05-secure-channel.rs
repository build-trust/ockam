use ockam::{Context, Result, Route, SecureChannel};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start the echoer worker.
    ctx.start_worker("echoer", Echoer).await?;

    SecureChannel::create_listener(&mut ctx, "secure_channel_listener").await?;
    let channel = SecureChannel::create(&mut ctx, "secure_channel_listener").await?;

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
