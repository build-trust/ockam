use ockam::compat::rand::{thread_rng, Rng};
use ockam::{
    Context, Entity, Result, Routed, SecureChannels, TcpTransport, TrustEveryonePolicy, Vault,
    Worker,
};

struct InletCreatorWorker {
    tcp: TcpTransport,
}

#[ockam::worker]
impl Worker for InletCreatorWorker {
    type Message = String;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();
        let outlet_address = msg.body();

        let mut inlet_route = return_route.clone();
        inlet_route.modify().pop_back().append(outlet_address);

        // let port = {
        //     let mut rng = thread_rng();
        //     rng.gen_range(3000..3005)
        // };
        let port = 5000;

        self.tcp
            .create_inlet(format!("0.0.0.0:{}", port), inlet_route)
            .await?;

        println!("Created inlet on port {}", port);

        ctx.send(return_route, port).await?;

        Ok(())
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    println!("Hello, world!");

    let vault = Vault::create(&ctx)?;
    let mut hub = Entity::create(&ctx, &vault)?;

    hub.create_secure_channel_listener("secure_channel_listener", TrustEveryonePolicy)?;

    let tcp = TcpTransport::create(&ctx).await?;

    let fabric_worker = InletCreatorWorker { tcp: tcp.clone() };

    ctx.start_worker("inlet_fabric", fabric_worker).await?;

    tcp.listen("0.0.0.0:4000").await?;

    Ok(())
}
