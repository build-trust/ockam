use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::{Address, Decodable, LocalMessage, Processor, Result, TransportMessage};
use ockam_node::Context;
use ockam_transport_core::TransportError;

use crate::driver::Source;
use crate::driver::{BleEvent, BleStreamDriver, PacketBuffer};

/// A BLE receiving message worker
///
/// This half of the worker is created when spawning a new worker
/// pair, and listens for incoming BLE events, to relay into the node
/// message system.
pub struct BleRecvProcessor<A> {
    rx_stream: Source<A>,
    peer_addr: Address,
    packet_buffer: PacketBuffer,
}

impl<A> BleRecvProcessor<A>
where
    A: BleStreamDriver + Send + 'static,
{
    pub(crate) fn new(rx_stream: Source<A>, peer_addr: Address) -> Self {
        Self {
            rx_stream,
            peer_addr,
            packet_buffer: PacketBuffer::default(),
        }
    }
}

#[async_trait]
impl<A> Processor for BleRecvProcessor<A>
where
    A: BleStreamDriver + Send + 'static,
{
    type Context = Context;

    async fn process(&mut self, ctx: &mut Context) -> Result<bool> {
        let mut buffer = [0_u8; crate::driver::MAX_OCKAM_MESSAGE_LENGTH];

        match self.rx_stream.poll(&mut buffer).await {
            Ok(BleEvent::None) => {}
            Ok(BleEvent::Unknown) => {
                debug!("\t=> BleEvent::Unknown");
            }
            Ok(BleEvent::ConnectionComplete) => {
                debug!("\t=> BleEvent::ConnectionComplete");
            }
            Ok(BleEvent::DisconnectionComplete) => {
                debug!("\t=> BleEvent::DisconnectionComplete");
            }
            Ok(BleEvent::Received(fragment)) => {
                debug!("\t=> BleEvent::ReceivedData -> {:?} bytes", fragment.len());

                self.handle_received(ctx, fragment).await?;
            }
            Err(e) => {
                error!("BleRecvProcessor::process poll error: {:?}", e);
                return Err(crate::error::BleError::ReadError)?;
            }
        }

        Ok(true)
    }
}

impl<A> BleRecvProcessor<A>
where
    A: BleStreamDriver + Send + 'static,
{
    async fn handle_received(&mut self, ctx: &mut Context, fragment: &[u8]) -> Result<()> {
        // first fragment contains the expected packet length
        if let Some(packet_len) = self.packet_buffer.receive_packet_length(fragment) {
            debug!("Received packet length: {} bytes", packet_len);
            return Ok(());
        }

        match self.packet_buffer.receive_next_fragment(fragment) {
            Ok(Some(packet)) => {
                trace!("Received packet: {:?}", packet);

                // Try to deserialize a message
                let result =
                    TransportMessage::decode(packet).map_err(|_| TransportError::RecvBadMessage);
                let mut msg = match result {
                    Err(e) => {
                        error!("Error decoding message: {:?}", e);
                        return Err(e)?;
                    }
                    Ok(msg) => LocalMessage::from_transport_message(msg),
                };

                trace!("Deserialized message successfully: {:?}", msg);

                // Insert the peer address into the return route so that
                // reply routing can be properly resolved
                msg = msg.push_front_return_route(self.peer_addr.clone());

                // Some verbose logging we may want to remove
                debug!("Message onward route: {}", msg.onward_route());
                debug!("Message return route: {}", msg.return_route());

                // Forward the message to the final destination worker,
                // which consumes the TransportMessage and yields the
                // final message type
                ctx.forward(msg).await?;

                // reset packet buffer
                self.packet_buffer.reset();
            }
            Ok(None) => {
                trace!("Added fragment to packet buffer -> {:?}", fragment);
            }
            Err(e) => {
                error!("Invalid packet fragment: {:?} -> {:?}", e, fragment);
                self.packet_buffer.reset();
            }
        }

        Ok(())
    }
}
