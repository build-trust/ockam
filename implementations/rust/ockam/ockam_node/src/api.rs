use crate::Context;
use core::fmt::Display;
use minicbor::Encode;
use ockam_core::api::{assert_request_match, RequestBuilder};
use ockam_core::compat::vec::Vec;
use ockam_core::{Result, Route};

/// Encode request header and body (if any), send the package to the server and returns its response.
pub async fn request<T, R>(
    ctx: &mut Context,
    label: &str,
    struct_name: impl Into<Option<&str>>,
    route: R,
    req: RequestBuilder<'_, T>,
) -> Result<Vec<u8>>
where
    T: Encode<()>,
    R: Into<Route> + Display,
{
    let mut buf = Vec::new();
    req.encode(&mut buf)?;
    assert_request_match(struct_name, &buf);
    trace! {
        target:  "ockam_api",
        id     = %req.header().id(),
        method = ?req.header().method(),
        path   = %req.header().path(),
        body   = %req.header().has_body(),
        "-> {label}"
    };
    // TODO: Check IdentityId here?
    let vec: Vec<u8> = ctx.send_and_receive(route, buf).await?;
    Ok(vec)
}
