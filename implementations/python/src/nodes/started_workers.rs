use crate::nodes::WorkerType;
use ockam::compat::asynchronous::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Default)]
pub(crate) struct StartedWorkers {
    workers: Arc<RwLock<HashMap<String, StartedWorker>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct StartedWorker {
    pub(crate) name: String,
    pub(crate) worker_type: WorkerType,
}

impl StartedWorker {
    pub fn is_internal(&self) -> bool {
        matches!(self.worker_type, WorkerType::InternalWorker)
    }
}

impl StartedWorkers {
    pub(crate) async fn started(&self, name: String, worker_type: WorkerType) {
        let started_worker = StartedWorker {
            name: name.clone(),
            worker_type,
        };
        let mut guard = self.workers.write().await;
        guard.insert(name, started_worker);
    }

    pub(crate) async fn stopped(&self, name: &str) {
        self.workers.write().await.remove(name);
    }

    pub(crate) async fn list_workers(&self) -> Vec<StartedWorker> {
        self.workers.read().await.values().cloned().collect()
    }
}
