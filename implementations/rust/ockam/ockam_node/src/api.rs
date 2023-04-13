#![allow(missing_docs)]

use crate::{Context, MessageSendReceiveOptions};
use core::fmt::Display;
use minicbor::Encode;
use ockam_core::api::RequestBuilder;
use ockam_core::compat::vec::Vec;
use ockam_core::{LocalInfo, Result, Route};

#[cfg(feature = "tag")]
use {
    cddl_cat::context::BasicContext,
    ockam_core::api::{assert_request_match, merged_cddl},
    once_cell::race::OnceBox,
};

#[cfg(feature = "tag")]
pub fn cddl() -> &'static BasicContext {
    static INSTANCE: OnceBox<BasicContext> = OnceBox::new();
    INSTANCE.get_or_init(|| Box::new(merged_cddl(&[]).unwrap()))
}

/// Encode request header and body (if any), send the package to the server and returns its response.
pub async fn request<T>(
    ctx: &Context,
    label: &str,
    #[allow(unused_variables)] struct_name: impl Into<Option<&str>>,
    route: impl Into<Route> + Display,
    req: RequestBuilder<'_, T>,
) -> Result<Vec<u8>>
where
    T: Encode<()>,
{
    request_with_options(
        ctx,
        label,
        struct_name,
        route,
        req,
        MessageSendReceiveOptions::new(),
    )
    .await
}

/// Encode request header and body (if any), send the package to the server and returns its response.
pub async fn request_with_options<T>(
    ctx: &Context,
    label: &str,
    #[allow(unused_variables)] struct_name: impl Into<Option<&str>>,
    route: impl Into<Route> + Display,
    req: RequestBuilder<'_, T>,
    options: MessageSendReceiveOptions,
) -> Result<Vec<u8>>
where
    T: Encode<()>,
{
    let buf = req.to_vec()?;
    #[cfg(feature = "tag")]
    assert_request_match(struct_name, &buf, cddl());
    trace! {
        target:  "ockam_node",
        id     = %req.header().id(),
        method = ?req.header().method(),
        path   = %req.header().path(),
        body   = %req.header().has_body(),
        "-> {label}"
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
    label: &str,
    #[allow(unused_variables)] struct_name: impl Into<Option<&str>>,
    route: impl Into<Route> + Display,
    req: RequestBuilder<'_, T>,
) -> Result<(Vec<u8>, Vec<LocalInfo>)>
where
    T: Encode<()>,
{
    let route = route.into();
    let mut buf = Vec::new();
    req.encode(&mut buf)?;
    #[cfg(feature = "tag")]
    assert_request_match(struct_name, &buf, cddl());
    trace! {
        target:  "ockam_api",
        id     = %req.header().id(),
        method = ?req.header().method(),
        path   = %req.header().path(),
        body   = %req.header().has_body(),
        "-> {label}"
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
