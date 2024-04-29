use rand::Rng;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use ockam::compat::tokio;
use ockam::{Context, Processor, Route, Routed, Worker};
use ockam_api::nodes::InMemoryNode;
use ockam_core::flow_control::FlowControlId;
use ockam_core::{async_trait, route, Address, AllowAll, DenyAll, NeutralMessage};
use ockam_multiaddr::MultiAddr;

use crate::config::Throughput;

pub struct PortalStats {
    pub messages_out_of_order: Arc<AtomicU64>,
    pub bytes_received: Arc<AtomicU64>,
    pub messages_received: Arc<AtomicU64>,
    pub bytes_sent: Arc<AtomicU64>,
    pub messages_sent: Arc<AtomicU64>,
}

pub async fn create(
    context: Arc<Context>,
    node: Arc<InMemoryNode>,
    id: String,
    to: MultiAddr,
    relay_address: Address,
    throughput: Throughput,
    relay_flow_control_id: FlowControlId,
) -> ockam::Result<PortalStats> {
    let portal_stats = PortalStats {
        messages_out_of_order: Default::default(),
        bytes_received: Default::default(),
        messages_received: Default::default(),
        bytes_sent: Default::default(),
        messages_sent: Default::default(),
    };

    let receiver_address = Address::from_string(format!("receiver_{id}", id = id));
    let sender_address = Address::from_string(format!("sender_{id}", id = id));

    context
        .flow_controls()
        .add_consumer(receiver_address.clone(), &relay_flow_control_id);

    let worker = PortalSimulatorReceiver {
        messages_out_of_order: portal_stats.messages_out_of_order.clone(),
        bytes_received: portal_stats.bytes_received.clone(),
        messages_received: portal_stats.messages_received.clone(),
        next_message_number: 0,
    };

    context
        .start_worker(receiver_address.clone(), worker)
        .await
        .unwrap();

    let connection = node
        .make_connection(context.clone(), &to, node.identifier(), None, None)
        .await?;

    let processor = PortalSimulatorSender {
        to: route![connection.route()?, relay_address, receiver_address],
        throughput,
        bytes_sent: portal_stats.bytes_sent.clone(),
        messages_sent: portal_stats.messages_sent.clone(),
    };

    context
        .start_processor_with_access_control(sender_address, processor, DenyAll, AllowAll)
        .await
        .unwrap();

    Ok(portal_stats)
}

struct PortalSimulatorSender {
    to: Route,
    throughput: Throughput,
    bytes_sent: Arc<AtomicU64>,
    messages_sent: Arc<AtomicU64>,
}
#[async_trait]
impl Processor for PortalSimulatorSender {
    type Context = Context;

    // assume this method is called once per second
    async fn process(&mut self, context: &mut Self::Context) -> ockam::Result<bool> {
        let timestamp = Instant::now();
        let mut bytes_left = match self.throughput {
            // assume an arbitrary MB, should not impact since there is no sleep
            Throughput::Unlimited => 1024 * 1024,
            Throughput::Bytes(bytes) => bytes as usize,
        };

        while bytes_left > 0 {
            let next_message_number = self.messages_sent.fetch_add(1, Ordering::Relaxed);
            let payload_size = std::cmp::min(bytes_left, 48 * 1024);
            let mut message = Vec::with_capacity(8 + 8 + payload_size);

            message.extend_from_slice(&next_message_number.to_le_bytes());
            message.extend_from_slice(&payload_size.to_le_bytes());
            message.write_all(&vec![0u8; payload_size]).unwrap();

            context
                .send(self.to.clone(), NeutralMessage::from(message))
                .await?;

            self.bytes_sent
                .fetch_add(payload_size as u64, Ordering::Relaxed);
            bytes_left -= payload_size;
        }

        // sleep only when the throughput is limited
        if matches!(self.throughput, Throughput::Bytes(_)) {
            // range sleep time between 0.8-1.2 to better distribute the load across time
            let duration =
                std::time::Duration::from_millis(rand::thread_rng().gen_range(800..1200));
            let elapsed = timestamp.elapsed();
            if duration > elapsed {
                tokio::time::sleep(duration - elapsed).await;
            }
        }

        Ok(true)
    }
}

struct PortalSimulatorReceiver {
    messages_out_of_order: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    messages_received: Arc<AtomicU64>,
    next_message_number: u64,
}

#[async_trait]
impl Worker for PortalSimulatorReceiver {
    type Message = NeutralMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        _context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> ockam::Result<()> {
        let message = message.into_payload();

        if message.len() < 16 {
            panic!("Invalid message size: {}", message.len());
        }

        let message_number = u64::from_le_bytes(message[0..8].try_into().unwrap());
        let payload_size = u64::from_le_bytes(message[8..16].try_into().unwrap());

        if message_number != self.next_message_number {
            self.messages_out_of_order.fetch_add(1, Ordering::Relaxed);
        }

        self.next_message_number = message_number.max(self.next_message_number) + 1;

        if payload_size != message.len() as u64 - 16 {
            panic!(
                "Invalid payload size: {}, expected {}",
                payload_size,
                message.len() - 16
            );
        }

        self.bytes_received
            .fetch_add(payload_size, Ordering::Relaxed);
        self.messages_received.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }
}
