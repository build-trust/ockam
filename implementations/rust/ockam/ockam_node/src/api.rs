#![allow(missing_docs)]

use crate::Context;
use core::fmt::Display;
use core::time::Duration;
use minicbor::Encode;
use ockam_core::api::RequestBuilder;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{
    Address, AllowAll, AllowOnwardAddress, LocalInfo, Mailbox, Mailboxes, Result, Route,
};

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
pub async fn request<T, R>(
    ctx: &Context,
    label: &str,
    #[allow(unused_variables)] struct_name: impl Into<Option<&str>>,
    route: R,
    req: RequestBuilder<'_, T>,
) -> Result<Vec<u8>>
where
    T: Encode<()>,
    R: Into<Route> + Display,
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
    let vec: Vec<u8> = ctx.send_and_receive(route, buf).await?;
    Ok(vec)
}

/// Encode request header and body (if any), send the package to the server and returns its response.
pub async fn request_with_timeout<T, R>(
    ctx: &Context,
    label: &str,
    #[allow(unused_variables)] struct_name: impl Into<Option<&str>>,
    route: R,
    req: RequestBuilder<'_, T>,
    timeout: Duration,
) -> Result<Vec<u8>>
where
    T: Encode<()>,
    R: Into<Route> + Display,
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
    let vec: Vec<u8> = ctx
        .send_and_receive_with_timeout(route, buf, timeout)
        .await?;
    Ok(vec)
}

/// Encode request header and body (if any), send the package to the server and returns its response.
pub async fn request_with_local_info<T, R>(
    ctx: &Context,
    label: &str,
    #[allow(unused_variables)] struct_name: impl Into<Option<&str>>,
    route: R,
    req: RequestBuilder<'_, T>,
) -> Result<(Vec<u8>, Vec<LocalInfo>)>
where
    T: Encode<()>,
    R: Into<Route> + Display,
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
    let next = route.next()?.clone();
    let mailboxes = Mailboxes::new(
        Mailbox::new(
            Address::random_tagged("api.request_with_local_info"),
            Arc::new(AllowAll), // FIXME: @ac there is no way to ensure that we're receiving response from the worker we sent request to
            Arc::new(AllowOnwardAddress(next)),
        ),
        vec![],
    );
    let mut child_ctx = ctx.new_detached_with_mailboxes(mailboxes).await?;
    child_ctx.send(route, buf).await?;
    let resp = child_ctx.receive::<Vec<u8>>().await?;
    let local_info = resp.local_message().local_info().to_vec();
    let body = resp.body();

    Ok((body, local_info))
}
