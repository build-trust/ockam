use ockam::{Context, Message, Result, Worker};
use serde::{Deserialize, Serialize};

struct Printer;

#[derive(Debug, Serialize, Deserialize)]
struct PrintMessage(String);

impl Message for PrintMessage {}

impl Worker for Printer {
    type Message = PrintMessage;
    type Context = Context;

    fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        println!("Printer starting");
        Ok(())
    }

    fn handle_message(&mut self, _context: &mut Context, msg: PrintMessage) -> Result<()> {
        println!("{}", msg.0);
        Ok(())
    }
}

fn main() {
    let (ctx, mut exe) = ockam::node();

    exe.execute(async move {
        let node = ctx.node();

        node.start_worker("printer", Printer {}).unwrap();
        node.send_message(
            "printer",
            PrintMessage {
                0: "hi".to_string(),
            },
        )
        .unwrap();
        node.stop().unwrap();
    })
    .unwrap();
}
