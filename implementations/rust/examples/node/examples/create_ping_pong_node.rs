//! Spawn to workers that play some ping-pong

use async_trait::async_trait;
use ockam::{Address, Context, Result, Worker};
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

#[async_trait]
impl Worker for Player {
    type Message = Action;
    type Context = Context;

    fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        println!("Starting player {}", ctx.address());
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Action) -> Result<()> {
        println!("{}: {:?}", ctx.address(), msg);
        match msg {
            Action::Intro(addr) if self.friend.is_none() => {
                self.friend = Some(addr);
                ctx.send_message(self.friend(), Action::Intro(ctx.address()))
                    .await
                    .unwrap();
            }

            // Redundant intro -> start the game
            Action::Intro(_) => ctx.send_message(self.friend(), Action::Ping).await.unwrap(),

            // Ping -> Pong
            Action::Ping if self.count < 5 => {
                ctx.send_message(self.friend(), Action::Pong).await.unwrap();
                self.count += 1;
            }

            // Pong -> Ping
            Action::Pong if self.count < 5 => {
                ctx.send_message(self.friend(), Action::Ping).await.unwrap();
                self.count += 1;
            }

            // When the count >= 5
            _ => ctx.stop().await.unwrap(),
        }

        Ok(())
    }
}

fn main() {
    let (ctx, mut exe) = ockam::start_node();

    exe.execute(async move {
        let a1: Address = "player1".into();
        let a2: Address = "player2".into();

        // Create two players
        ctx.start_worker(a1.clone(), Player::new()).await.unwrap();
        ctx.start_worker(a2.clone(), Player::new()).await.unwrap();

        // Tell player1 to start the match with player2
        ctx.send_message(a1, Action::Intro(a2)).await.unwrap();

        // Block until all workers are done
    })
    .unwrap();
}
