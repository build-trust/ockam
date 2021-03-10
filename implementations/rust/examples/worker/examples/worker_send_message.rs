use ockam::{async_worker, Context, Result, Routed, Worker};
use serde::{Deserialize, Serialize};

struct Printer;

// Types that are Serialize + Deserialize are automatically Message
#[derive(Debug, Serialize, Deserialize)]
struct PrintMessage(String);

#[async_worker]
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
    let (app, mut exe) = ockam::start_node();

    exe.execute(async move {
        app.start_worker("io.ockam.printer", Printer {})
            .await
            .unwrap();
        app.send_message("io.ockam.printer", PrintMessage("Hello, ockam!".into()))
            .await
            .unwrap();
        app.stop().await.unwrap();
    })
    .unwrap();
}
