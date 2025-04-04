use crate::nodes::models::workers::{WorkerList, WorkerStatus};
use crate::nodes::NodeManagerWorker;
use ockam_core::api::{Error, Response};
use ockam_core::Result;

impl NodeManagerWorker {
    /// Return the current list of workers
    pub async fn list_workers(&self) -> Result<Response<WorkerList>, Response<Error>> {
        let ctx = self.node_manager.tcp_transport.ctx();
        let list = ctx
            .list_workers()?
            .into_iter()
            .map(|addr| WorkerStatus::new(addr.address()))
            .collect();

        Ok(Response::ok().body(WorkerList::new(list)))
    }
}
