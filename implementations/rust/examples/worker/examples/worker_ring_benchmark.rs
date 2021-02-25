use chrono::{DateTime, Utc};
use ockam::{async_worker, Address, Context, Result, Worker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct RingMessage(DateTime<Utc>);

struct RingWorker {
    ctr: usize,
    next: Option<Address>,
}

#[async_worker]
impl Worker for RingWorker {
    type Context = Context;
    type Message = RingMessage;

    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        msg: Self::Message,
    ) -> Result<()> {
        self.ctr += 1;
        if self.ctr <= 1024 {
            context
                .send_message(self.next.as_ref().unwrap().clone(), msg)
                .await?;
        } else {
            let now = Utc::now();
            println!(
                "Worker ring took {}ms to execute",
                (now - msg.0).num_milliseconds()
            );
            context.stop().await?;
        }

        Ok(())
    }
}

#[ockam::node]
async fn main(ctx: Context) {
    // Create worker state with each worker having a 'next' address
    let mut workers: Vec<RingWorker> = (0..8).fold(vec![], |mut vec, x| {
        let w = RingWorker { ctr: 0, next: None };

        if let Some(prev) = vec.last_mut() {
            prev.next = Some(format!("io.ockam.ring{}", x).into());
        }
        vec.push(w);
        vec
    });

    // Update the last worker to have the first worker as 'next'
    if let Some(last) = workers.last_mut() {
        last.next = Some(format!("io.ockam.ring0").into());
    }

    // Start all the workers
    for (idx, worker) in workers.into_iter().enumerate() {
        let addr: Address = format!("io.ockam.ring{}", idx).into();
        ctx.start_worker(addr, worker).await.unwrap();
    }

    // Create the first message in the system
    let msg = RingMessage(Utc::now());
    ctx.send_message("io.ockam.ring0", msg).await.unwrap();
}
