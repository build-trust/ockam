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
use std::collections::VecDeque;

use super::StreamCmdParser;

pub struct StreamProducer {
    parser: ProtocolParser<Self>,
    outbox: VecDeque<ProtocolPayload>,
    ids: Monotonic,
    tx_name: String,
    peer: Route,
    /// Keep track of whether this producer has been initialised
    ///
    /// The reason for this is that `peer` first is the Route to the
    /// `stream_service`, and later to the exact stream worker for
    /// this stream
    init: bool,
}

fn parse_response(w: &mut StreamProducer, ctx: &mut Context, resp: Routed<Response>) -> bool {
    let return_route = resp.return_route();

    match resp.body() {
        Response::Init(Init { stream_name }) => {
            w.peer = return_route;
            w.init = true;

            // Send queued messages
            let mut outbox = std::mem::replace(&mut w.outbox, VecDeque::new());
            outbox.into_iter().for_each(|trans| {
                let peer = w.peer.clone();
                debug!("Sending queued message to {}", peer);
                if let Err(e) = block_future(&ctx.runtime(), async { ctx.send(peer, trans).await })
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

        debug!("Create producer stream: {}", self.tx_name);

        // Create a stream for this sender
        ctx.send(
            self.peer.clone().modify().append("stream_service"),
            CreateStreamRequest::new(Some(self.tx_name.clone())),
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
                self.tx_name
            );
            ctx.send(self.peer.clone(), proto_msg).await?;
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
    pub(crate) fn new(tx_name: String, peer: Route) -> Self {
        Self {
            parser: ProtocolParser::new(),
            ids: Monotonic::new(),
            outbox: VecDeque::new(),
            tx_name,
            peer,
            init: false,
        }
    }
}
