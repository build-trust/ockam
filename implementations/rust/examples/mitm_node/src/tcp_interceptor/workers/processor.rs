use crate::tcp_interceptor::{Role, TcpMitmRegistry};
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Address, AllowAll};
use ockam_core::{Processor, Result};
use ockam_node::compat::asynchronous::Mutex;
use ockam_node::Context;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};
use tracing::debug;

pub(crate) struct TcpMitmProcessor {
    address_of_other_processor: Address,
    role: Role,
    read_half: OwnedReadHalf,
    write_half: Arc<Mutex<OwnedWriteHalf>>,
    registry: TcpMitmRegistry,
}

impl TcpMitmProcessor {
    fn new(
        address_of_other_processor: Address,
        role: Role,
        read_half: OwnedReadHalf,
        write_half: Arc<Mutex<OwnedWriteHalf>>,
        registry: TcpMitmRegistry,
    ) -> Self {
        Self {
            address_of_other_processor,
            role,
            read_half,
            write_half,
            registry,
        }
    }

    pub fn start(
        ctx: &Context,
        role: Role,
        address: Address,
        address_of_other_processor: Address,
        read_half: OwnedReadHalf,
        write_half: OwnedWriteHalf,
        registry: TcpMitmRegistry,
    ) -> Result<()> {
        let write_half = Arc::new(Mutex::new(write_half));

        let receiver = Self::new(address_of_other_processor, role, read_half, write_half, registry);

        ctx.start_processor_with_access_control(address, receiver, AllowAll, AllowAll)?;

        Ok(())
    }
}

#[async_trait]
impl Processor for TcpMitmProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        self.registry
            .add_processor(ctx.primary_address(), self.role, self.write_half.clone());

        debug!("Initialize {}", ctx.primary_address());

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.remove_processor(ctx.primary_address());

        debug!("Shutdown {}", ctx.primary_address());

        Ok(())
    }

    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        let mut buf = vec![0; 1024];

        let len = match self.read_half.read(&mut buf).await {
            Ok(l) if l != 0 => l,
            _ => {
                debug!("Connection was closed; dropping stream {}", ctx.primary_address());

                let _ = ctx.stop_address(&self.address_of_other_processor);

                return Ok(false);
            }
        };

        match self.write_half.lock().await.write_all(&buf[..len]).await {
            Ok(_) => {
                debug!("Forwarded {} bytes from {}", len, ctx.primary_address());
            }
            _ => {
                debug!("Connection was closed; dropping stream {}", ctx.primary_address());

                let _ = ctx.stop_address(&self.address_of_other_processor);

                return Ok(false);
            }
        }

        Ok(true)
    }
}
