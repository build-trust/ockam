//! This example demonstrates how to create a simple node with
//! workers, without invoking the ockam::node attribute macro!

use ockam::{Context, Message, Result, Worker};
use serde::{Deserialize, Serialize};

struct Printer;

/// Create a message our worker should be able to respond to
#[derive(Serialize, Deserialize)]
struct PrintMessage(String);

impl PrintMessage {
    fn new(s: &str) -> Self {
        Self(s.into())
    }
}

// Auto-implement the print message type
impl Message for PrintMessage {}

// Implement optional functions: initialize, shutdown, and handle_message
impl Worker for Printer {
    type Context = Context;
    type Message = PrintMessage;

    fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        println!("Creating printer worker with address: `{}`", ctx.address());
        Ok(())
    }

    fn shutdown(&mut self, context: &mut Self::Context) -> Result<()> {
        println!("Shutting down print worker `{}`!", context.address());
        Ok(())
    }

    fn handle_message(&mut self, context: &mut Self::Context, msg: Self::Message) -> Result<()> {
        println!("Printer({}): {}", context.address(), msg.0);
        Ok(())
    }
}

fn main() {
    let (ctx, mut e) = ockam::node();
    e.execute(async move {
        let addr = String::from("io.ockam.printer1");

        ctx.node()
            .start_worker(addr.clone(), Printer)
            .await
            .unwrap();

        ctx.node()
            .send_message(addr.clone(), PrintMessage::new("Hello ockam!"))
            .await
            .unwrap();

        ctx.node()
            .send_message(addr.clone(), PrintMessage::new("How are you?"))
            .await
            .unwrap();

        // ctx.node().stop_worker(addr.clone()).await.unwrap();
        ctx.node().stop().await.unwrap();
    })
    .unwrap();
}
