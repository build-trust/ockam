use ockam::{route, Context, RemoteForwarder, Result, Route, TcpTransport, TCP};
use ockam::{Entity, TrustEveryonePolicy, Vault};

async fn inlet_main(ctx: Context, inlet: &str, hub: &str) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create(&ctx)?;
    let mut e = Entity::create(&ctx, &vault)?;

    let forwarding_address =
        std::env::var("OCKAM_FORWARD").expect("no outlet forwarding address set in OCKAM_FORWARD");

    let r = route![(TCP, hub.clone()), forwarding_address.clone()];

    println!(
        "Establishing a secure channel to forwarding address {} on {}",
        forwarding_address.clone(),
        hub.clone()
    );

    let channel = e.create_secure_channel(r, TrustEveryonePolicy)?;

    let route_to_outlet: Route = route![channel, "outlet"];

    println!(
        "Starting Ockam Inlet at {} forwarding to {} on {}",
        inlet, forwarding_address, hub
    );

    tcp.create_inlet(inlet, route_to_outlet).await?;

    Ok(())
}

async fn outlet_main(ctx: Context, hub: &str) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create(&ctx)?;
    let mut e = Entity::create(&ctx, &vault)?;
    e.create_secure_channel_listener("secure_channel_listener", TrustEveryonePolicy)?;

    let outlet = std::env::var("OCKAM_OUT").expect("no outlet target set in OCKAM_OUT");
    tcp.create_outlet("outlet", outlet.clone()).await?;

    let node_in_hub = (TCP, hub);
    let forwarder = RemoteForwarder::create(&ctx, node_in_hub, "secure_channel_listener").await?;
    println!(
        "Started Ockam Outlet at forwarding address {} on {} output to {}",
        forwarder.remote_address(),
        hub,
        outlet
    );
    Ok(())
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let hub = std::env::var("OCKAM_HUB").expect("no hub address set in OCKAM_HUB");

    if let Ok(inlet) = std::env::var("OCKAM_IN") {
        println!("Ockam is running in Inlet mode");
        inlet_main(ctx, &inlet, &hub).await
    } else {
        println!("Ockam is running in Outlet mode");
        outlet_main(ctx, &hub).await
    }
}
