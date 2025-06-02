use crate::errors::ockam_error;
use ockam::abac::tokio::runtime::Handle;
use ockam::compat::sync::Arc;
use ockam::compat::tokio;
use ockam::errcode::{Kind, Origin};
use once_cell::sync::OnceCell;
use pyo3::types::PyAnyMethods;
use pyo3::{Bound, PyAny, PyErr, PyObject, PyResult, Python};
use pyo3_async_runtimes::TaskLocals;
use std::sync::Mutex;

#[derive(Clone)]
pub struct PythonAsyncExecutor {
    event_loop: Arc<Mutex<Option<PyObject>>>,
}

static PYTHON_EXECUTOR: OnceCell<PythonAsyncExecutor> = OnceCell::new();

impl PythonAsyncExecutor {
    fn get_instance() -> Self {
        PYTHON_EXECUTOR
            .get_or_init(|| Self {
                event_loop: Default::default(),
            })
            .clone()
    }

    fn create_event_loop(py: Python) -> Bound<PyAny> {
        let asyncio = py.import("asyncio").expect("Failed to import asyncio");
        let event_loop = asyncio
            .call_method0("new_event_loop")
            .expect("Failed to call new_event_loop");

        asyncio
            .call_method1("set_event_loop", (event_loop.clone().unbind(),))
            .expect("Failed to call set_event_loop");

        event_loop
    }

    pub async fn run_python_future<F>(
        handle: &Handle,
        py_future_creator: F,
    ) -> ockam::Result<PyObject>
    where
        F: for<'py> FnOnce(Python<'py>) -> Result<PyObject, PyErr>,
        F: Send + 'static,
    {
        let instance = Self::get_instance();
        let future = handle
            .spawn_blocking(move || {
                Python::with_gil(|py| {
                    let event_loop = {
                        instance
                            .event_loop
                            .lock()
                            .unwrap()
                            .as_ref()
                            .map(|e| e.clone_ref(py))
                    };

                    if let Some(event_loop) = event_loop {
                        let locals = TaskLocals::new(event_loop.into_bound(py));
                        let py_future = py_future_creator(py).map_err(ockam_error)?;
                        pyo3_async_runtimes::into_future_with_locals(
                            &locals,
                            py_future.into_bound(py),
                        )
                        .map_err(ockam_error)
                    } else {
                        Err(ockam::Error::new(
                            Origin::Executor,
                            Kind::NotReady,
                            "Event loop isn't initialized",
                        ))
                    }
                })
            })
            .await
            .unwrap();

        future?.await.map_err(ockam_error)
    }

    pub fn run_python_main(py: Python<'_>, py_future: PyObject) -> PyResult<PyObject> {
        let instance = Self::get_instance();

        let event_loop_reference = instance.event_loop.clone();
        let event_loop = Self::create_event_loop(py);
        event_loop_reference
            .lock()
            .unwrap()
            .replace(event_loop.clone().unbind());

        event_loop
            .call_method1("run_until_complete", (py_future,))
            .map(|py_object| py_object.into())
    }
}

static TOKIO_RUNTIME: OnceCell<Arc<tokio::runtime::Runtime>> = OnceCell::new();

fn create_runtime() -> Arc<tokio::runtime::Runtime> {
    Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime"),
    )
}

pub fn get_runtime() -> Arc<tokio::runtime::Runtime> {
    TOKIO_RUNTIME.get_or_init(create_runtime).clone()
}

pub fn get_runtime_ref<'a>() -> &'a tokio::runtime::Runtime {
    TOKIO_RUNTIME.get_or_init(create_runtime)
}
