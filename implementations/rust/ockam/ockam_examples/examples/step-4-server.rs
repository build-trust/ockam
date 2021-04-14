use ockam::{async_worker, Context, Result, Route, Routed, Worker};
use ockam_transport_tcp::{TcpTransport, TCP};

struct EchoService;

const HUB_ADDRESS: &str = "Paste the address of the node you created on Ockam Hub here.";

#[async_worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.send(
            Route::new()
                .append_t(TCP, HUB_ADDRESS)
                .append("forwarding_service"),
            "register".to_string(),
        )
        .await
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        if "register" == msg.as_str() {
            let address = msg.return_route().recipient().to_string();
            println!(
                "echo_service: My address on the hub is {}",
                address.strip_prefix("0#").unwrap()
            );
            Ok(())
        } else {
            println!("echo_service: {}", msg);
            ctx.send(msg.return_route(), msg.body()).await
        }
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    TcpTransport::create(&ctx, HUB_ADDRESS).await?;

    ctx.start_worker("echo_service", EchoService).await?;

    Ok(())
}
