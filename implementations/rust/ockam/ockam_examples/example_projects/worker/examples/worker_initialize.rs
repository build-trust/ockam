use ockam::{Context, Result, Worker};

struct Nothing;

#[ockam::worker]
impl Worker for Nothing {
    type Message = ();
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        println!("Worker that does nothing is starting");
        Ok(())
    }
}

fn main() {
    let (mut app, mut exe) = ockam::NodeBuilder::new().build();

    exe.execute(async move {
        app.start_worker("io.ockam.nothing", Nothing).await.unwrap();
        app.stop().await.unwrap();
    })
    .unwrap();
}
