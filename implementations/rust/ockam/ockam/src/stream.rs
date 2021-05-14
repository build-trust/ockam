#![allow(unused)]

use crate::{
    protocols::{
        stream::{requests::*, responses::*},
        ProtocolParser,
    },
    Address, Any, Context, DelayedEvent, Message, Result, Route, Routed, TransportMessage, Worker,
};
use serde::{Deserialize, Serialize};

/////////////// ^-^ Stream Publisher ^-^ ///////////////

pub struct StreamPublisher {
    parser: ProtocolParser<Self>,
    stream_name: Option<String>,
    /// The route to the publisher peer service on Hub
    peer: Route,
}

fn handle_stream(w: &mut StreamPublisher, r: Routed<Response>) {
    match &*r {
        // When we receive an Init we set the stream name and `peer`,
        // which is the return address to the stream
        Response::Init(Init { stream_name }) => {
            w.stream_name = Some(stream_name.into());
            w.peer = r.return_route();
        }
        _ => {}
    }
}

#[crate::worker]
impl Worker for StreamPublisher {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, _: &mut Context) -> Result<()> {
        self.parser.attach(ResponseParser::new(handle_stream));

        Ok(())
    }

    async fn handle_message(&mut self, _: &mut Context, msg: Routed<Any>) -> Result<()> {
        // Take a user message and send a PUSH to the stream_service on HUB

        Ok(())
    }
}

/////////////// ^-^ Stream Consumer ^-^ ///////////////

pub struct StreamConsumer {
    peer: Route,
    prod: Address,
}

#[derive(Debug, Serialize, Deserialize)]
enum StreamWorkerCmd {
    /// Trigger a new fetch event
    Fetch,
    /// Ensure the producer exists
    Ensure,
    /// The producers address response
    Addr(Address),
}

fn stream_worker_cmd(msg: &TransportMessage) -> Option<StreamWorkerCmd> {
    StreamWorkerCmd::decode(&msg.payload).ok()
}

#[crate::worker]
impl Worker for StreamConsumer {
    type Context = Context;
    type Message = Any;

    // let msg = msg.into_transport_message();

    // // If the message couldn't be parsed via the parser, it must
    // // be a user message and should be forwarded
    // if let Err(_) = self.parser.prepare().parse(self, &msg) {}

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        // Send stream_create
        ctx.send(self.peer.clone(), CreateStreamRequest::new(None))
            .await?;
        let init = ctx.receive::<Init>().await?;

        // Send initial fetch, then go into message loop
        ctx.send(self.peer.clone(), PullRequest::new(0, 0, 4))
            .await?;

        // Create a delayed trigger for this worker
        DelayedEvent::new(&ctx, ctx.address().into(), StreamWorkerCmd::Fetch)
            .await?
            .millis(500)
            .spawn();

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let msg = msg.into_transport_message();

        // on fetch:
        //    - start publisher (?)
        //    - insert publisher address into return_route
        //    - forward message

        match stream_worker_cmd(&msg) {
            Some(StreamWorkerCmd::Fetch) => {
                // Send a PullRequest
                ctx.send(self.peer.clone(), PullRequest::new(0, 0, 4))
                    .await?;

                // Then re-schedule a delayed trigger
                DelayedEvent::new(&ctx, ctx.address().into(), StreamWorkerCmd::Fetch)
                    .await?
                    .millis(500)
                    .spawn();
            }
            Some(cmd) => {
                warn!("Unexpected stream command payload `{:?}`!", cmd);
            }
            _ => {
                // This point means we are _probably_ dealing with a PullResponse!
                let PullResponse { messages, .. } = PullResponse::decode(&msg.payload)?;
            }
        }

        Ok(())
    }
}

/// Start a new pair of workers to manage a bi-directional stream
pub async fn connect<R: Into<Route>>(_: &Context, _: R) -> Result<(Address, Address)> {
    todo!()
}
