#![allow(dead_code)]

use pyo3::prelude::{PyAnyMethods, PyModule};
use pyo3::{PyResult, Python};

/// Log an INFO message using Python's logging
pub fn py_info(py: Python, msg: impl ToString) -> PyResult<()> {
    py_log(py, "info", msg)
}

/// Log a WARNING message using Python's logging
pub fn py_warning(py: Python, msg: impl ToString) -> PyResult<()> {
    py_log(py, "warning", msg)
}

/// Log an ERROR message using Python's logging
pub fn py_error(py: Python, msg: impl ToString) -> PyResult<()> {
    py_log(py, "error", msg)
}

/// Log a DEBUG message using Python's logging
pub fn py_debug(py: Python, msg: impl ToString) -> PyResult<()> {
    py_log(py, "debug", msg)
}

/// Log a message using Python's logging
fn py_log(py: Python, level: &str, msg: impl ToString) -> PyResult<()> {
    let logging = PyModule::import(py, "logging")?;
    let logger = logging.call_method1("getLogger", ("node",))?;
    logger.call_method1(level, (msg.to_string(),))?;
    Ok(())
}
