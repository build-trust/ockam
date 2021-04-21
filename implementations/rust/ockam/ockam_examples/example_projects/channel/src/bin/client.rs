use channel_examples::client_worker::Client;
use ockam::{Result, TcpTransport, Vault};

#[ockam::node]
async fn main(ctx: ockam::Context) -> Result<()> {
    let vault_address = Vault::create(&ctx).await?;

    let hub_addr = "40.78.99.34:4000";

    // Create and register a connection worker pair
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub_addr).await?;

    let client = Client::new(hub_addr, "a6a7be76".to_string(), vault_address);

    ctx.start_worker("echo_client", client).await?;

    // Crashes: ctx.stop().await

    Ok(())
}
