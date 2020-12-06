use ockam_node::Node;
use std::rc::Rc;
use std::cell::RefCell;
use ockam_print_worker::PrintWorker;

fn main() {
    // create node
    let mut node = Node::new().unwrap();
    // 4. Now create the worker(s) and register them with the worker manager
    let mut print_worker =
        Rc::new(RefCell::new(PrintWorker::new("aabbccdd".into(), "text".into())));

    node.register_worker("aabbccdd".into(), Some(print_worker.clone()), Some(print_worker.clone()));

    node.run();
}
