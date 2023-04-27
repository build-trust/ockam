use mitm_node::tcp_interceptor::utils::prepare_message;
use mitm_node::tcp_interceptor::{ProcessorInfo, Role, TcpMitmTransport};
use ockam::{Context, Result};
use ockam_core::{route, Address, AsyncTryClone, TransportMessage};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tracing::info;

#[derive(AsyncTryClone)]
struct MitmMonitor {
    tcp_mitm: TcpMitmTransport,
}

impl MitmMonitor {
    // Read Addresses from the stdin and send a message to those addresses
    async fn send_malicious_message(&self, processor: &ProcessorInfo) -> Result<()> {
        info!("Ready to read Addresses");
        loop {
            let mut address = "".to_string();
            std::io::stdin().read_line(&mut address).unwrap();

            let should_be_not_reachable_address: Address = address.trim().into();

            let msg = TransportMessage::v1(route![should_be_not_reachable_address.clone()], route![], vec![]);

            let msg = prepare_message(msg)?;
            {
                let mut write_half = processor.write_half.lock().await;
                info!(
                    "Sending malicious message from {} to {}",
                    processor.address, should_be_not_reachable_address
                );

                write_half.write_all(msg.as_slice()).await.unwrap();
            }
        }
    }

    // Attach to the intercepted tcp connection
    async fn query_processor(&self, processor: ProcessorInfo) -> Result<()> {
        let self_clone = self.async_try_clone().await?;
        tokio::spawn(async move { self_clone.send_malicious_message(&processor).await })
            .await
            .unwrap()
    }

    // Find and intercept one tcp connection
    async fn monitor_mitm_connections(self) -> Result<()> {
        tokio::spawn(async move {
            loop {
                let processors = self.tcp_mitm.registry().get_all_processors();

                for processor in processors {
                    if let Role::ReadTarget = processor.role {
                        self.query_processor(processor).await?;

                        return Ok::<(), ockam_core::Error>(());
                    }
                }

                self.tcp_mitm.ctx().sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .unwrap()?;

        Ok(())
    }

    pub fn new(tcp_mitm: TcpMitmTransport) -> Self {
        Self { tcp_mitm }
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let listen_ip = "127.0.0.1";
    let target_ip = "52.9.196.1";
    let port1 = "4015";
    let port2 = "4016";

    let tcp_mitm = TcpMitmTransport::create(&ctx).await?;

    tcp_mitm
        .listen(format!("{}:{}", listen_ip, port1), format!("{}:{}", target_ip, port1))
        .await?;

    tcp_mitm
        .listen(format!("{}:{}", listen_ip, port2), format!("{}:{}", target_ip, port2))
        .await?;

    let monitor = MitmMonitor::new(tcp_mitm);
    monitor.monitor_mitm_connections().await?;

    Ok(())
}
