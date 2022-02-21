use ockam::{route, Context, Result, TcpTransport, TCP};
use ockam::{Identity, TrustEveryonePolicy, Vault};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let cloud_address = std::env::args().nth(1).expect("no cloud address given");
    let alias = std::env::args().nth(2).expect("no alias given");
    let inlet_address = std::env::args().nth(3).expect("no inlet address given");

    let tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create();
    let mut e = Identity::create(&ctx, &vault).await?;

    let channel = e
        .create_secure_channel(
            route![
                (TCP, cloud_address),
                format!("forward_to_{}", alias),
                "secure_channel_listener"
            ],
            TrustEveryonePolicy,
            // TODO: TrustIdentifierPolicy::new()
        )
        .await?;

    tcp.create_inlet(inlet_address, route![channel, "outlet"]).await?;

    Ok(())
}
