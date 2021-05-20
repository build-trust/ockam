use crate::{
    protocols::ProtocolParser, stream::StreamWorkerCmd, Any, Context, Result, Route, Routed, Worker,
};

pub struct StreamProducer {
    parser: ProtocolParser<Self>,
    peer: Route,
    stream: String,
}

#[crate::worker]
impl Worker for StreamProducer {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        if let Ok(true) = self.parser.prepare().parse(self, ctx, &msg) {
            return Ok(());
        }

        // Reaching this point means that it is _probably_ a user message
        let trans = msg.into_transport_message();
        let msg = StreamWorkerCmd::fwd(trans);

        ctx.send(self.peer.clone(), msg).await
    }
}
