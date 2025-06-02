use pyo3::prelude::*;

mod errors;
mod integrations;
mod nodes;
mod traces;

#[pymodule]
#[pyo3(name = "ockam_in_rust_for_python")]
fn ockam_in_rust_for_python(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<nodes::PyNode>()?;
    module.add_class::<nodes::PyMailbox>()?;

    module.add_class::<integrations::mcp::PyMcpClient>()?;
    module.add_class::<integrations::mcp::PyMcpServer>()?;

    module.add_function(wrap_pyfunction!(traces::info, module)?)?;
    module.add_function(wrap_pyfunction!(traces::error, module)?)?;
    module.add_function(wrap_pyfunction!(traces::warn, module)?)?;
    module.add_function(wrap_pyfunction!(traces::debug, module)?)?;

    Ok(())
}
