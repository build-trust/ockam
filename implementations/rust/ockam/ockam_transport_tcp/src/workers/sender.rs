use async_trait::async_trait;
use ockam_core::{Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::net::SocketAddr;
use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf};
use tracing::warn;

/// A TCP sending message worker
///
/// Create this worker type by calling
/// [`start_tcp_worker`](crate::start_tcp_worker)!
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub(crate) struct TcpSendWorker {
    tx: OwnedWriteHalf,
    peer: SocketAddr,
}

impl TcpSendWorker {
    pub fn new(tx: OwnedWriteHalf, peer: SocketAddr) -> Self {
        Self { tx, peer }
    }
}

fn prepare_message(msg: TransportMessage) -> Result<Vec<u8>> {
    let mut msg_buf = serde_bare::to_vec(&msg).map_err(|_| TransportError::SendBadMessage)?;

    // Create a buffer that includes the message length in big endian
    let mut len = (msg_buf.len() as u16).to_be_bytes().to_vec();

    // Fun fact: reversing a vector in place, appending the length,
    // and then reversing it again is faster for large message sizes
    // than adding the large chunk of data.
    //
    // https://play.rust-lang.org/?version=stable&mode=release&edition=2018&gist=8669a640004ac85c7be38b19e3e73dcb
    msg_buf.reverse();
    len.reverse();
    msg_buf.append(&mut len);
    msg_buf.reverse();

    Ok(msg_buf)
}

#[async_trait]
impl Worker for TcpSendWorker {
    type Context = Context;
    type Message = TransportMessage;

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        mut msg: Routed<TransportMessage>,
    ) -> Result<()> {
        // Remove our own address from the route so the other end
        // knows what to do with the incoming message
        msg.onward_route.step()?;

        // Create a message buffer with pre-pended length
        let msg = prepare_message(msg.body())?;

        if self.tx.write(msg.as_slice()).await.is_err() {
            warn!("Failed to send message to peer {}", self.peer);
            ctx.stop_worker(ctx.address()).await?;
        }

        Ok(())
    }
}
