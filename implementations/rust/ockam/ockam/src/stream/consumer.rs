use crate::{
    delay::DelayedEvent,
    monotonic::Monotonic,
    protocols::{
        stream::{requests::*, responses::*},
        ProtocolParser, ProtocolPayload,
    },
    stream::StreamWorkerCmd,
    OckamError, TransportMessage,
};
use crate::{Address, Any, Context, Result, Route, Routed, Worker};
use core::time::Duration;
use ockam_core::compat::{boxed::Box, string::String, vec::Vec};
use ockam_core::{Decodable, LocalMessage};

/// A stream worker
pub struct StreamConsumer {
    /// This client ID
    client_id: String,
    /// Stream service remote
    service_route: Route,
    /// Index service remote
    index_route: Route,
    /// Sender address
    sender_address: Option<Address>,
    /// Receiving stream name
    receiver_name: String,
    /// Fetch interval
    interval: Duration,
    /// ReceiverAddress address
    receiver_rx: Address,
    /// Last known index position
    idx: u64,
    ids: Monotonic,
}

/// Function which is called whenever a `Response` message is parsed
async fn handle_response(
    w: &mut StreamConsumer,
    ctx: &mut Context,
    r: Routed<Any>,
    response: Response,
) -> Result<()> {
    let return_route = r.return_route();
    match response {
        Response::Init(InitResponse { stream_name }) => {
            info!(
                "Initialised consumer for stream '{}' and route: {:?}",
                stream_name, return_route
            );

            assert_eq!(w.receiver_name, stream_name);
            w.service_route = return_route;

            ctx.send(
                w.index_route.clone(),
                IndexRequest::get(stream_name, w.client_id.clone()),
            )
            .await
        }
        Response::Index(IndexResponse {
            stream_name, index, ..
        }) => {
            let index = index.unwrap_or_else(|| 0.into());
            info!("Updating index '{}' to: {}", stream_name, index.u64());
            w.index_route = return_route;
            w.idx = index.u64();

            // Queue a near-immediate fetch event -- however future
            // events will be using the specified user interval
            fetch_interval(ctx, Duration::from_millis(10))
                .await
                .expect("Failed to start fetch event loop!");

            Ok(())
        }
        Response::PullResponse(PullResponse { messages, .. }) => {
            trace!("PullResponse, {} message(s) available", messages.len());

            let last_idx = w.idx;

            // Update the index if we received messages
            if let Some(msg) = messages.last() {
                w.idx = msg.index.u64() + 1;
            }

            for msg in messages {
                let mut trans = match TransportMessage::decode(&msg.data) {
                    Ok(t) => t,
                    _ => {
                        error!("Failed to decode TransportMessage from StreamMessage payload; skipping!");
                        continue;
                    }
                };

                // If a producer exists, insert its address into the return_route
                if let Some(ref addr) = w.sender_address {
                    trans.return_route.modify().prepend(addr.clone());
                }

                // Either forward to the next hop, or to the consumer address
                let res = match trans.onward_route.next() {
                    Ok(addr) => {
                        info!("Forwarding {} message to addr: {:?}", w.receiver_name, addr);
                        let local_msg = LocalMessage::new(trans, Vec::new());
                        ctx.forward(local_msg).await
                    }
                    Err(_) => {
                        info!("Forwarding {} message to receiver.next()", w.receiver_name);
                        ctx.send(w.receiver_rx.clone(), msg).await
                    }
                };

                match res {
                    Ok(()) => {}
                    Err(e) => {
                        error!("Failed forwarding stream message: {}", e);
                    }
                }
            }

            // If the index was updated, save it
            if last_idx != w.idx {
                ctx.send(
                    w.index_route.clone(),
                    IndexRequest::save(w.receiver_name.clone(), w.client_id.clone(), w.idx),
                )
                .await?;
            }

            // Queue a new fetch event and mark this event as handled
            if fetch_interval(ctx, w.interval).await.is_err() {
                warn!("Failed to create fetch_interval event: node shutting down");
            }

            Ok(())
        }

        _ => Err(OckamError::NoSuchProtocol.into()),
    }
}

async fn handle_cmd(
    w: &mut StreamConsumer,
    ctx: &mut Context,
    _r: Routed<Any>,
    cmd: StreamWorkerCmd,
) -> Result<()> {
    match cmd {
        StreamWorkerCmd::Fetch => {
            trace!("Handling StreamWorkerCmd::Fetch");

            // Generate a new request_id and send a PullRequest
            let request_id = w.ids.next() as u64;
            trace!("Sending PullRequest to stream {:?}...", w.receiver_name);
            ctx.send(
                w.service_route.clone(),
                // TOOD: make fetch amount configurable/ dynamic?
                PullRequest::new(request_id, w.idx, 8),
            )
            .await?;

            Ok(())
        }
        f => {
            warn!("Unhandled message type {:?}", f);
            Err(OckamError::NoSuchProtocol.into())
        }
    }
}

/// Dispatch a fetch event with an interval duration
///
/// This function must be re-called whenever a fetch event is handled
/// in the `parse_cmd` function.
async fn fetch_interval(ctx: &Context, interval: Duration) -> Result<()> {
    DelayedEvent::new(ctx, ctx.address().into(), StreamWorkerCmd::fetch())
        .await?
        .with_duration(interval)
        .spawn();
    Ok(())
}

#[crate::worker]
impl Worker for StreamConsumer {
    type Context = Context;
    type Message = Any;

    /// Initialise the stream consumer
    ///
    /// This involves sending a CreateStreamRequest to the peer and
    /// waiting for a reply.
    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        info!("Initialising stream consumer {:?}", ctx.address());

        // Send a create_stream_request with the registered name
        ctx.send(
            self.service_route.clone(),
            CreateStreamRequest::new(self.receiver_name.clone()),
        )
        .await?;

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let pp = ProtocolPayload::decode(msg.payload())?;
        let id = pp.protocol.as_str();

        if Response::check_id(id) {
            let response = Response::parse(pp)?;
            handle_response(self, ctx, msg, response).await?;
        } else if StreamWorkerCmd::check_id(id) {
            let cmd = StreamWorkerCmd::parse(pp)?;
            handle_cmd(self, ctx, msg, cmd).await?;
        } else {
            warn!(
                "Unhandled message for consumer {:?}: {:?}", // TODO: attempt to get protocol ID
                ctx.address(),
                msg.body()
            );
        }

        Ok(())
    }
}

impl StreamConsumer {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        client_id: String,
        mut route: Route,
        sender_address: Option<Address>,
        receiver_name: String,
        interval: Duration,
        _forwarding_address: Option<Address>, // TODO implement forwarding
        receiver_rx: Address,
        stream_service: Address,
        index_service: Address,
    ) -> Self {
        Self {
            client_id,
            service_route: route.clone().modify().append(stream_service).into(),
            index_route: route.modify().append(index_service).into(),
            sender_address,
            receiver_name,
            interval,
            receiver_rx,
            idx: 0,
            ids: Monotonic::new(),
        }
    }
}
