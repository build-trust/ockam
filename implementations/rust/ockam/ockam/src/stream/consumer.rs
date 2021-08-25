use crate::{
    block_future,
    monotonic::Monotonic,
    protocols::{
        stream::{
            requests::{Index as IndexReq, *},
            responses::{Index as IndexResp, *},
        },
        ProtocolParser,
    },
    stream::{StreamCmdParser, StreamWorkerCmd},
    DelayedEvent, Message, TransportMessage,
};
use crate::{Address, Any, Context, Result, Route, Routed, Worker};
use core::time::Duration;
use ockam_core::compat::{boxed::Box, string::String, vec::Vec};
use ockam_core::LocalMessage;

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
    parser: ProtocolParser<Self>,
    ids: Monotonic,
}

fn parse_response(w: &mut StreamConsumer, ctx: &mut Context, resp: Routed<Response>) -> bool {
    let return_route = resp.return_route();
    match resp.body() {
        // When our stream is initialised we start the fetch_interval
        Response::Init(Init { stream_name }) => {
            info!(
                "Initialised consumer for stream '{}' and route: {}",
                stream_name, return_route
            );

            assert_eq!(w.receiver_name, stream_name);
            w.service_route = return_route.clone();

            // Next up we get the current index
            block_future(&ctx.runtime(), async move {
                ctx.send(
                    w.index_route.clone(),
                    IndexReq::get(stream_name, w.client_id.clone()),
                )
                .await
            });

            true
        }
        Response::Index(IndexResp {
            stream_name, index, ..
        }) => {
            let index = index.unwrap_or(serde_bare::Uint(0));
            info!("Updating index '{}' to: {}", stream_name, index.0);
            w.index_route = return_route.clone();
            w.idx = index.0;

            // Queue a near-immediate fetch event -- however future
            // events will be using the specified user interval
            fetch_interval(ctx, Duration::from_millis(10))
                .expect("Failed to start fetch event loop!");

            true
        }
        Response::PullResponse(PullResponse {
            request_id,
            messages,
        }) => {
            trace!("PullResponse, {} message(s) available", messages.len());

            let last_idx = w.idx;

            // Update the index if we received messages
            if let Some(ref msg) = messages.last() {
                w.idx = msg.index.0 + 1;
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
                        info!("Forwarding {} message to addr: {}", w.receiver_name, addr);
                        let local_msg = LocalMessage::new(trans, Vec::new());
                        block_future(&ctx.runtime(), async { ctx.forward(local_msg).await })
                    }
                    Err(_) => {
                        info!("Forwarding {} message to receiver.next()", w.receiver_name);
                        block_future(&ctx.runtime(), async {
                            ctx.send(w.receiver_rx.clone(), msg).await
                        })
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
                block_future(&ctx.runtime(), async {
                    ctx.send(
                        w.index_route.clone(),
                        IndexReq::save(w.receiver_name.clone(), w.client_id.clone(), w.idx),
                    )
                    .await
                });
            }

            // Queue a new fetch event and mark this event as handled
            fetch_interval(ctx, w.interval.clone()).unwrap();

            true
        }
        _ => false,
    }
}

fn parse_cmd(
    w: &mut StreamConsumer,
    ctx: &mut Context,
    cmd: Routed<StreamWorkerCmd>,
) -> Result<bool> {
    match cmd.body() {
        StreamWorkerCmd::Fetch => {
            trace!("Handling StreamWorkerCmd::Fetch");

            // Generate a new request_id and send a PullRequest
            block_future(&ctx.runtime(), async {
                let request_id = w.ids.next() as u64;
                trace!("Sending PullRequest to stream {:?}...", w.receiver_name);
                ctx.send(
                    w.service_route.clone(),
                    // TOOD: make fetch amount configurable/ dynamic?
                    PullRequest::new(request_id, w.idx, 8),
                )
                .await
            })?;

            Ok(true)
        }
        f => {
            warn!("Unhandled message type {:?}", f);
            Ok(false)
        }
    }
}

/// Dispatch a fetch event with an interval duration
///
/// This function must be re-called whenever a fetch event is handled
/// in the `parse_cmd` function.
fn fetch_interval(ctx: &Context, interval: Duration) -> Result<()> {
    DelayedEvent::new(ctx, ctx.address().into(), StreamWorkerCmd::fetch())?
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
        info!("Initialising stream consumer {}", ctx.address());

        self.parser.attach(ResponseParser::new(parse_response));
        self.parser.attach(StreamCmdParser::new(parse_cmd));

        // Send a create stream request with the registered name
        ctx.send(
            self.service_route.clone(),
            (CreateStreamRequest::new(self.receiver_name.clone())),
        )
        .await?;

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        // Handle payloads via our protocol parser
        if let Ok(true) = self.parser.prepare().parse(self, ctx, &msg) {
            return Ok(());
        }

        warn!(
            "Unhandled message for consumer {}: {:?}", // TODO: attempt to get protocol ID
            ctx.address(),
            msg.body()
        );
        Ok(())
    }
}

impl StreamConsumer {
    pub(crate) fn new(
        client_id: String,
        route: Route,
        sender_address: Option<Address>,
        receiver_name: String,
        interval: Duration,
        _forwarding_address: Option<Address>, // TODO implement forwarding
        receiver_rx: Address,
        stream_service: String,
        index_service: String,
    ) -> Self {
        Self {
            client_id,
            service_route: route.clone().modify().append(stream_service).into(),
            index_route: route.clone().modify().append(index_service).into(),
            sender_address,
            receiver_name,
            interval,
            receiver_rx,
            idx: 0,
            parser: ProtocolParser::new(),
            ids: Monotonic::new(),
        }
    }
}
