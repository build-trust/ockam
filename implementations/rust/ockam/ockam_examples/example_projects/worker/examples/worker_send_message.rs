use ockam::{Context, Result, Routed, Worker};
use serde::{Deserialize, Serialize};

struct Printer;

// Types that are Serialize + Deserialize are automatically Message
#[derive(Debug, Serialize, Deserialize)]
struct PrintMessage(String);

#[ockam::worker]
impl Worker for Printer {
    type Message = PrintMessage;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        println!("[PRINTER]: starting");
        Ok(())
    }

    async fn handle_message(
        &mut self,
        _context: &mut Context,
        msg: Routed<PrintMessage>,
    ) -> Result<()> {
        println!("[{:?}]: {}", msg.sender(), msg.0);
        Ok(())
    }
}

fn main() {
    let (mut app, mut exe) = ockam::NodeBuilder::without_access_control().build();

    exe.execute(async move {
        app.start_worker("io.ockam.printer", Printer {})
            .await
            .unwrap();
        app.send("io.ockam.printer", PrintMessage("Hello, ockam!".into()))
            .await
            .unwrap();
        app.stop().await.unwrap();
    })
    .unwrap();
}
