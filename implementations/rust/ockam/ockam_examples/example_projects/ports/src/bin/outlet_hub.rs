use ockam::{route, Context, Identity, Result, TcpTransport, TrustEveryonePolicy, Vault, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let vault = Vault::create(&ctx).await?;
    let mut fabric_machine = Identity::create(&ctx, vault)?;

    let tcp = TcpTransport::create(&ctx).await?;

    let channel = fabric_machine.create_secure_channel(
        route![(TCP, "127.0.0.1:4000"), "secure_channel_listener"],
        TrustEveryonePolicy,
    )?;

    tcp.create_outlet("outlet", "127.0.0.1:22").await?;

    ctx.send(route![channel, "inlet_fabric"], "outlet".to_string())
        .await?;

    let port = ctx.receive::<i32>().await?.take().body();
    println!("Inlet is accessible on port {}", port);

    Ok(())
}
