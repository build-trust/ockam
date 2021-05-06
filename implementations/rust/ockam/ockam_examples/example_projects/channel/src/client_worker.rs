use ockam::{Address, Context, Result, Route, Routed, SecureChannel, Worker};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use tokio::time::Duration;
use tracing::info;

const XX_CHANNEL_LISTENER_ADDRESS: &str = "xx_channel_listener";

pub struct Client {
    route: Option<Route>,
    hub_addr: String,
    hub_handle: String,
    vault_worker_address: Address,
}

impl Client {
    pub fn new(hub_addr: &str, hub_handle: String, vault_worker_address: Address) -> Self {
        Client {
            route: None,
            hub_addr: hub_addr.to_string(),
            hub_handle,
            vault_worker_address,
        }
    }
}

#[ockam::worker]
impl Worker for Client {
    type Context = Context;
    type Message = String;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        info!("Starting channel");

        let channel_info = SecureChannel::create(
            ctx,
            Route::new()
                .append(format!("1#{}", self.hub_addr))
                .append(format!("0#{}", self.hub_handle))
                .append(XX_CHANNEL_LISTENER_ADDRESS.to_string()),
            &self.vault_worker_address,
        )
        .await?;
        info!("Key exchange completed");
        self.route = Some(
            Route::new()
                .append(channel_info.address())
                .append("echo_server")
                .into(),
        );

        ctx.send(ctx.address(), "recursion".to_string()).await?;

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        let str = msg.body();
        match str.as_str() {
            "recursion" => {
                let rand_string: String = thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(30)
                    .map(char::from)
                    .collect();
                info!("Client sent message: {}", rand_string);
                ctx.send(ctx.address(), "recursion".to_string()).await?;
                ctx.send(self.route.clone().unwrap(), rand_string).await?;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            _ => info!("Client received msg: {}", str),
        }
        Ok(())
    }
}
