
use crate::queue::*;
use core::cell::RefCell;
use alloc::rc::Rc;
use alloc::string::{String, ToString};

use hashbrown::HashMap;

pub trait Topic {
    fn topic_address(&self) -> String;
}

pub struct MemTopic {
    pub topic_address: String,
}

impl MemTopic {
    pub fn create<T>(topic_name: T) -> TopicHandle where T: ToString {
        Rc::new(RefCell::new(MemTopic {
            topic_address: topic_name.to_string(),
        }))
    }
}

impl Topic for MemTopic {
    fn topic_address(&self) -> String {
        self.topic_address.clone()
    }
}

pub type TopicHandle = Rc<RefCell<dyn Topic>>;

#[derive(Clone)]
pub struct MemSubscription {
    pub topic: String,
    pub queue: Rc<RefCell<dyn Queue<QueueMessage>>>,
    pub subscriber_address: String,
}

type MemSubscriptionHandle = Rc<RefCell<MemSubscription>>;

impl MemSubscription {
    fn create<S>(topic: S, queue: QueueHandle, subscriber_address: S) -> MemSubscriptionHandle
    where
        S: ToString,
    {
        Rc::new(RefCell::new(MemSubscription {
            topic: topic.to_string(),
            queue: queue.clone(),
            subscriber_address: subscriber_address.to_string(),
        }))
    }
}

pub trait Subscription {
    fn topic(&self) -> &str;

    fn queue(&self) -> QueueHandle;

    fn subscriber_address(&self) -> String;
}

impl Subscription for MemSubscription {
    fn topic(&self) -> &str {
        &self.topic
    }

    fn queue(&self) -> QueueHandle {
        self.queue.clone()
    }

    fn subscriber_address(&self) -> String {
        self.subscriber_address.clone()
    }
}

type SubscriptionHandle = Rc<RefCell<dyn Subscription>>;

pub trait TopicWorker {
    fn publish(&mut self, topic: &str, message: QueueMessage);

    fn subscribe(&mut self, topic: &str) -> Option<String>;

    fn consume_messages(&mut self, subscriber: &str, handler: &dyn Fn(&QueueMessage));

    fn unsubscribe(&mut self, subscriber: &str);
}

pub struct MemTopicWorker {
    queue_worker: Rc<RefCell<dyn QueueWorker>>,
    subscriptions: HashMap<String, SubscriptionHandle>,
    subscription_id_counter: usize,
}

type TopicWorkerHandle = Rc<RefCell<dyn TopicWorker>>;

impl MemTopicWorker {
    pub fn new(queue_worker: QueueWorkerHandle) -> MemTopicWorker {
        MemTopicWorker {
            subscriptions: HashMap::new(),
            subscription_id_counter: 0,
            queue_worker,
        }
    }

    pub fn create(queue_worker: QueueWorkerHandle) -> TopicWorkerHandle {
        Rc::new(RefCell::new(MemTopicWorker::new(queue_worker)))
    }
}

impl TopicWorker for MemTopicWorker {
    fn publish(&mut self, topic: &str, message: QueueMessage) {
        for subscriber in self.subscriptions.values() {
            let sub = subscriber.borrow_mut();
            if sub.topic() == topic {
                sub.queue().borrow_mut().enqueue(message.clone());
            }
        }
    }

    fn subscribe(&mut self, topic_address: &str) -> Option<String> {
        let subscriber_address = format!("{}_{}", self.subscription_id_counter, topic_address);
        match self
            .queue_worker
            .borrow_mut()
            .get_queue(subscriber_address.as_str())
        {
            Some(queue) => {
                let sub = MemSubscription::create(topic_address, queue, &subscriber_address);

                self.subscriptions
                    .insert(subscriber_address.clone(), sub.clone());
                self.subscription_id_counter += 1;
                Some(subscriber_address.clone())
            }
            _ => None,
        }
    }

    fn consume_messages(&mut self, subscriber: &str, handler: &dyn Fn(&QueueMessage)) {
        match self.subscriptions.get(subscriber) {
            Some(sub) => {
                let q = sub.borrow().queue();
                let mut queue = q.borrow_mut();
                while queue.has_messages() {
                    match queue.dequeue() {
                        Some(message) => {
                            handler(&message);
                        }
                        _ => (),
                    };
                }
            }
            _ => (),
        }
    }

    fn unsubscribe(&mut self, subscriber: &str) {
        self.subscriptions.remove(subscriber);
    }
}

#[test]
fn topic_tdd() {
    use core::str::FromStr;

    let queue_worker = MemQueueWorker::create();
    let topic_worker = MemTopicWorker::create(queue_worker);

    let mut tw = topic_worker.borrow_mut();
    let sub = tw.subscribe("test").unwrap();

    tw.publish("test",  QueueMessage::from_str("ockam").unwrap());

    tw.publish("test", QueueMessage::from_str("ockam!").unwrap());

    // TODO jds allow for additional captured scope/context for the callback

    tw.consume_messages(&sub, &|_m,|{ });

    tw.unsubscribe(&sub);

}
