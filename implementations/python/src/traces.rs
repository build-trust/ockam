use pyo3::pyfunction;
use tracing::{debug, error, info, warn};

#[pyfunction]
pub fn info(msg: String) {
    info!(msg)
}

#[pyfunction]
pub fn error(msg: String) {
    error!(msg)
}

#[pyfunction]
pub fn warn(msg: String) {
    warn!(msg)
}

#[pyfunction]
pub fn debug(msg: String) {
    debug!(msg)
}
