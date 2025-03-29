use crate::Result;
use std::fmt::Display;
use tokio::net::TcpListener;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Port {
    TryExplicitOrRandom(u16),
    Explicit(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindAddress {
    port: Port,
    address: String,
}

impl BindAddress {
    pub fn new(address: String, port: Port) -> Self {
        Self { port, address }
    }

    pub async fn bind(&self) -> Result<TcpListener> {
        let address = &self.address;
        match self.port {
            Port::TryExplicitOrRandom(port) => {
                // Try to bind to the explicit port
                match TcpListener::bind(&format!("{address}:{port}")).await {
                    Ok(listener) => Ok(listener),
                    Err(err) => {
                        // If the port is already in use, bind to a random port
                        if err.kind() == std::io::ErrorKind::AddrInUse {
                            Ok(TcpListener::bind(format!("{address}:0")).await?)
                        } else {
                            Err(err.into())
                        }
                    }
                }
            }
            // If explicit, try to bind to that port or fail
            Port::Explicit(port) => Ok(TcpListener::bind(&format!("{address}:{port}")).await?),
        }
    }
}

impl Display for BindAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let port = match self.port {
            Port::TryExplicitOrRandom(port) => port,
            Port::Explicit(port) => port,
        };
        write!(f, "{}:{port}", &self.address)
    }
}
