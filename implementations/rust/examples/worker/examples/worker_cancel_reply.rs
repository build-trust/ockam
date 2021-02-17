use ockam::{Context, Result, Worker};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Serialize, Deserialize)]
enum Message {
    Good,
    Bad,
}

/// This worker requests more data and is very picky about what data it accepts.
struct Picky;

impl Worker for Picky {
    type Message = Message;
    type Context = Context;

    fn handle_message(&mut self, ctx: &mut Context, msg: Message) -> Result<()> {
        match msg {
            Message::Good => {
                println!("[PICKY]: I got a good message!  I want another one");
                ctx.send_message("io.ockam.echo", Message::Good).unwrap();

                loop {
                    let msg = ctx.receive::<Message>().unwrap();
                    if msg == Message::Bad {
                        println!("[PICKY]: Ignoring bad message");
                        msg.cancel();
                    } else {
                        println!("[PICKY]: Yay, another good message!");
                        break;
                    }
                }
            }
            Message::Bad => {
                println!("[PICKY]: Oh, a bad message...");
                ctx.stop().unwrap();
            }
        }

        Ok(())
    }
}

struct Echo;

impl Worker for Echo {
    type Message = Message;
    type Context = Context;

    fn handle_message(&mut self, ctx: &mut Context, _: Message) -> Result<()> {
        println!("[ECHO]: Received message: sending one Bad, then one Good");
        ctx.send_message("io.ockam.picky", Message::Bad).unwrap();
        ctx.send_message("io.ockam.picky", Message::Good).unwrap();
        Ok(())
    }
}

fn main() {
    let (app, mut exe) = ockam::start_node();

    exe.execute(async move {
        app.start_worker("io.ockam.picky", Picky).unwrap();
        app.start_worker("io.ockam.echo", Echo).unwrap();

        app.send_message("io.ockam.picky", Message::Good).unwrap();
    })
    .unwrap();
}
