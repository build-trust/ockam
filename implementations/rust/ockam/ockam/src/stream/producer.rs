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
    peer: Option<Route>,
    outbox: VecDeque<ProtocolPayload>,
    ids: Monotonic,
}

fn parse_cmd(
    w: &mut StreamProducer,
    ctx: &mut Context,
    cmd: Routed<StreamWorkerCmd>,
) -> Result<bool> {
    match cmd.body() {
        StreamWorkerCmd::Init { peer } => {
            info!("Initialising stream producer with route {}", peer);
            w.peer = Some(peer);

            // Send queued messages
            let mut outbox = std::mem::replace(&mut w.outbox, VecDeque::new());
            outbox.into_iter().for_each(|trans| {
                let peer = w.peer.clone().unwrap();
                debug!("Sending queued message to {}", peer);
                if let Err(e) = block_future(&ctx.runtime(), async { ctx.send(peer, trans).await })
                {
                    error!("Failed to send queued message: {}", e);
                }
            });

            Ok(true)
        }
        _ => Ok(false),
    }
}

fn parse_response(w: &mut StreamProducer, ctx: &mut Context, resp: Routed<Response>) -> bool {
    match resp.body() {
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

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.parser.attach(ResponseParser::new(parse_response));
        self.parser.attach(StreamCmdParser::new(parse_cmd));

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        if let Ok(true) = self.parser.prepare().parse(self, ctx, &msg) {
            return Ok(());
        }

        let trans = msg.into_transport_message();
        let proto_msg = PushRequest::new(self.ids.next() as u64, trans.encode()?);
        match self.peer {
            Some(ref route) => {
                debug!("Sending PushRequest for incoming message to {}", route);
                ctx.send(route.clone(), proto_msg).await?;
            }
            None => {
                debug!("Stream producer not ready yet, queueing message...");
                self.outbox.push_back(proto_msg);
            }
        }

        Ok(())
    }
}

impl StreamProducer {
    /// When creating a StreamProducer we don't initialise the route
    /// because this will be filled in by the stream consumer which
    /// registers the stream
    pub(crate) fn new() -> Self {
        Self {
            parser: ProtocolParser::new(),
            peer: None,
            outbox: VecDeque::new(),
            ids: Monotonic::new(),
        }
    }
}
