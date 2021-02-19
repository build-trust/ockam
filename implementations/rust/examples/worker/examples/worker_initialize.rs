use ockam::{Context, Result, Worker};

struct Nothing;

impl Worker for Nothing {
    type Message = ();
    type Context = Context;

    fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        println!("Worker that does nothing is starting");
        Ok(())
    }
}

fn main() {
    let (app, mut exe) = ockam::start_node();

    exe.execute(async move {
        app.start_worker("io.ockam.nothing", Nothing).await.unwrap();
        app.stop().await.unwrap();
    })
    .unwrap();
}
