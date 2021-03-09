#[macro_use]
extern crate tracing;

use ockam::{async_worker, Address, Context, Result, Route, RouterMessage, Worker};
use std::collections::BTreeMap;

/// A simple external router
#[derive(Default)]
struct Router {
    routes: BTreeMap<Address, Address>,
}

#[async_worker]
impl Worker for Router {
    type Message = RouterMessage;
    type Context = Context;

    async fn initialize(&mut self, _: &mut Context) -> Result<()> {
        info!("Starting router...");
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: RouterMessage) -> Result<()> {
        use RouterMessage::*;
        match msg {
            Route(mut msg) => {
                info!("Router route request: {}", msg.onward.next().unwrap());
                let onward = msg.onward.step().unwrap();

                // Look up the next address in the local routing table
                let next = self.routes.get(&onward).unwrap();

                // Modify the transport message route
                msg.onward.modify().prepend(next.clone());
                msg.return_.modify().prepend(onward);

                // Forward the message to the next hop
                ctx.forward_message(msg).await?;
            }
            Register { accepts, self_addr } => {
                info!(
                    "Router register: `{}` address to worker `{}`",
                    accepts, self_addr
                );
                self.routes.insert(accepts, self_addr);
            }
        }

        Ok(())
    }
}

/// A connoisseur of strings
struct Consumer;

#[async_worker]
impl Worker for Consumer {
    type Message = String;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        info!("Starting consumer...");

        // Register this consumer with the router by its accept scope
        // (which is used in a look-up table in the router to forward
        // messages), as well as its own address.
        //
        // The accept scope is address type `10`, with a prefix and
        // its own address.  Messages sent to this address will be
        // sent to the 10-type router, which will then forward them.
        ctx.send_message(
            "simple.router",
            RouterMessage::Register {
                accepts: format!("10#proxy_me_{}", ctx.address()).into(),
                self_addr: ctx.address(),
            },
        )
        .await?;

        ctx.send_message("app", String::from("")).await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: String) -> Result<()> {
        info!("Consumer {}: {}", ctx.address(), msg);
        ctx.send_message("app", String::from("")).await?;
        Ok(())
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("trace").init();

    // Create a simple router worker
    ctx.start_worker("simple.router", Router::default()).await?;

    // Register it with the internal node router.  This ensures that
    // addresses with type = 10 will be handled by this worker.
    ctx.register(10, "simple.router").await?;

    // Now we create two consumer workers that will register
    // themselves with the domain specific router
    ctx.start_worker("cons1", Consumer).await?;

    // Block until the consumer has reported back as being ready
    let _ = ctx.receive::<String>().await?;

    ctx.send_message(
        Route::new().append("10#proxy_me_0:cons1"),
        String::from("Hello consumer!"),
    )
    .await?;

    // Block until the consumer has received and handled its message
    let _ = ctx.receive::<String>().await?;

    ctx.stop().await?;
    Ok(())
}
