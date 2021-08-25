use crate::{
    block_future,
    monotonic::Monotonic,
    protocols::{
        stream::{requests::*, responses::*},
        ProtocolParser, ProtocolPayload,
    },
    stream::StreamWorkerCmd,
    Any, Context, Message, Result, Route, Routed, TransportMessage, Worker,
};
use ockam_core::compat::{boxed::Box, collections::VecDeque, string::String};

use super::StreamCmdParser;

pub struct StreamProducer {
    sender_name: String,
    route: Route,
    stream_service: String,
    parser: ProtocolParser<Self>,
    outbox: VecDeque<ProtocolPayload>,
    ids: Monotonic,
    /// Keep track of whether this producer has been initialised
    ///
    /// The reason for this is that `route` first is the Route to the
    /// `stream_service`, and later to the exact stream worker for
    /// this stream
    init: bool,
}

fn parse_response(w: &mut StreamProducer, ctx: &mut Context, resp: Routed<Response>) -> bool {
    let return_route = resp.return_route();

    match resp.body() {
        Response::Init(Init { stream_name }) => {
            w.route = return_route;
            w.init = true;

            info!(
                "Initialised producer for stream '{}' and route: {}",
                stream_name, w.route
            );

            // Send queued messages
            let mut outbox = core::mem::replace(&mut w.outbox, VecDeque::new());
            outbox.into_iter().for_each(|trans| {
                let route = w.route.clone();
                debug!("Sending queued message to {}", route);
                if let Err(e) = block_future(&ctx.runtime(), async { ctx.send(route, trans).await })
                {
                    error!("Failed to send queued message: {}", e);
                }
            });

            true
        }

        Response::PushConfirm(PushConfirm {
            request_id,
            status,
            index,
        }) => {
            // TODO: handle status == ERROR
            debug!(
                "PushConfirm for request_id: {}, index: {}, status == {:?}",
                request_id.0, index.0, status
            );
            true
        }
        _ => false,
    }
}

#[crate::worker]
impl Worker for StreamProducer {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.parser.attach(ResponseParser::new(parse_response));

        debug!("Create producer stream: {}", self.sender_name);

        // Create a stream for this sender
        ctx.send(
            self.route
                .clone()
                .modify()
                .append(self.stream_service.clone()),
            CreateStreamRequest::new(Some(self.sender_name.clone())),
        )
        .await
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        if let Ok(true) = self.parser.prepare().parse(self, ctx, &msg) {
            return Ok(());
        }

        let mut trans = msg.into_transport_message();
        trans.onward_route.step()?; // Consume THIS address

        let proto_msg = PushRequest::new(self.ids.next() as u64, trans.encode()?);

        if self.init {
            debug!(
                "Sending PushRequest for incoming message to stream {}",
                self.sender_name
            );
            ctx.send(self.route.clone(), proto_msg).await?;
        } else {
            debug!("Stream producer not ready yet, queueing message...");
            self.outbox.push_back(proto_msg);
        }

        Ok(())
    }
}

impl StreamProducer {
    /// When creating a StreamProducer we don't initialise the route
    /// because this will be filled in by the stream consumer which
    /// registers the stream
    pub(crate) fn new(sender_name: String, route: Route, stream_service: String) -> Self {
        Self {
            sender_name,
            route,
            stream_service,
            parser: ProtocolParser::new(),
            outbox: VecDeque::new(),
            ids: Monotonic::new(),
            init: false,
        }
    }
}
