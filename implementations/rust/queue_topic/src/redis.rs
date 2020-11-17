use crate::queue::{Queue, QueueHandle, QueueMessage, QueueWorker, QueueWorkerHandle};
use simple_redis::client::Client;
use std::cell::RefCell;
use std::rc::Rc;
use simple_redis::RedisResult;

pub struct RedisQueue {
    address: String,
    client: Rc<RefCell<Client>>,
}

impl Queue for RedisQueue {
    fn address(&self) -> String {
        self.address.clone()
    }

    fn enqueue(&mut self, message: QueueMessage) {
        match String::from_utf8(message.body) {
            Ok(s) => match self.client.borrow_mut().lpush(&self.address, s.as_str()) {
                Ok(_) => (),
                Err(e) => {
                    println!("Redis enqueue failure: {}", e)
                }
            },
            _ => (),
        };
    }

    fn dequeue(&mut self) -> Option<QueueMessage> {
        let message: RedisResult<String> = self.client.borrow_mut().rpop(&self.address);
        match message {
            Ok(message) => Some(QueueMessage {
                body: message.into_bytes(),
            }),
            Err(_) => None,
        }
    }
}

pub struct RedisQueueWorker {
    client: Rc<RefCell<Client>>,
}

impl RedisQueueWorker {
    pub fn create(url: &str) -> Option<QueueWorkerHandle> {
        match simple_redis::create(url) {
            Ok(client) => Some(RefCell::new(Box::new(RedisQueueWorker {
                client: Rc::new(RefCell::new(client)),
            }))),
            Err(_) => None,
        }
    }
}

impl QueueWorker for RedisQueueWorker {
    fn get_queue(&mut self, queue_address: &str) -> Option<QueueHandle> {
        Some(Rc::new(RefCell::new(Box::new(RedisQueue {
            client: self.client.clone(),
            address: queue_address.to_string(),
        }))))
    }

    fn remove_queue(&mut self, queue_name: &str) {
        match self.client.borrow_mut().ltrim(queue_name, -1, 0) {
            Err(e) => { println!("Redis remove failed: {}", e)},
            _ => ()
        }
    }
}

#[test]
pub fn redis_tdd() {
    use crate::topic::MemTopicWorker;

    let queue_worker = RedisQueueWorker::create("redis://127.0.0.1:6379/").unwrap();
    let topic_worker = MemTopicWorker::create(queue_worker);

    let mut tw = topic_worker.borrow_mut();
    let sub = tw.subscribe("test").unwrap();

    let message = QueueMessage::from_str("ockam");
    tw.publish("test", message);

    tw.publish("test", QueueMessage::from_str("ockam!"));

    tw.poll(&sub, &|m| assert_eq!("ockam".as_bytes().to_vec(), m.body));

    tw.poll(&sub, &|m| assert_eq!("ockam!".as_bytes().to_vec(), m.body));

    tw.unsubscribe(&sub);
}
