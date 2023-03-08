use core::time::Duration;
use ockam::compat::collections::VecDeque;
use ockam::compat::rand::{thread_rng, Rng};
use ockam::{
    route, Address, Context, DelayedEvent, Identity, Result, Routed, TcpTransport,
    TrustEveryonePolicy, Vault, Worker,
};
use std::env;
use tracing::info;

struct TcpInletService {
    tcp: TcpTransport,
    internal_address: Address,
    available_ports: Vec<u32>,
    inlet_registry: VecDeque<(u32, Address)>,
    inlet_ttl_secs: Option<Duration>,
}

impl TcpInletService {
    pub fn new(
        tcp: TcpTransport,
        internal_address: Address,
        available_inlet_ports: Vec<u32>,
        inlet_ttl_secs: Option<Duration>,
    ) -> Self {
        if available_inlet_ports.is_empty() {
            panic!("Ports range is empty");
        }

        let available_ports = available_inlet_ports.clone();

        let ttl_display = if let Some(t) = inlet_ttl_secs {
            t.as_secs() as i32
        } else {
            -1
        };
        info!(
            "Creating tcp_inlet_service with ports: {} to {}; inlet_ttl: {}",
            available_ports.first().unwrap(),
            available_ports.last().unwrap(),
            ttl_display
        );

        TcpInletService {
            tcp,
            internal_address,
            available_ports,
            inlet_registry: Default::default(),
            inlet_ttl_secs,
        }
    }
}

#[ockam::worker]
impl Worker for TcpInletService {
    type Message = String;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        if msg.onward_route().recipient() == self.internal_address {
            let inlet_address = Address::from_string(msg.body());

            if let Some(inlet_index) = self
                .inlet_registry
                .iter()
                .position(|x| x.1 == inlet_address)
            {
                let (port, _) = self.inlet_registry[inlet_index];
                info!("Shutting down inlet on port {} due to timeout", port);
                self.tcp.stop_inlet(inlet_address).await?;
                self.inlet_registry.remove(inlet_index).unwrap();

                self.available_ports.push(port);
            }

            return Ok(());
        }

        let return_route = msg.return_route();
        let outlet_address = msg.body();

        let port = if !self.available_ports.is_empty() {
            let mut rng = thread_rng();
            let index = rng.gen_range(0..self.available_ports.len());

            self.available_ports.remove(index)
        } else {
            let (old_inlet_port, old_inlet_address) = self.inlet_registry.pop_front().unwrap();

            info!(
                "Shutting down inlet on port {} due to lack of available ports",
                old_inlet_port
            );
            self.tcp.stop_inlet(old_inlet_address).await?;

            old_inlet_port
        };

        let mut inlet_route = return_route.clone();
        inlet_route.modify().pop_back().append(outlet_address);

        let address = self
            .tcp
            .create_inlet(format!("0.0.0.0:{}", port), inlet_route)
            .await?;

        info!("Created inlet on port {}", port);

        self.inlet_registry.push_back((port, address.clone()));

        ctx.send(return_route, port).await?;

        if let Some(inlet_ttl) = self.inlet_ttl_secs {
            DelayedEvent::new(
                ctx,
                route![self.internal_address.clone()],
                address.to_string(),
            )?
            .with_duration(inlet_ttl)
            .spawn();
        }

        Ok(())
    }
}

struct Config {
    listening_port: u32,
    available_inlet_port_start: u32,
    available_inlet_port_end: u32,
    inlet_ttl_secs: Option<Duration>,
}

impl Config {
    fn new() -> Self {
        let listening_port = if let Ok(p) = env::var("LISTENING_PORT") {
            p.parse::<u32>().unwrap()
        } else {
            4000
        };

        let available_inlet_port_start = if let Ok(p) = env::var("INLET_PORT_START") {
            p.parse::<u32>().unwrap()
        } else {
            4001
        };

        let available_inlet_port_end = if let Ok(p) = env::var("INLET_PORT_END") {
            p.parse::<u32>().unwrap()
        } else {
            4045
        };

        assert!(
            available_inlet_port_end >= available_inlet_port_start,
            "Invalid port range"
        );

        let inlet_ttl_secs = if let Ok(t) = env::var("INLET_TTL_SECS") {
            let t = t.parse::<i32>().unwrap();

            if t == -1 {
                None
            } else if t < 1 {
                panic!("Invalid inlet ttl");
            } else {
                Some(Duration::new(t as u64, 0))
            }
        } else {
            Some(Duration::new(30 * 60 /* Half hour */, 0))
        };

        Self {
            listening_port,
            available_inlet_port_start,
            available_inlet_port_end,
            inlet_ttl_secs,
        }
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let config = Config::new();

    let vault = Vault::create(&ctx).await?;
    let mut hub = Identities::create(&ctx, vault)?;

    hub.create_secure_channel_listener("secure_channel_listener_service", TrustEveryonePolicy)?;

    let tcp = TcpTransport::create(&ctx).await?;

    let available_inlet_ports =
        (config.available_inlet_port_start..config.available_inlet_port_end + 1).collect();

    let internal_address = Address::random_local();
    let fabric_worker = TcpInletService::new(
        tcp.clone(),
        internal_address.clone(),
        available_inlet_ports,
        config.inlet_ttl_secs,
    );

    ctx.start_worker(
        vec!["tcp_inlet_service".into(), internal_address],
        fabric_worker,
    )
    .await?;

    let listen_addr = format!("0.0.0.0:{}", config.listening_port);
    info!("Listening on {}", listen_addr);
    tcp.listen(listen_addr).await?;

    Ok(())
}
