use crate::{
    block_future,
    monotonic::Monotonic,
    protocols::{
        stream::{requests::*, responses::*},
        ProtocolParser,
    },
    stream::{StreamCmdParser, StreamWorkerCmd},
    DelayedEvent, TransportMessage,
};
use crate::{Address, Any, Context, Result, Route, Routed, Worker};
use std::time::Duration;

/// A stream worker
pub struct StreamConsumer {
    parser: ProtocolParser<Self>,
    ids: Monotonic,
    /// Stream remote
    peer: Route,
    /// Producer address
    prod: Address,
    /// Stream name
    stream: Option<String>,
    /// Fetch interval
    interval: Duration,
    /// Forwarding address
    fwd: Option<Address>,
    /// ReceiverAddress address
    rx_rx: Address,
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
            w.stream = Some(stream_name);
            w.peer = return_route.clone();

            // Queue fetch events
            fetch_interval(ctx, w.interval.clone()).expect("Failed to start fetch event loop!");

            // Initialise the producer!
            block_future(&ctx.runtime(), async {
                ctx.send(w.prod.clone(), StreamWorkerCmd::init(return_route))
                    .await
            })
            .expect("Failed to initialise stream producer!");

            true
        }
        Response::PullResponse(PullResponse {
            request_id,
            messages,
        }) => {
            trace!("PullResponse, {} message(s) available", messages.len());

            match w.fwd {
                Some(_) => {
                    // TODO: forward to a worker
                }
                None => {
                    // Send to the rx_rx address
                    for m in messages {
                        trace!("Forwarding message {:?} to rx.next()", m);
                        block_future(&ctx.runtime(), async { ctx.send(w.rx_rx.clone(), m).await })
                            .expect("Failed to forward received message!");
                    }
                }
            }

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
                ctx.send(w.peer.clone(), PullRequest::new(request_id, 0, 8))
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
        ctx.send(self.peer.clone(), (CreateStreamRequest::new(None)))
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
        peer: Route,
        prod: Address,
        stream: Option<String>,
        interval: Duration,
        fwd: Option<Address>,
        rx_rx: Address,
    ) -> Self {
        Self {
            parser: ProtocolParser::new(),
            ids: Monotonic::new(),
            peer,
            prod,
            stream,
            interval,
            fwd,
            rx_rx,
        }
    }
}
