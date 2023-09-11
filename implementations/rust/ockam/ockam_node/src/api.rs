#![allow(missing_docs)]

use core::fmt::Display;

use minicbor::Encode;

use ockam_core::api::RequestBuilder;
use ockam_core::compat::vec::Vec;
use ockam_core::{LocalInfo, Result, Route};

use crate::{Context, MessageSendReceiveOptions};

/// Encode request header and body (if any), send the package to the server and returns its response.
pub async fn request<T>(
    ctx: &Context,
    route: impl Into<Route> + Display,
    req: RequestBuilder<T>,
) -> Result<Vec<u8>>
where
    T: Encode<()>,
{
    request_with_options(ctx, route, req, MessageSendReceiveOptions::new()).await
}

/// Encode request header and body (if any), send the package to the server and returns its response.
pub async fn request_with_options<T>(
    ctx: &Context,
    route: impl Into<Route> + Display,
    req: RequestBuilder<T>,
    options: MessageSendReceiveOptions,
) -> Result<Vec<u8>>
where
    T: Encode<()>,
{
    let buf = req.to_vec()?;
    trace! {
        target:  "ockam_node",
        id     = %req.header().id(),
        method = ?req.header().method(),
        path   = %req.header().path(),
        body   = %req.header().has_body(),
    };
    // TODO: Check IdentityId is the same we sent message to?
    // TODO: Check response id matches request id?
    let vec = ctx
        .send_and_receive_extended::<Vec<u8>>(route, buf, options)
        .await?
        .body();
    Ok(vec)
}

/// Encode request header and body (if any), send the package to the server and returns its response.
pub async fn request_with_local_info<T>(
    ctx: &Context,
    route: impl Into<Route> + Display,
    req: RequestBuilder<T>,
) -> Result<(Vec<u8>, Vec<LocalInfo>)>
where
    T: Encode<()>,
{
    let route = route.into();
    let mut buf = Vec::new();
    req.encode(&mut buf)?;
    trace! {
        target:  "ockam_api",
        id     = %req.header().id(),
        method = ?req.header().method(),
        path   = %req.header().path(),
        body   = %req.header().has_body(),
    };

    // TODO: Check IdentityId is the same we sent message to?
    // TODO: Check response id matches request id?
    let resp = ctx
        .send_and_receive_extended::<Vec<u8>>(route, buf, MessageSendReceiveOptions::new())
        .await?;
    let local_info = resp.local_message().local_info().to_vec();
    let body = resp.body();

    Ok((body, local_info))
}
