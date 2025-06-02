use super::PyNode;
use std::time::Duration;

use crate::errors::py_error;

use ockam::compat::asynchronous::RwLock;
use ockam::compat::sync::Arc;
use ockam::{Context, MessageReceiveOptions, MessageSendOptions};

use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;

#[pyclass(name = "Mailbox")]
#[derive(Clone)]
pub struct PyMailbox {
    ctx: Arc<RwLock<Context>>,
    node: PyNode,
}

impl PyMailbox {
    pub fn new(ctx: Context, node: PyNode) -> Self {
        Self {
            ctx: Arc::new(RwLock::new(ctx)),
            node,
        }
    }
}

#[pymethods]
impl PyMailbox {
    #[pyo3(signature = (destination, message, node=None, policy=None))]
    fn send<'a>(
        &self,
        py: Python<'a>,
        destination: String,
        message: String,
        node: Option<String>,
        policy: Option<String>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let policy = self.node.policy(policy).map_err(py_error)?;
        let self_clone = self.clone();
        let node_clone = self.node.clone();
        let node_manager = self.node.node_manager_clone();

        future_into_py(py, async move {
            node_clone
                .with_route(node, destination, move |route| async move {
                    let (_, outgoing_ac) = node_manager
                        .create_abac(node_manager.project_authority(), policy)
                        .await?;

                    let ctx = self_clone.ctx.read().await;
                    ctx.send_extended(
                        route,
                        message,
                        MessageSendOptions::new().with_outgoing_access_control(outgoing_ac),
                    )
                    .await
                })
                .await
                .map_err(py_error)
        })
    }

    #[pyo3(signature = (policy=None, timeout=None))]
    fn receive<'a>(
        &self,
        py: Python<'a>,
        policy: Option<String>,
        timeout: Option<u64>,
    ) -> PyResult<Bound<'a, PyAny>> {
        // TODO: Currently there is no way to use return_route to send a response (if needed)

        // TODO: Receive will hold the context lock until we receive a message (which might be as
        //  long as the timeout argument). Sending messages  will block meanwhile. It's possible
        //  to improve that, but tweaks to the ockam code are required. P.S. ockam itself behaves
        //  the same, but we usually don't wrap Context instances into Arc<RwLock> and don't try
        //  to send&receive from different places at the same time.
        let policy = self.node.policy(policy).map_err(py_error)?;
        let self_clone = self.clone();
        let node_manager = self.node.node_manager_clone();

        future_into_py(py, async move {
            let (incoming_ac, _) = node_manager
                .create_abac(node_manager.project_authority(), policy)
                .await
                .map_err(py_error)?;

            let options = MessageReceiveOptions::new().with_incoming_access_control(incoming_ac);

            let options = if let Some(timeout) = timeout {
                options.with_timeout(Duration::from_secs(timeout))
            } else {
                options
            };

            let mut ctx = self_clone.ctx.write().await;
            let result = ctx
                .receive_extended::<String>(options)
                .await
                .map_err(py_error)?;

            result.into_body().map_err(py_error)
        })
    }
}
