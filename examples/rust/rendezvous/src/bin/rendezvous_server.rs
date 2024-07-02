use ockam::errcode::{Kind, Origin};
use ockam::transport::{parse_socket_addr, HostnamePort};
use ockam::udp::{RendezvousService, UdpBindArguments, UdpBindOptions, UdpTransport};
use ockam::Context;
use ockam::{Error, Result};
use ockam_core::{route, AsyncTryClone};
use ockam_node::tokio::io::{AsyncReadExt, AsyncWriteExt};
use ockam_node::tokio::net::TcpListener;
use ockam_transport_udp::RendezvousClient;
use std::net::SocketAddr;
use tracing::info;

struct Healthcheck {
    ctx: Context,
    udp: UdpTransport,
    peer: String,
}

impl Healthcheck {
    async fn create(udp: &UdpTransport, socket_address: SocketAddr) -> Result<Self> {
        let peer = if socket_address.ip().is_unspecified() {
            HostnamePort::new("localhost", socket_address.port()).to_string()
        } else {
            socket_address.to_string()
        };

        let ctx = udp.ctx().async_try_clone().await?;

        Ok(Self {
            ctx,
            udp: udp.clone(),
            peer,
        })
    }

    async fn run(&self) -> Result<()> {
        let bind = self
            .udp
            .bind(
                UdpBindArguments::new().with_peer_address(self.peer.clone())?,
                UdpBindOptions::new(),
            )
            .await?;

        let client = RendezvousClient::new(
            &self.ctx,
            &bind,
            route!["rendezvous" /* FIXME RENDEZVOUS_SERVICE */],
        )
        .await?;

        client.ping().await.map_err(|_| {
            Error::new(
                Origin::Application,
                Kind::Unknown,
                "Can't ping the Rendezvous server",
            )
        })
    }
}

#[ockam_macros::node]
async fn main(ctx: Context) -> Result<()> {
    let udp_listen_address = std::env::args()
        .nth(1)
        .unwrap_or(String::from("0.0.0.0:4000"));

    let udp_listen_address = parse_socket_addr(&udp_listen_address)?;

    let healthcheck_addr = std::env::args()
        .nth(2)
        .unwrap_or(String::from("0.0.0.0:4001"));

    info!(
        "Starting UDP Rendezvous service listening on {}",
        udp_listen_address
    );

    RendezvousService::start(&ctx, "rendezvous" /* FIXME RENDEZVOUS_SERVICE */).await?;

    let udp = UdpTransport::create(&ctx).await?;
    let bind = udp
        .bind(
            UdpBindArguments::new().with_bind_socket_address(udp_listen_address),
            UdpBindOptions::new(),
        )
        .await?;

    ctx.flow_controls().add_consumer(
        "rendezvous", /* FIXME RENDEZVOUS_SERVICE*/
        bind.flow_control_id(),
    );

    let healthcheck = Healthcheck::create(&udp, udp_listen_address).await?;
    healthcheck.run().await?;

    let listener = TcpListener::bind(healthcheck_addr.clone())
        .await
        .map_err(|e| {
            Error::new(
                Origin::Transport,
                Kind::Io,
                format!(
                    "Can't listen TCP on address {}, error: {}",
                    healthcheck_addr, e
                ),
            )
        })?;

    info!("Healthcheck active on {}", healthcheck_addr);

    ockam_node::tokio::spawn(async move {
        let mut buffer = [0u8; 1024];
        loop {
            if let Ok((mut tcp_stream, _)) = listener.accept().await {
                // Try to read the request to not trigger error on the client side
                if tcp_stream.read(&mut buffer).await.is_ok() {
                    if healthcheck.run().await.is_ok() {
                        _ = tcp_stream
                            .write_all(b"HTTP/1.1 200 OK\r\n\r\nAlive\n")
                            .await;

                        info!("Healthcheck connection received on {}", healthcheck_addr);
                    } else {
                        info!("Healthcheck failed on {}", healthcheck_addr);
                    }
                }
            }
        }
    });

    // Don't stop context/node. Run forever.
    Ok(())
}
