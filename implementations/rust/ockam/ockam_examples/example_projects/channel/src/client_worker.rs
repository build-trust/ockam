use ockam::{async_worker, Context, Result, Route, Routed, Worker};
use ockam_channel::{Channel, ChannelMessage, KeyExchangeCompleted};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::net::SocketAddr;
use tokio::time::Duration;
use tracing::info;

const XX_CHANNEL_LISTENER_ADDRESS: &str = "xx_channel_listener";

pub struct Client {
    channel_id: String,
    hub_addr: SocketAddr,
    hub_handle: String,
}

impl Client {
    pub fn new(channel_id: String, hub_addr: SocketAddr, hub_handle: String) -> Self {
        Client {
            channel_id,
            hub_addr,
            hub_handle,
        }
    }
}

#[async_worker]
impl Worker for Client {
    type Context = Context;
    type Message = String;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        info!("Starting channel");

        Channel::start_initiator_channel(
            &ctx,
            self.channel_id.clone(),
            Route::new()
                .append(format!("1#{}", self.hub_addr))
                .append(format!("0#{}", self.hub_handle))
                .append(XX_CHANNEL_LISTENER_ADDRESS.to_string()),
        )
        .await?;

        let _ = ctx
            .receive_match(|m: &KeyExchangeCompleted| m.channel_id() == self.channel_id)
            .await?;
        info!("Key exchange completed");

        ctx.send_message(ctx.address(), "recursion".to_string())
            .await?;

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        let str = msg.take();
        match str.as_str() {
            "recursion" => {
                let rand_string: String = thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(30)
                    .map(char::from)
                    .collect();
                info!("Client sent message: {}", rand_string);
                ctx.send_message(ctx.address(), "recursion".to_string())
                    .await?;
                ctx.send_message(
                    Route::new()
                        .append(self.channel_id.clone())
                        .append("echo_server"),
                    ChannelMessage::encrypt(rand_string)?,
                )
                .await?;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            _ => info!("Client received msg: {}", str),
        }
        Ok(())
    }
}
