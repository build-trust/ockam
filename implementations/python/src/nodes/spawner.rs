use super::PyWorker;
use std::sync::Arc;

use crate::errors::ockam_error;
use crate::nodes::started_agents::StartedAgents;

use crate::nodes::logging::py_debug;
use ockam::access_control::{AllowAll, IncomingAccessControl, OutgoingAccessControl};
use ockam::{Address, Context, ContextRouter, Result, Routed, Worker, WorkerBuilder, route};
use pyo3::{PyObject, Python};

pub struct Spawner {
    agent_name: String,
    worker_constructor: Arc<PyObject>,
    key_extractor: Arc<PyObject>,
    started_agents: StartedAgents,
    outgoing_ac: Arc<dyn OutgoingAccessControl>,
}

#[ockam::worker]
impl Worker for Spawner {
    type Message = String;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let local_msg = msg.local_message().clone();
        let message = msg.into_body()?;

        let key_extractor = self.key_extractor.clone();

        let key: Option<String> = ctx
            .runtime()
            .spawn_blocking(move || {
                Python::with_gil(|py| key_extractor.call1(py, (message,))?.extract(py))
                    .map_err(ockam_error)
            })
            .await
            .unwrap()?;

        let key = match key {
            Some(k) => format!("{}/{}", self.agent_name, k),
            None => self.agent_name.clone(),
        };

        let worker_address = match self.started_agents.get_worker(&self.agent_name, &key).await {
            Some(address) => address,
            None => {
                let address = self.create_worker(ctx).await?;
                self.started_agents
                    .add_worker(&self.agent_name, key, &address)
                    .await;
                address
            }
        };

        ctx.forward(local_msg.set_onward_route(route![worker_address]))
            .await
    }
}

impl Spawner {
    pub fn start(
        ctx: &ContextRouter,
        agent_name: &str,
        worker_constructor: PyObject,
        key_extractor: PyObject,
        started_agents: StartedAgents,
        incoming_ac: Arc<dyn IncomingAccessControl>,
        outgoing_ac: Arc<dyn OutgoingAccessControl>,
    ) -> Result<()> {
        let agent_name = agent_name.to_string();
        let address: Address = agent_name.clone().into();
        let worker = Self {
            agent_name,
            worker_constructor: Arc::new(worker_constructor),
            key_extractor: Arc::new(key_extractor),
            started_agents,
            outgoing_ac,
        };

        WorkerBuilder::new(worker)
            .with_address(address)
            .with_incoming_access_control_arc(incoming_ac)
            // TODO: Because of the current topology spawner only sends messages to the
            //  workers it spawned itself, so no need to have access control here.
            .with_outgoing_access_control(AllowAll)
            .start_using_router_context(ctx)?;

        Ok(())
    }

    async fn create_worker(&self, ctx: &mut Context) -> ockam::Result<Address> {
        let address = Address::random_tagged(&self.agent_name);
        let agent_name = self.agent_name.clone();

        let worker_constructor = self.worker_constructor.clone();
        let worker = ctx
            .runtime()
            .spawn_blocking(move || {
                Python::with_gil(|py| {
                    py_debug(
                        py,
                        format!("The agent '{}' is creating a new worker", agent_name),
                    )?;
                    worker_constructor.call0(py)
                })
                .map_err(ockam_error)
            })
            .await
            .unwrap()?;

        WorkerBuilder::new(PyWorker::new(worker))
            .with_address(address.clone())
            .with_outgoing_access_control_arc(self.outgoing_ac.clone())
            .start(ctx)
            .map_err(ockam_error)?;

        Ok(address)
    }
}
