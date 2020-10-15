use hashbrown::*;
use ockam_message::message::{Address, Message};
use ockam_router::router::{Direction, Routable};
use std::sync::{Arc, Mutex};

pub struct WorkerManager {
    workers: hashbrown::HashMap<String, Arc<Mutex<dyn Routable>>>,
}

impl Routable for WorkerManager {
    fn handle_message(&mut self, m: Message, d: Direction) -> Option<(Message, Direction)> {
        let address = m.onward_route.addresses[0].address.as_string();
        let handler = self.workers.get(&address).unwrap();
        handler
            .lock()
            .unwrap()
            .handle_message(m, Direction::Incoming);
        None
    }
}

impl WorkerManager {
    pub fn new() -> WorkerManager {
        WorkerManager {
            workers: hashbrown::HashMap::new(),
        }
    }

    pub fn register(&mut self, a: Address, r: Arc<Mutex<dyn Routable>>) -> Result<(), String> {
        self.workers.insert(a.as_string(), r);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::Worker;
    use ockam_message::message::Address::WorkerAddress;
    use ockam_message::message::{MessageType, Route, RouterAddress};

    #[test]
    fn test_handle_message() {
        let mut wm = WorkerManager::new();
        let address = Address::worker_address_from_string("00010203").unwrap();
        let m = Message {
            onward_route: Route {
                addresses: vec![RouterAddress::from_address(address.clone()).unwrap()],
            },
            return_route: Route { addresses: vec![] },
            message_type: MessageType::Payload,
            message_body: "hello worker manager".as_bytes().to_vec(),
        };
        let w = Worker {
            payload: "I'm a worker".into(),
        };
        wm.register(address.clone(), Arc::new(Mutex::new(w)));
        wm.handle_message(m, Direction::Incoming);
    }
}
