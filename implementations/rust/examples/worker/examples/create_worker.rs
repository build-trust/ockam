use ockam::{Context, Result, Worker};

struct Printer;
impl Worker for Printer {
    type Context = Context;

    fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        println!("Initializing Printer.");
        Ok(())
    }
}

#[ockam::node]
async fn main(context: Context) {
    let node = context.node();

    node.start_worker("printer", Printer {}).await.unwrap();

    node.stop().await.unwrap();
}
