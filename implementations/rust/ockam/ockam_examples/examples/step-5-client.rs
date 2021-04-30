use ockam::{
    Context, Result, Route, SecureChannel, TcpTransport, Vault, VaultSync, XXNewKeyExchanger, TCP,
};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let remote_node = "40.78.99.34:4000";
    let secure_channel_forwarded_address = "f8ff78d3";

    let tcp = TcpTransport::create(&ctx).await?;

    tcp.connect(remote_node).await?;

    let vault_address = Vault::create(&ctx)?;

    let vault_sync = VaultSync::create_with_worker(&ctx, &vault_address, "FIXME").unwrap();
    let xx_key_exchanger = XXNewKeyExchanger::new(vault_sync.clone());

    let channel_info = SecureChannel::create(
        &mut ctx,
        Route::new()
            .append_t(TCP, remote_node)
            .append(secure_channel_forwarded_address)
            .append("echo_service"),
        None,
        &xx_key_exchanger,
        vault_sync,
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
