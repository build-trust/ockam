use ockam::{Context, Result, Routed, Worker};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Serialize, Deserialize)]
enum Message {
    Good,
    Bad,
}

/// This worker requests more data and is very picky about what data it accepts.
struct Picky;

#[ockam::worker]
impl Worker for Picky {
    type Message = Message;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Message>) -> Result<()> {
        match *msg {
            Message::Good => {
                println!("[PICKY]: I got a good message!  I want another one");
                ctx.send("io.ockam.echo", Message::Good).await?;

                loop {
                    let msg = ctx.receive::<Message>().await.unwrap();
                    if msg == Message::Bad {
                        println!("[PICKY]: Ignoring bad message");
                        msg.cancel().await?;
                    } else {
                        println!("[PICKY]: Yay, another good message!");
                        break;
                    }
                }
            }
            Message::Bad => {
                println!("[PICKY]: Oh, a bad message...");
                ctx.stop().await?;
            }
        }

        Ok(())
    }
}

struct Echo;

#[ockam::worker]
impl Worker for Echo {
    type Message = Message;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, _: Routed<Message>) -> Result<()> {
        println!("[ECHO]: Received message: sending one Bad, then one Good");
        ctx.send("io.ockam.picky", Message::Bad).await?;
        ctx.send("io.ockam.picky", Message::Good).await?;
        Ok(())
    }
}

#[ockam::node]
async fn main(app: Context) -> Result<()> {
    app.start_worker("io.ockam.picky", Picky).await?;
    app.start_worker("io.ockam.echo", Echo).await?;

    app.send("io.ockam.picky", Message::Good).await?;

    Ok(())
}
