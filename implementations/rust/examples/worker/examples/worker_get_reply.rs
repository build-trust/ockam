use ockam::{Context, Result, Worker};
use serde::{Deserialize, Serialize};

struct Square;

#[derive(Serialize, Deserialize)]
struct Num(usize);

impl Worker for Square {
    type Message = Num;
    type Context = Context;

    fn handle_message(&mut self, ctx: &mut Context, msg: Num) -> Result<()> {
        println!("Getting square request for number {}", msg.0);
        ctx.send_message("app", Num(msg.0 * msg.0))
    }
}

fn main() {
    let (mut app, mut exe) = ockam::start_node();

    exe.execute(async move {
        app.start_worker("io.ockam.square", Square).unwrap();

        let num = 3;
        app.send_message("io.ockam.square", Num(num)).unwrap();

        // block until it receives a message
        let square = app.receive::<Num>().unwrap();
        println!("App: {} ^ 2 = {}", num, square.0);

        app.stop().unwrap();
    })
    .unwrap();
}
