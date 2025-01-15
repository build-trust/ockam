use miette::miette;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// A callback node should call when it's up and running, implemented via creating a localhost TCP
/// connection
pub struct NodeCallback {
    tcp_listener: tokio::net::TcpListener,
    callback_port: u16,
}

impl NodeCallback {
    /// Create a callback and listen for incoming signal from the child node via TCP
    pub async fn create() -> miette::Result<Self> {
        let tcp_listener =
            tokio::net::TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
                .await
                .map_err(|_| miette!("Failed to bind callback listener"))?;

        let callback_port = tcp_listener
            .local_addr()
            .map_err(|_| miette!("Failed to get callback listener port"))?
            .port();

        Ok(Self {
            tcp_listener,
            callback_port,
        })
    }

    /// Wait for a signal from the child process
    pub async fn wait_for_signal(self) -> miette::Result<()> {
        _ = self
            .tcp_listener
            .accept()
            .await
            .map_err(|_| miette!("Failed to accept node callback connection"))?;

        Ok(())
    }

    /// Send the signal to the parent process
    pub fn signal(callback_port: u16) {
        // let the parent process or whatever started us know that we're up and running
        // no need to wait for that operation to complete, so spawn to the background
        tokio::spawn(tokio::net::TcpStream::connect(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            callback_port,
        )));
    }

    /// TPC port used for the callback
    pub fn callback_port(&self) -> u16 {
        self.callback_port
    }
}
