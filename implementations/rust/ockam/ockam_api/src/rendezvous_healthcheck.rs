use crate::DefaultAddress;
use ockam::transport::HostnamePort;
use ockam::udp::{RendezvousClient, UdpBindArguments, UdpBindOptions, UdpTransport};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Error, Result, TryClone};
use ockam_node::Context;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tracing::info;

pub struct RendezvousHealthcheck {
    task: Option<RendezvousHealthcheckTask>,
    healthcheck_listening_address: String,
    handle: Option<JoinHandle<()>>,
}

impl RendezvousHealthcheck {
    pub fn create(
        healthcheck_listening_address: &str,
        udp: &UdpTransport,
        udp_socket_address: SocketAddr,
    ) -> Result<Self> {
        let peer = if udp_socket_address.ip().is_unspecified() {
            HostnamePort::new("localhost", udp_socket_address.port())?.to_string()
        } else {
            udp_socket_address.to_string()
        };

        let ctx = udp.ctx().try_clone()?;

        let task = RendezvousHealthcheckTask {
            ctx,
            udp: udp.clone(),
            peer,
        };

        Ok(Self {
            task: Some(task),
            healthcheck_listening_address: healthcheck_listening_address.to_string(),
            handle: None,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.handle.is_some() {
            return Err(Error::new(
                Origin::Application,
                Kind::Unknown,
                "Can't start Healthcheck because it is already started (handle is present)",
            ));
        }

        let task = self.task.take().ok_or_else(|| {
            Error::new(
                Origin::Application,
                Kind::Unknown,
                "Can't start Healthcheck because it is already started (task is present)",
            )
        })?;

        let listener = TcpListener::bind(self.healthcheck_listening_address.clone())
            .await
            .map_err(|e| {
                Error::new(
                    Origin::Transport,
                    Kind::Io,
                    format!(
                        "Can't listen TCP on address {}, error: {}",
                        self.healthcheck_listening_address, e
                    ),
                )
            })?;

        info!(
            "Healthcheck active on {}",
            self.healthcheck_listening_address
        );

        let healthcheck_address = self.healthcheck_listening_address.clone();

        let handle = ockam_node::tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            loop {
                if let Ok((mut tcp_stream, _)) = listener.accept().await {
                    // Try to read the request to not trigger error on the client side
                    if tcp_stream.read(&mut buffer).await.is_ok() {
                        if task.run_check().await.is_ok() {
                            _ = tcp_stream
                                .write_all(b"HTTP/1.1 200 OK\r\n\r\nAlive\n")
                                .await;

                            info!("Healthcheck connection received on {}", healthcheck_address);
                        } else {
                            info!("Healthcheck failed on {}", healthcheck_address);
                        }
                    }
                }
            }
        });

        self.handle = Some(handle);

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            Ok(())
        } else {
            Err(Error::new(
                Origin::Application,
                Kind::Unknown,
                "Can't stop Healthcheck because it is already stopped",
            ))
        }
    }
}

struct RendezvousHealthcheckTask {
    ctx: Context,
    udp: UdpTransport,
    peer: String,
}

impl RendezvousHealthcheckTask {
    async fn run_check(&self) -> Result<()> {
        let bind = self
            .udp
            .bind(
                UdpBindArguments::new()
                    .with_peer_address(self.peer.clone())
                    .await?,
                UdpBindOptions::new(),
            )
            .await?;

        let client = RendezvousClient::new(&bind, route![DefaultAddress::RENDEZVOUS_SERVICE]);

        let res = client.ping(&self.ctx).await.map_err(|_| {
            Error::new(
                Origin::Application,
                Kind::Unknown,
                "Can't ping the Rendezvous server",
            )
        });

        self.udp.unbind(bind.as_ref())?;

        res
    }
}
