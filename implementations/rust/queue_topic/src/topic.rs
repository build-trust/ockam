use crate::queue::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub trait Topic {
    fn topic_address(&self) -> String;
}

pub struct MemTopic {
    pub topic_address: String,
}

impl MemTopic {
    pub fn create(topic_name: String) -> TopicHandle {
        Rc::new(Box::new(MemTopic {
            topic_address: topic_name,
        }))
    }
}

impl Topic for MemTopic {
    fn topic_address(&self) -> String {
        self.topic_address.clone()
    }
}

pub type TopicHandle = Rc<Box<dyn Topic>>;

#[derive(Clone)]
pub struct TopicSubscription {
    pub topic: String,
    pub queue: Rc<RefCell<Box<dyn Queue>>>,
    pub subscriber_address: String,
}

type TopicSubscriptionHandle = RefCell<Box<TopicSubscription>>;

impl TopicSubscription {
    fn create<S>(topic: S, queue: QueueHandle, subscriber_address: S) -> TopicSubscriptionHandle
    where
        S: ToString,
    {
        RefCell::new(Box::new(TopicSubscription {
            topic: topic.to_string(),
            queue: queue.clone(),
            subscriber_address: subscriber_address.to_string(),
        }))
    }
}

pub trait Subscription {
    fn topic(&self) -> &str;

    fn queue(&self) -> QueueHandle;

    fn enqueue(&mut self, message: QueueMessage);

    fn dequeue(&mut self) -> Option<QueueMessage>;

    fn subscriber_address(&self) -> String;
}

impl Subscription for TopicSubscription {
    fn topic(&self) -> &str {
        &self.topic
    }

    fn queue(&self) -> QueueHandle {
        self.queue.clone()
    }

    fn enqueue(&mut self, message: QueueMessage) {
        self.queue.borrow_mut().enqueue(message);
    }

    fn dequeue(&mut self) -> Option<QueueMessage> {
        self.queue.borrow_mut().dequeue()
    }

    fn subscriber_address(&self) -> String {
        self.subscriber_address.clone()
    }
}

type SubscriptionHandle = RefCell<Box<dyn Subscription>>;

pub trait TopicWorker {
    fn publish(&mut self, topic: &str, message: QueueMessage);

    fn subscribe(&mut self, topic: &str) -> Option<String>;

    fn poll(&mut self, subscriber: &str, handler: &dyn Fn(&QueueMessage));

    fn unsubscribe(&mut self, subscriber: &str);
}

pub struct MemTopicWorker {
    queue_worker: RefCell<Box<dyn QueueWorker>>,
    subscriptions: HashMap<String, SubscriptionHandle>,
    subscription_id_counter: usize,
}

type TopicWorkerHandle = RefCell<Box<dyn TopicWorker>>;

impl MemTopicWorker {
    pub fn new(queue_worker: QueueWorkerHandle) -> MemTopicWorker {
        MemTopicWorker {
            subscriptions: HashMap::new(),
            subscription_id_counter: 0,
            queue_worker,
        }
    }

    pub fn create(queue_worker: QueueWorkerHandle) -> TopicWorkerHandle {
        RefCell::new(Box::new(MemTopicWorker::new(queue_worker)))
    }
}

impl TopicWorker for MemTopicWorker {
    fn publish(&mut self, topic: &str, message: QueueMessage) {
        for subscriber in self.subscriptions.values() {
            let mut sub = subscriber.borrow_mut();
            if sub.topic() == topic {
                sub.enqueue(message.clone());
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
                let sub = TopicSubscription::create(topic_address, queue, &subscriber_address);

                self.subscriptions
                    .insert(subscriber_address.clone(), sub.clone());
                self.subscription_id_counter += 1;
                Some(subscriber_address.clone())
            }
            _ => None,
        }
    }

    fn poll(&mut self, subscriber: &str, handler: &dyn Fn(&QueueMessage)) {
        match self.subscriptions.get(subscriber) {
            Some(sub) => {
                match sub.borrow_mut().dequeue() {
                    Some(message) => {
                        handler(&message);
                    }
                    _ => (),
                };
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
    let queue_worker = MemQueueWorker::create();
    let topic_worker = RefCell::new(MemTopicWorker::new(queue_worker));

    let mut tw = topic_worker.borrow_mut();
    let sub = tw.subscribe("test").unwrap();

    let message = QueueMessage::from_str("ockam");
    tw.publish("test", message);

    tw.publish("test", QueueMessage::from_str("ockam!"));

    tw.poll(&sub, &|m| assert_eq!("ockam".as_bytes().to_vec(), m.body));

    tw.poll(&sub, &|m| assert_eq!("ockam!".as_bytes().to_vec(), m.body));

    tw.unsubscribe(&sub);
}
