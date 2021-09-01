use ockam::{stream_responses::*, worker, Any, Context, ProtocolParser, Result, Routed, Worker};

#[derive(Default)]
struct MyWorker {
    parser: ProtocolParser<MyWorker>,
    stream: Option<String>,
}

/// Util function that maps stream-protocol responses to worker state
fn handle_stream(w: &mut MyWorker, _: &mut Context, r: Routed<Response>) -> bool {
    match r.body() {
        Response::Init(Init { stream_name }) => w.stream = Some(stream_name),
        _ => {}
    }
    true // XXX(thom): unsure if true or false is what this should be returning...
}

#[worker]
impl Worker for MyWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, _: &mut Context) -> Result<()> {
        self.parser.attach(ResponseParser::new(handle_stream));

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        self.parser.prepare().parse(self, ctx, &msg)?;

        println!("Stream name is now: `{:?}`", self.stream);
        ctx.stop().await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.start_worker("worker", MyWorker::default()).await?;

    // Send an Init message to our worker -- this message would
    // normally be sent from the Ockam Hub stream service
    ctx.send("worker", Init::new("test-stream")).await?;

    Ok(())
}
