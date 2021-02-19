use async_trait::async_trait;
use ockam::{Context, Result, Worker};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Serialize, Deserialize)]
enum Message {
    Good,
    Bad,
}

/// This worker requests more data and is very picky about what data it accepts.
struct Picky;

#[async_trait]
impl Worker for Picky {
    type Message = Message;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Message) -> Result<()> {
        match msg {
            Message::Good => {
                println!("[PICKY]: I got a good message!  I want another one");
                ctx.send_message("io.ockam.echo", Message::Good)
                    .await
                    .unwrap();

                loop {
                    let msg = ctx.receive::<Message>().await.unwrap();
                    if msg == Message::Bad {
                        println!("[PICKY]: Ignoring bad message");
                        msg.cancel().await;
                    } else {
                        println!("[PICKY]: Yay, another good message!");
                        break;
                    }
                }
            }
            Message::Bad => {
                println!("[PICKY]: Oh, a bad message...");
                ctx.stop().await.unwrap();
            }
        }

        Ok(())
    }
}

struct Echo;

#[async_trait]
impl Worker for Echo {
    type Message = Message;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, _: Message) -> Result<()> {
        println!("[ECHO]: Received message: sending one Bad, then one Good");
        ctx.send_message("io.ockam.picky", Message::Bad)
            .await
            .unwrap();
        ctx.send_message("io.ockam.picky", Message::Good)
            .await
            .unwrap();
        Ok(())
    }
}

fn main() {
    let (app, mut exe) = ockam::start_node();

    exe.execute(async move {
        app.start_worker("io.ockam.picky", Picky).await.unwrap();
        app.start_worker("io.ockam.echo", Echo).await.unwrap();

        app.send_message("io.ockam.picky", Message::Good)
            .await
            .unwrap();
    })
    .unwrap();
}
