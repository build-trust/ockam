use ockam::{
    block_future, stream_requests::*, stream_responses::*, Any, Context, ProtocolParser, Result,
    Route, Routed, Worker,
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
fn handle_stream(w: &mut MyWorker, ctx: &mut Context, r: Routed<Response>) -> bool {
    match &*r {
        Response::Init(Init { stream_name }) => {
            w.stream = Some(stream_name.clone());
            w.peer = r.return_route();

            println!("Init Ok! Sending a PushRequest for vec![1, 3, 5, 7]");

            block_future(
                &ctx.runtime(),
                ctx.send(w.peer.clone(), PushRequest::new(5, vec![1, 3, 5, 7])),
            )
            .unwrap();
            true
        }
        Response::PushConfirm(PushConfirm {
            request_id,
            status,
            index,
        }) => {
            println!(
                "PushConfirm req_id: {:?}, status: {:?}, index: {:?}",
                request_id, status, index
            );

            block_future(
                &ctx.runtime(),
                ctx.send(w.peer.clone(), PullRequest::new(0, 0, 8)),
            )
            .unwrap();
            true
        }

        Response::PullResponse(PullResponse {
            request_id,
            messages,
        }) => {
            println!(
                "PullResponse req_id: {:?}, num messages: {}",
                request_id,
                messages.len()
            );
            true
        }
        _ => false,
    }
}

#[ockam::worker]
impl Worker for MyWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        self.parser.attach(ResponseParser::new(handle_stream));

        // Generate a stream name for us please
        ctx.send(self.peer.clone(), dbg!(CreateStreamRequest::new(None)))
            .await?;

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        self.parser.prepare().parse(self, ctx, &msg)?;
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
