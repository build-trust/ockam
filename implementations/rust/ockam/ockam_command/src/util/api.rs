//! API shim to make it nicer to interact with the ockam messaging API

use minicbor::Decoder;
use ockam::Result;
use ockam_api::{nodes::types::NodeStatus, Method, Request, Response};

/// Construct a request to query node status
pub(crate) fn query_status() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Get, "/node").encode(&mut buf)?;
    Ok(buf)
}

/// Parse the returned status response
pub(crate) fn parse_status(resp: &[u8]) -> Result<NodeStatus> {
    let mut dec = Decoder::new(resp);
    let _ = dec.decode::<Response>()?;
    Ok(dec.decode::<NodeStatus>()?)
}
