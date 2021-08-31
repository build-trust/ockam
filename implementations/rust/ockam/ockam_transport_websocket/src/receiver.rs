use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use ockam_core::{async_trait, Address, LocalMessage, Result, TransportMessage, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::WebSocketStream;

use crate::atomic::{self, ArcBool};

pub struct WebSocketRecvWorker<AsyncStream>
where
    AsyncStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub(crate) ws_stream: SplitStream<WebSocketStream<AsyncStream>>,
    pub(crate) run: ArcBool,
    pub(crate) peer_addr: Address,
}

#[async_trait::async_trait]
impl<AsyncStream> Worker for WebSocketRecvWorker<AsyncStream>
where
    AsyncStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // Do not actually listen for messages
    type Message = ();
    type Context = Context;

    // We are using the initialize function here to run a custom loop,
    // while never listening for messages sent to our address
    //
    // Note: when the loop exits, we _must_ call stop_worker(..) on
    // Context not to spawn a zombie task.
    //
    // Also: we must stop the loop when the worker gets killed by
    // the user or node.
    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        loop {
            if !atomic::check(&self.run) {
                break;
            }

            let ws_msg = match self.ws_stream.next().await {
                Some(res) => match res {
                    Ok(ws_msg) => ws_msg,
                    Err(_e) => {
                        info!(
                            "Connection to peer '{}' was closed; dropping stream",
                            self.peer_addr
                        );
                        break;
                    }
                },
                None => {
                    info!(
                        "Stream connected to peer '{}' is exhausted; dropping it",
                        self.peer_addr
                    );
                    break;
                }
            };

            let data = ws_msg.into_data();
            trace!("Received message header for {} bytes", data.len());

            // Deserialize the message now
            let mut msg: TransportMessage = serde_bare::from_slice(data.as_slice())
                .map_err(|_| TransportError::RecvBadMessage)?;

            // Insert the peer address into the return route so that
            // reply routing can be properly resolved
            msg.return_route.modify().prepend(self.peer_addr.clone());

            // Some verbose logging we may want to remove
            trace!("Message onward route: {}", msg.onward_route);
            trace!("Message return route: {}", msg.return_route);

            // FIXME: if we need to re-route (i.e. send it to another
            // domain specific router) the message here, use
            // send_message, instead of forward_message.

            // Forward the message to the final destination worker,
            // which consumes the TransportMessage and yields the
            // final message type
            ctx.forward(LocalMessage::new(msg, Vec::new())).await?;
        }

        // Stop the worker to not fall into the next read loop
        ctx.stop_worker(ctx.address()).await?;
        Ok(())
    }
}
