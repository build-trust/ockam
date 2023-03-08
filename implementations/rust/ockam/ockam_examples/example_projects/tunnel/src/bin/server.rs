use ockam::{Context, Identity, Result, Routed, TcpTransport, TrustEveryonePolicy, Vault, Worker};
use rand::{thread_rng, Rng};

struct ConnectionBrokerWorker {
    tcp: TcpTransport,
    ports: Vec<u16>,
}

impl ConnectionBrokerWorker {
    pub fn new(tcp: TcpTransport) -> Self {
        Self {
            tcp,
            ports: (4000..4100).collect(),
        }
    }
}

#[ockam::worker]
impl Worker for ConnectionBrokerWorker {
    type Message = String;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();

        // the incoming traffic from the inlet will be redirected
        // to this address
        let inlet_dst = {
            let mut route = return_route.clone();
            route.modify().pop_back().append(msg.as_body().as_str());
            route
        };

        // find a free port
        let port = {
            if self.ports.is_empty() {
                println!("Ran out of free ports!");
                return Ok(());
            }

            let idx = thread_rng().gen_range(0..self.ports.len());
            self.ports.remove(idx)
        };
        let addr = format!("0.0.0.0:{}", port);

        // create the inlet
        self.tcp.create_inlet(addr, inlet_dst).await?;

        println!("Created new tunnel from port {} to {}", port, msg.body());

        // answer with the port that we are listening to
        ctx.send(return_route, port).await?;

        Ok(())
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // create a secure listening channel
    let vault = Vault::create(&ctx).await?;
    let mut me = Identity::create(&ctx, vault)?;
    me.create_secure_channel_listener("secure_listener", TrustEveryonePolicy)?;

    // start listening over TCP and start worker
    let tcp = TcpTransport::create(&ctx).await?;
    ctx.start_worker(
        "connection_broker",
        ConnectionBrokerWorker::new(tcp.clone()),
    )
    .await?;
    tcp.listen("0.0.0.0:8000").await?;

    // we don't call `ctx.stop()` because we want this node to run forever
    // so it can wait for new connections
    Ok(())
}
