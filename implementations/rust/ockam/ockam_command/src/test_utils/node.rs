use assert_cmd::prelude::*;
use duct::cmd;
use once_cell::sync::OnceCell;
use rand::random;
use std::cell::Cell;
use std::sync::{Arc, Mutex};

static NODE_POOL: OnceCell<Arc<Mutex<()>>> = OnceCell::new();

pub struct NodePool;

impl NodePool {
    fn init() -> Arc<Mutex<()>> {
        let expr = cmd!(super::ockam_bin(), "node", "delete", "-af")
            .stdout_null()
            .run()
            .unwrap();
        expr.assert().success();
        std::thread::sleep(std::time::Duration::from_millis(250));
        Arc::new(Mutex::new(()))
    }

    pub fn pull() -> TestNode {
        let node_name = hex::encode(&random::<[u8; 4]>());
        TestNode::new(node_name)
    }
}

pub struct TestNode {
    name: String,
    init: Cell<bool>,
}

impl TestNode {
    fn new(name: String) -> Self {
        Self {
            name,
            init: Cell::new(false),
        }
    }

    pub fn name(&self) -> &str {
        self.try_init();
        &self.name
    }

    fn try_init(&self) {
        if !self.init.get() {
            self.init.set(true);
            let lock = NODE_POOL.get_or_init(NodePool::init).lock().unwrap();
            let expr = cmd!(super::ockam_bin(), "node", "create", &self.name)
                .stdout_null()
                .run()
                .unwrap();
            expr.assert().success();
            drop(lock);
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}
