use ockam::{Context, Message, Result, Routed, Worker};
use serde::{Deserialize, Serialize};

struct Square;

#[derive(Serialize, Deserialize, Message)]
struct Num(usize);

#[ockam::worker]
impl Worker for Square {
    type Message = Num;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Num>) -> Result<()> {
        println!("Getting square request for number {}", msg.0);
        ctx.send(msg.sender(), Num(msg.0 * msg.0)).await
    }
}

fn main() {
    let (mut app, mut exe) = ockam::NodeBuilder::without_access_control().build();

    exe.execute(async move {
        app.start_worker("io.ockam.square", Square).await.unwrap();

        let num = 3;
        app.send("io.ockam.square", Num(num)).await.unwrap();

        // block until it receives a message
        let square = app.receive::<Num>().await.unwrap();
        println!("App: {} ^ 2 = {}", num, square.0);

        app.stop().await.unwrap();
    })
    .unwrap();
}
