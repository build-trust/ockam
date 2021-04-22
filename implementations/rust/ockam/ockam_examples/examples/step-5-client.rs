use ockam::{Context, Result, Route, SecureChannel, TcpTransport, Vault, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let remote_node = "Paste the address of the node you created on Ockam Hub here.";
    let secure_channel_forwarded_address =
        "Paste the forwarded address that the server received from registration here.";

    let tcp = TcpTransport::create(&ctx).await?;

    tcp.connect(remote_node).await?;

    let vault_address = Vault::create(&ctx).await?;

    let channel_info = SecureChannel::create(
        &mut ctx,
        Route::new()
            .append_t(TCP, remote_node)
            .append(secure_channel_forwarded_address)
            .append("echo_service"),
        &vault_address,
    )
    .await?;

    ctx.send(
        Route::new()
            .append(channel_info.address().clone())
            .append("echo_service"),
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    println!("Received echo: '{}'", msg);
    ctx.stop().await
}
