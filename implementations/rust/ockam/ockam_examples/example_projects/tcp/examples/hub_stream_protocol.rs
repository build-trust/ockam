use ockam::{
    async_worker,
    protocols::{
        stream::{requests::*, responses::*},
        ProtocolParser,
    },
    Any, Context, Result, Route, Routed, Worker,
};
use ockam_transport_tcp::{TcpTransport, TCP};

#[derive(Default)]
struct MyWorker {
    parser: ProtocolParser<MyWorker>,
    stream: Option<String>,
    peer: String,
}

impl MyWorker {
    fn new(peer: String) -> Self {
        Self {
            peer,
            ..Default::default()
        }
    }
}

/// Util function that maps stream-protocol responses to worker state
fn handle_stream(w: &mut MyWorker, r: Response) {
    match r {
        Response::Init(Init { stream_name }) => w.stream = Some(stream_name),
        _ => {}
    }
}

#[async_worker]
impl Worker for MyWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        self.parser.attach(ResponseParser::new(handle_stream));

        ctx.send(
            Route::new()
                .append_t(TCP, &self.peer)
                .append("stream_service"),
            CreateStreamRequest::new(None), // Generate a stream name for us please
        )
        .await?;

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        self.parser.prepare().parse(self, msg)?;

        println!("Stream name is now: `{:?}`", self.stream);
        ctx.stop().await
    }
}

fn get_peer_addr() -> String {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        // This value can be used when running the ockam-hub locally
        .unwrap_or(format!("127.0.0.1:4000"))
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let peer = get_peer_addr();

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(peer.clone()).await?;

    ctx.start_worker("worker", MyWorker::new(peer)).await?;

    Ok(())
}
