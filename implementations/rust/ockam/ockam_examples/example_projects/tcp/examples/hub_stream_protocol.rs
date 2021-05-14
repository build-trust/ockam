use ockam::{
    protocols::{
        stream::{requests::*, responses::*},
        ProtocolParser,
    },
    Any, Context, DelayedEvent, Result, Route, Routed, Worker,
};
use ockam_transport_tcp::{TcpTransport, TCP};

struct MyWorker {
    parser: ProtocolParser<MyWorker>,
    stream: Option<String>,
    peer: Route,
}

impl MyWorker {
    fn new(peer: Route) -> Self {
        Self {
            parser: ProtocolParser::new(),
            stream: None,
            peer,
        }
    }
}

/// Util function that maps stream-protocol responses to worker state
fn handle_stream(w: &mut MyWorker, r: Routed<Response>) {
    match &*r {
        Response::Init(Init { stream_name }) => {
            w.stream = Some(stream_name.clone());
            w.peer = r.return_route();
        }
        Response::PushConfirm(PushConfirm {
            request_id,
            status,
            index,
        }) => {
            println!(
                "req_id: {}, status: {:?}, index: {}",
                request_id, status, index
            );
        }
        Response::PullResponse(PullResponse {
            request_id,
            messages,
        }) => {
            println!(
                "Requestid: {}, num messages: {}",
                request_id,
                messages.len()
            );
        }
        _ => {}
    }
}

#[ockam::worker]
impl Worker for MyWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        self.parser.attach(ResponseParser::new(handle_stream));

        // Generate a stream name for us please
        ctx.send(self.peer.clone(), CreateStreamRequest::new(None))
            .await?;

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        self.parser.prepare().parse(self, &msg)?;

        println!("Stream return route is now: `{:?}`", self.peer);
        ctx.send(self.peer.clone(), PushRequest::new(5, vec![1, 3, 5, 7]))
            .await?;

        // Start a delayed event to pull messages too!
        DelayedEvent::new(&ctx, self.peer.clone(), PullRequest::new(5, 0, 2))
            .await?
            .seconds(2)
            .spawn();

        Ok(())
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

    ctx.start_worker(
        "worker",
        MyWorker::new(
            Route::new()
                .append_t(TCP, &peer)
                .append("stream_service")
                .into(),
        ),
    )
    .await?;

    Ok(())
}
