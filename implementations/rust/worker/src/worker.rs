#[allow(unused)]
#[allow(dead_code)]
use ockam_message::message::{Message, Receiver, Route, Sender};
use std::sync::{Arc, Mutex};

pub struct SampleWorker {
    pub route: Route,
    pub payload: String,
    pub router: Arc<Mutex<dyn Sender + 'static>>,
}

impl Receiver for SampleWorker {
    fn recv(&mut self, _m: Message) -> Result<Option<Message>, String> {
        //           self.payload = std::str::from_utf8(&m.message_body).unwrap().into();
        println!("{}", self.payload);
        Ok(None)
    }
}
