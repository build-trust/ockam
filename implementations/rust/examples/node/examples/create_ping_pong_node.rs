//! Spawn to workers that play some ping-pong

use ockam::{Address, Context, Message, Result, Worker};
use serde::{Deserialize, Serialize};

struct Player {
    friend: Option<Address>,
    count: u8,
}

impl Player {
    fn new() -> Self {
        Self {
            friend: None,
            count: 0,
        }
    }

    fn friend(&self) -> Address {
        self.friend.clone().unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum Action {
    Intro(Address),
    Ping,
    Pong,
}

impl Message for Action {}

#[async_trait::async_trait]
impl Worker for Player {
    type Context = Context;
    type Message = Action;

    fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        println!("Starting player {}", ctx.address());
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Action) -> Result<()> {
        println!("{}: {:?}", ctx.address(), msg);
        match msg {
            Action::Intro(addr) if self.friend.is_none() => {
                self.friend = Some(addr);
                ctx.node()
                    .send_message(self.friend(), Action::Intro(ctx.address()))
                    .await
                    .unwrap();
            }

            // Redundant intro -> start the game
            Action::Intro(_) => ctx
                .node()
                .send_message(self.friend(), Action::Ping)
                .await
                .unwrap(),

            // Ping -> Pong
            Action::Ping if self.count < 5 => {
                ctx.node()
                    .send_message(self.friend(), Action::Pong)
                    .await
                    .unwrap();
                self.count += 1;
            }

            // Pong -> Ping
            Action::Pong if self.count < 5 => {
                ctx.node()
                    .send_message(self.friend(), Action::Ping)
                    .await
                    .unwrap();
                self.count += 1;
            }

            // When the count >= 5
            _ => ctx.node().stop().await.unwrap(),
        }

        Ok(())
    }
}




fn main() {
    let (ctx, mut exe) = ockam::node();

    exe.execute(async move {
        let a1: Address = "player1".into();
        let a2: Address = "player2".into();

        let node = ctx.node();

        // Create two players
        node.start_worker(a1.clone(), Player::new()).await.unwrap();
        node.start_worker(a2.clone(), Player::new()).await.unwrap();

        // Tell player1 to start the match with player2
        node.send_message(a1, Action::Intro(a2)).await.unwrap();

        // Block until all workers are done
    })
    .unwrap();
}
