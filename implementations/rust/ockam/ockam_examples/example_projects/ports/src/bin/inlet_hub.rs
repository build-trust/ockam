use ockam::compat::collections::BTreeMap;
use ockam::compat::rand::{thread_rng, Rng};
use ockam::{
    Address, Context, Identity, Result, Routed, TcpTransport, TrustEveryonePolicy, Vault, Worker,
};

struct InletCreatorWorker {
    tcp: TcpTransport,
    used_ports: BTreeMap<i32, Address>,
}

impl InletCreatorWorker {
    pub fn new(tcp: TcpTransport) -> Self {
        InletCreatorWorker {
            tcp,
            used_ports: Default::default(),
        }
    }
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

        let port = {
            let mut rng = thread_rng();
            // TODO: Extend range according to exposed ports range
            rng.gen_range(5000..5001)
        };

        let old_inlet = self.used_ports.remove(&port);

        if let Some(old_inlet) = old_inlet {
            println!("Shutting down inlet on port {}", port);
            self.tcp.stop_inlet(old_inlet).await?;
        }

        let address = self
            .tcp
            .create_inlet(format!("0.0.0.0:{}", port), inlet_route)
            .await?;

        println!("Created inlet on port {}", port);

        self.used_ports.insert(port, address);

        ctx.send(return_route, port).await?;

        Ok(())
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let vault = Vault::create(&ctx).await?;
    let mut hub = Identities::create(&ctx, vault)?;

    hub.create_secure_channel_listener("secure_channel_listener", TrustEveryonePolicy)?;

    let tcp = TcpTransport::create(&ctx).await?;

    let fabric_worker = InletCreatorWorker::new(tcp.clone());

    ctx.start_worker("inlet_fabric", fabric_worker).await?;

    tcp.listen("0.0.0.0:4000").await?;

    Ok(())
}
