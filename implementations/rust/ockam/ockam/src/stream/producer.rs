use crate::{
    monotonic::Monotonic,
    protocols::{
        stream::{requests::*, responses::*},
        ProtocolParser, ProtocolPayload,
    },
    Any, Context, OckamError, Result, Route, Routed, Worker,
};
use ockam_core::compat::{boxed::Box, collections::VecDeque, string::String};
use ockam_core::{Decodable, Encodable};

pub struct StreamProducer {
    sender_name: String,
    route: Route,
    stream_service: String,
    // parser: ProtocolParser<Self>,
    outbox: VecDeque<ProtocolPayload>,
    ids: Monotonic,
    /// Keep track of whether this producer has been initialised
    ///
    /// The reason for this is that `route` first is the Route to the
    /// `stream_service`, and later to the exact stream worker for
    /// this stream
    init: bool,
}

async fn handle_response(
    w: &mut StreamProducer,
    ctx: &mut Context,
    route: Routed<Any>,
    response: Response,
) -> Result<()> {
    let return_route = route.return_route();

    match response {
        Response::Init(InitResponse { stream_name }) => {
            w.route = return_route;
            w.init = true;

            info!(
                "Initialised producer for stream '{}' and route: {:?}",
                stream_name, w.route
            );

            // Send queued messages
            let outbox = core::mem::take(&mut w.outbox);
            for trans in outbox.into_iter() {
                let route = w.route.clone();
                debug!("Sending queued message to {:?}", route);
                ctx.send(route, trans).await?;
            }

            Ok(())
        }

        Response::PushConfirm(PushConfirm {
            request_id,
            status,
            index,
        }) => {
            // TODO: handle status == ERROR
            debug!(
                "PushConfirm for request_id: {}, index: {}, status == {:?}",
                request_id.u64(),
                index.u64(),
                status
            );
            Ok(())
        }
        _ => Err(OckamError::NoSuchProtocol.into()),
    }
}

#[crate::worker]
impl Worker for StreamProducer {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        debug!("Create producer stream: {}", self.sender_name);

        // Create a stream for this sender
        ctx.send(
            self.route
                .clone()
                .modify()
                .try_append(self.stream_service.clone())?,
            CreateStreamRequest::new(Some(self.sender_name.clone())),
        )
        .await
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        if let Ok(pp) = ProtocolPayload::decode(msg.payload()) {
            let id = pp.protocol.as_str();

            if Response::check_id(id) {
                let response = Response::parse(pp)?;
                return handle_response(self, ctx, msg, response).await;
            }
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
            outbox: VecDeque::new(),
            ids: Monotonic::new(),
            init: false,
        }
    }
}
