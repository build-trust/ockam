use ockam::{Context, Error, Result, Worker as WorkerTrait};

struct Supervisor;

impl WorkerTrait for Supervisor {
    type Context = Context;
    type Message = ();

    fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        // During initialisation, start another worker
        println!("[SV]: Starting a new worker...");
        ctx.supervise("io.ockam.worker", Worker)
    }

    fn handle_failures(&mut self, ctx: &mut Self::Context, _msg: Error) {
        // Stop the node
        println!("[SV]: Shutting down node then...");
        ctx.stop().unwrap();
    }
}

struct Worker;

impl WorkerTrait for Worker {
    type Context = Context;
    type Message = ();

    fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        // But this worker is broken
        println!("[W] I can't work!");
        Err(Error::new(1, "broken"))
    }
}

#[ockam::node]
async fn main(ctx: Context) {
    ctx.start_worker("io.ockam.supervisor", Supervisor).unwrap();
}
