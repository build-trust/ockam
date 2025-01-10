use ockam_core::compat::{boxed::Box, string::String, vec::Vec};
use ockam_core::{
    async_trait, Address, AllowAll, Encodable, Result, Routed, TransportMessage, Worker,
};
use ockam_node::Context;
use ockam_transport_core::TransportError;

use crate::driver::{AsyncStream, BleStreamDriver, PacketBuffer, Sink, Source};
use crate::workers::BleRecvProcessor;
use crate::BleAddr;

/// Transmit and receive peers of a BLE connection
pub(crate) struct WorkerPair {
    servicenames: Vec<String>,
    peer: BleAddr,
    tx_addr: Address,
}

impl WorkerPair {
    pub fn servicenames(&self) -> &[String] {
        &self.servicenames
    }
    pub fn peer(&self) -> BleAddr {
        self.peer.clone()
    }
    pub fn tx_addr(&self) -> Address {
        self.tx_addr.clone()
    }
}

/// A BLE sending message worker
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub(crate) struct BleSendWorker<A> {
    rx_stream: Option<Source<A>>,
    tx_stream: Option<Sink<A>>,
    peer: BleAddr,
}

impl<A> BleSendWorker<A>
where
    A: BleStreamDriver + Send + 'static,
{
    fn new(stream: AsyncStream<A>, peer: BleAddr) -> Self {
        let (tx, rx) = stream.split();
        Self {
            rx_stream: Some(rx),
            tx_stream: Some(tx),
            peer,
        }
    }

    pub(crate) fn start_pair(
        ctx: &Context,
        stream: AsyncStream<A>,
        peer: BleAddr,
        servicenames: Vec<String>,
    ) -> Result<WorkerPair> {
        debug!("Creating new BLE worker pair");

        let tx_addr = Address::random_local();
        let sender = BleSendWorker::new(stream, peer.clone());

        debug!("start send worker({:?})", tx_addr.clone());
        ctx.start_worker(tx_addr.clone(), sender)?;

        Ok(WorkerPair {
            servicenames,
            peer,
            tx_addr,
        })
    }
}

#[async_trait]
impl<A> Worker for BleSendWorker<A>
where
    A: BleStreamDriver + Send + 'static,
{
    type Context = Context;
    type Message = TransportMessage;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        debug!("initialize for peer: {:?}", self.peer);

        if let Some(rx_stream) = self.rx_stream.take() {
            let rx_addr = Address::random_local();
            let receiver =
                BleRecvProcessor::new(rx_stream, format!("{}#{}", crate::BLE, self.peer).into());
            ctx.start_processor_with_access_control(
                rx_addr.clone(),
                receiver,
                AllowAll, // FIXME: @ac
                AllowAll, // FIXME: @ac
            )?;
            debug!("started receiver");
        } else {
            error!("TransportError::GenericIo");
            return Err(TransportError::GenericIo)?;
        }

        Ok(())
    }

    // BleSendWorker will receive messages from the BleRouter to send
    // across the TcpStream to the next remote peer.
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<TransportMessage>,
    ) -> Result<()> {
        trace!("BleSendWorker::handle_message -> {:?}", msg);
        let mut msg = msg.into_body()?;

        // Remove our own address from the route so the other end
        // knows what to do with the incoming message
        msg.onward_route.step()?;

        // encode message
        let msg = msg.encode().map_err(|_| TransportError::SendBadMessage)?;

        // create packet buffer
        debug!("creating packet buffer");
        let mut packet_buffer = PacketBuffer::from_packet(&msg);

        // send packet length
        debug!("sending packet length: {}", packet_buffer.packet_len());
        let fragment = packet_buffer.send_packet_length();
        match self.tx_stream.as_ref().unwrap().write(&fragment).await {
            Ok(_) => (),
            Err(e) => {
                error!("Failed to send fragment to peer {}: {:?}", self.peer, e);
                ctx.stop_address(ctx.primary_address())?;
            }
        }

        // send packet buffer
        debug!("sending packet fragments");
        while let Some(fragment) = packet_buffer.send_next_fragment() {
            debug!("sending packet fragment: {}", fragment.len());
            match self.tx_stream.as_ref().unwrap().write(fragment).await {
                Ok(_) => (),
                Err(e) => {
                    error!("Failed to send fragment to peer {}: {:?}", self.peer, e);
                    ctx.stop_address(ctx.primary_address())?;
                }
            }

            crate::wait_ms!(100);

            ockam_node::tokio::task::yield_now().await;
        }

        Ok(())
    }
}
