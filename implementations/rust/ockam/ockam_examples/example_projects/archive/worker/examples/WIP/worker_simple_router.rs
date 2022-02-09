#[macro_use]
extern crate tracing;

use ockam::{Address, Context, Result, Route, Routed, RouterMessage, Worker};
use std::collections::BTreeMap;

/// A simple external router
#[derive(Default)]
struct Router {
    routes: BTreeMap<Address, Address>,
}

#[ockam::worker]
impl Worker for Router {
    // Routers must handle `RouterMessage` provided by ockam_core, to
    // create a consistent interface across all router implementations
    type Message = RouterMessage;
    type Context = Context;

    async fn initialize(&mut self, _: &mut Context) -> Result<()> {
        info!("Starting router...");
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<RouterMessage>,
    ) -> Result<()> {
        let msg = msg.body();

        use RouterMessage::*;
        match msg {
            // Handle requests to route messages to the next hop in
            // the route.  This will usually be some kind of
            // domain-specific connection worker.
            Route(mut msg) => {
                info!("Router route request: {}", msg.onward_route.next().unwrap());
                let onward = msg.onward_route.step().unwrap();

                // Look up the next address in the local routing table
                let next = self.routes.get(&onward).unwrap();

                // Modify the transport message route
                msg.onward_route.modify().prepend(next.clone());
                msg.return_route.modify().prepend(onward);

                // Forward the message to the next hop
                ctx.forward(msg).await?;
            }
            // Handle new domain-specific worker registrations.  The
            // `accepts` address is the one provided by the message
            // sender (in this case `10#proxy_me_0:...`).  The
            // self_addr is local worker address used by the router to
            // forward the message.
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

#[ockam::worker]
impl Worker for Consumer {
    type Message = String;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        info!("Starting consumer '{}'...", ctx.address());

        // Register this consumer with the router by its accept scope
        // (which is used in a look-up table in the router to forward
        // messages), as well as its own address.
        //
        // The accept scope is address type `10`, with a prefix and
        // its own address.  Messages sent to this address will be
        // sent to the 10-type router, which will then forward them.
        ctx.send(
            "simple.router",
            RouterMessage::Register {
                accepts: format!("10#proxy_me_{}", ctx.address()).into(),
                self_addr: ctx.address(),
            },
        )
        .await?;

        ctx.send("app", String::from("")).await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        info!("Consumer {}: {}", msg.msg_addr(), msg);
        ctx.send("app", String::from("")).await?;
        Ok(())
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a simple router worker
    ctx.start_worker("simple.router", Router::default()).await?;

    // Register it with the internal node router.  This ensures that
    // addresses with type = 10 will be handled by this worker.
    ctx.register(10, "simple.router").await?;

    // Now we create two consumer workers that will register
    // themselves with the domain specific router
    ctx.start_worker("cons1", Consumer).await?;
    ctx.start_worker("cons2", Consumer).await?;

    // Block until the two consumers have reported back as being ready
    let _ = ctx.receive::<String>().await?;
    let _ = ctx.receive::<String>().await?;

    ctx.send(
        Route::new().append_t(10, "proxy_me").append("cons2"),
        String::from("Hello consumer!"),
    )
    .await?;

    // Block until the consumer has received and handled its message
    let _ = ctx.receive::<String>().await?;

    ctx.stop().await?;
    Ok(())
}
