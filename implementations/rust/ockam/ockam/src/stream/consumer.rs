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
use std::time::Duration;

/// A stream worker
pub struct StreamConsumer {
    parser: ProtocolParser<Self>,
    ids: Monotonic,
    /// Stream service remote
    stream_peer: Route,
    /// Index service remote
    index_peer: Route,
    /// This client ID
    client_id: String,
    /// Producer address
    prod: Option<Address>,
    /// Receiving stream name
    stream: String,
    /// Fetch interval
    interval: Duration,
    /// ReceiverAddress address
    rx_rx: Address,
    /// Last known index position
    idx: u64,
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

            assert_eq!(w.stream, stream_name);
            w.stream_peer = return_route.clone();

            // Next up we get the current index
            block_future(&ctx.runtime(), async move {
                ctx.send(
                    w.index_peer.clone(),
                    IndexReq::get(stream_name, w.client_id.clone()),
                )
                .await
            });

            true
        }
        Response::Index(IndexResp {
            stream_name, index, ..
        }) => {
            info!("Updating index '{}' to: {}", stream_name, index.0);
            w.index_peer = return_route.clone();
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

                let res = match trans.onward_route.next() {
                    Ok(addr) => {
                        info!("Forwarding {} message to addr: {}", w.stream, addr);
                        block_future(&ctx.runtime(), async { ctx.forward(trans).await })
                    }
                    Err(_) => {
                        info!("Forwarding {} message to rx.next()", w.stream);
                        block_future(&ctx.runtime(), async {
                            ctx.send(w.rx_rx.clone(), msg).await
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

            // TODO: After handling the messages we update the index

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
                trace!("Sending PullRequest to stream {:?}...", w.stream);
                ctx.send(
                    w.stream_peer.clone(),
                    // TOOD: make fetch amount configurable/ dynamic?
                    PullRequest::new(request_id, w.idx, 8),
                )
                .await
            })?;

            // Queue a new fetch event and mark this event as handled
            fetch_interval(ctx, w.interval.clone()).map(|_| true)
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

        // Send a create stream request with the reigestered name
        ctx.send(
            self.stream_peer.clone(),
            (CreateStreamRequest::new(self.stream.clone())),
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
        remote: Route,
        prod: Option<Address>,
        stream: String,
        interval: Duration,
        fwd: Option<Address>,
        rx_rx: Address,
    ) -> Self {
        Self {
            parser: ProtocolParser::new(),
            ids: Monotonic::new(),
            client_id,
            stream_peer: remote.clone().modify().append("stream_service").into(),
            index_peer: remote
                .clone()
                .modify()
                .append("stream_index_service")
                .into(),
            prod,
            stream,
            interval,
            rx_rx,
            idx: 0,
        }
    }
}
