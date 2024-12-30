use crate::nodes::models::workers::{WorkerList, WorkerStatus};
use crate::nodes::NodeManagerWorker;
use ockam_core::api::{Error, Response};
use ockam_core::Result;
use ockam_node::Context;

impl NodeManagerWorker {
    /// Return the current list of workers
    pub async fn list_workers(
        &self,
        ctx: &Context,
    ) -> Result<Response<WorkerList>, Response<Error>> {
        let list = ctx
            .list_workers()?
            .into_iter()
            .map(|addr| WorkerStatus::new(addr.address()))
            .collect();

        Ok(Response::ok().body(WorkerList::new(list)))
    }
}
