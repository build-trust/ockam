use ockam::{route, Context, Identity, Result, TcpTransport, TrustEveryonePolicy, Vault, TCP};

const OUTLET_NAME: &str = "outlet";

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // get the port argument
    let port = std::env::args()
        .nth(1)
        .expect("provide the local port as the first argument to the binary");

    // start tcp transport
    let tcp = TcpTransport::create(&ctx).await?;

    // create outlet for sending messages to the given port
    tcp.create_outlet(OUTLET_NAME, format!("127.0.0.1:{}", port))
        .await?;

    // create secure channel to the server
    let vault = Vault::create(&ctx).await?;
    let mut me = Identity::create(&ctx, vault)?;
    let route = route![(TCP, "127.0.0.1:8000"), "secure_listener"];
    let secure_channel = me.create_secure_channel(route, TrustEveryonePolicy)?;

    // tell the server that we want to open a new tunnel
    ctx.send(
        route![secure_channel, "connection_broker"],
        OUTLET_NAME.to_string(),
    )
    .await?;
    let new_port = ctx.receive::<u16>().await?;
    println!(
        "Local port {} is now reachable from new port {}",
        port, new_port
    );

    Ok(())
}
