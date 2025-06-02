use ockam::errcode::{Kind, Origin};
use pyo3::PyErr;
use std::fmt::{Debug, Display};

pub fn py_error<E: Display>(e: E) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Error: {e}"))
}

pub fn ockam_error<E: Debug>(e: E) -> ockam::Error {
    ockam::Error::new(Origin::Other, Kind::Unknown, format!("Error: {:?}", e))
}
