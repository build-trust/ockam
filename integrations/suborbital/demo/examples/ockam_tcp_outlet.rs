use ockam::{Context, RemoteForwarder, Result, TcpTransport, TCP};
use ockam::{Identity, TrustEveryonePolicy, Vault};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let cloud_address = std::env::args().nth(1).expect("no cloud address given");
    let alias = std::env::args().nth(2).expect("no alias given");
    let outlet_target = std::env::args().nth(3).expect("no outlet target given");

    let tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create();
    let mut e = Identity::create(&ctx, &vault).await?;

    e.create_secure_channel_listener("secure_channel_listener", TrustEveryonePolicy)
        .await?;

    tcp.create_outlet("outlet", outlet_target).await?;

    let _ = RemoteForwarder::create_static(&ctx, (TCP, cloud_address), alias).await?;

    Ok(())
}
