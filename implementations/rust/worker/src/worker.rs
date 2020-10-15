use ockam_message::message::*;
use ockam_router::router::{Direction, Routable};
use std::str;
use std::str::FromStr;

pub struct Worker {
    pub payload: String,
}

impl Routable for Worker {
    fn handle_message(&mut self, m: Message, d: Direction) -> Option<(Message, Direction)> {
        self.payload = str::from_utf8(&m.message_body).unwrap().into();
        println!("{}", self.payload);
        None
    }
}
