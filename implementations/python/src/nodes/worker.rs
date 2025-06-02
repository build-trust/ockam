use super::runtime::PythonAsyncExecutor;

use crate::errors::py_error;

use ockam::compat::sync::Arc;
use ockam::{Context, ContextSend};
use ockam::{Route, Routed, Worker};

use pyo3::{PyObject, Python, prelude::*, pyclass, pymethods};
use pyo3_async_runtimes::tokio::future_into_py;
use tracing::warn;

pub struct PyWorker {
    py_object: Arc<PyObject>,
}

impl PyWorker {
    pub fn new(py_object: PyObject) -> Self {
        Self {
            py_object: Arc::new(py_object),
        }
    }
}

#[ockam::worker]
impl Worker for PyWorker {
    type Message = String;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> ockam::Result<()> {
        let py_object = self.py_object.clone();
        let sending_context = ctx.get_sending_context();

        let return_route = msg.return_route().clone();
        let message: String = msg.into_body()?;
        let worker_context = PyWorkerContext {
            sending_context,
            return_route: return_route.clone(),
        };

        let result =
            PythonAsyncExecutor::run_python_future(ctx.runtime(), move |py: Python<'_>| {
                py_object.call_method1(py, "handle_message", (worker_context, message))
            })
            .await;
        if let Err(error) = result {
            // log each error line in a separate log line
            for line in error.to_string().split("\\n") {
                warn!("{line}");
            }
            return Err(error);
        }

        Ok(())
    }
}

#[pyclass(name = "WorkerContext")]
struct PyWorkerContext {
    sending_context: ContextSend,
    return_route: Route,
}

#[pymethods]
impl PyWorkerContext {
    pub fn reply<'a>(&self, msg: String, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let return_route = self.return_route.clone();
        let context = self.sending_context.clone();
        future_into_py(py, async move {
            context.send(return_route, msg).await.map_err(py_error)
        })
    }
}
