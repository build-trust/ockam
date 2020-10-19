use ockam_common::commands::ockam_commands::OckamCommand;
use ockam_message::message::*;
//use ockam_router::router::{Direction, Receiver};
use std::str;
use std::str::FromStr;

pub mod worker {
    use crate::worker_manager::WorkerManager;
    use ockam_message::message::{Message, Receiver, Route, Sender};
    use ockam_router::router::Direction;
    use std::sync::{Arc, Mutex};

    pub struct SampleWorker {
        pub route: Route,
        pub payload: String,
        pub router: Arc<Mutex<dyn Sender + 'static>>,
    }

    impl Receiver for SampleWorker {
        fn recv(&mut self, m: Message) -> Result<Option<Message>, String> {
            //           self.payload = std::str::from_utf8(&m.message_body).unwrap().into();
            println!("{}", self.payload);
            Ok(None)
        }
    }
}
