//! API shim to make it nicer to interact with the ockam messaging API

// TODO: maybe we can remove this cross-dependency inside the CLI?
use crate::{portal, transport};
use minicbor::Decoder;

use ockam::{Error, OckamError, Result};
use ockam_api::{multiaddr_to_route, nodes::types::*, Method, Request, Response};

////////////// !== generators

/// Construct a request to query node status
pub(crate) fn query_status() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Get, "/node").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to query node transports
pub(crate) fn query_transports() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Get, "/node/transport").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create node transports
pub(crate) fn create_transport(cmd: &crate::transport::CreateCommand) -> Result<Vec<u8>> {
    // FIXME: this should not rely on CreateCommand internals!
    let (tt, addr) = match &cmd.create_subcommand {
        transport::CreateTypeCommand::TcpConnector { addr } => (TransportMode::Connect, addr),
        transport::CreateTypeCommand::TcpListener { bind } => (TransportMode::Listen, bind),
    };

    let payload = CreateTransport::new(TransportType::Tcp, tt, addr);

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/transport")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to delete node transports
pub(crate) fn delete_transport(cmd: &transport::DeleteCommand) -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Delete, "/node/transport")
        .body(DeleteTransport::new(&cmd.id, cmd.force))
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create a forwarder
pub(crate) fn create_forwarder(cmd: &crate::forwarder::CreateCommand) -> Result<Vec<u8>> {
    let route = multiaddr_to_route(cmd.address()).ok_or_else(|| {
        Error::new(
            ockam::errcode::Origin::Other,
            ockam::errcode::Kind::Invalid,
            "failed to parse address",
        )
    })?;
    let mut buf = vec![];
    Request::builder(Method::Post, "/node/forwarder")
        .body(CreateForwarder::new(route, cmd.alias()))
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create Identity
pub(crate) fn create_identity() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Post, "/node/identity").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create Secure Channels
pub(crate) fn create_secure_channel(
    cmd: &crate::secure_channel::create::CreateSubCommand,
) -> Result<Vec<u8>> {
    let addr = match cmd {
        crate::secure_channel::create::CreateSubCommand::Connector { addr, .. } => addr,
        crate::secure_channel::create::CreateSubCommand::Listener { .. } => panic!(),
    };

    let payload = CreateSecureChannelRequest::new(addr.to_string());

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/secure_channel")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create Secure Channel Listeners
pub(crate) fn create_secure_channel_listener(
    cmd: &crate::secure_channel::create::CreateSubCommand,
) -> Result<Vec<u8>> {
    let addr = match cmd {
        crate::secure_channel::create::CreateSubCommand::Connector { .. } => panic!(),
        crate::secure_channel::create::CreateSubCommand::Listener { bind, .. } => bind,
    };

    let payload = CreateSecureChannelListenerRequest::new(addr);

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/secure_channel_listener")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create node transports
pub(crate) fn create_portal(cmd: &portal::CreateCommand) -> Result<Vec<u8>> {
    // FIXME: this should not rely on CreateCommand internals!
    let (tt, addr, fwd) = match &cmd.create_subcommand {
        portal::CreateTypeCommand::TcpInlet { bind, forward } => {
            let route = multiaddr_to_route(forward).ok_or(OckamError::InvalidParameter)?;
            (PortalType::Inlet, bind, Some(route))
        }
        portal::CreateTypeCommand::TcpOutlet { address } => (PortalType::Outlet, address, None),
    };
    let alias = cmd.alias.as_ref().map(Into::into);
    let fwd = fwd.map(|route| route.to_string().into());
    let payload = CreatePortal::new(tt, addr, fwd, alias);

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/portal")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

////////////// !== parsers

/// Parse the base response without the inner payload
pub(crate) fn parse_response(resp: &[u8]) -> Result<Response> {
    let mut dec = Decoder::new(resp);
    Ok(dec.decode::<Response>()?)
}

/// Parse the returned status response
pub(crate) fn parse_status(resp: &[u8]) -> Result<NodeStatus> {
    let mut dec = Decoder::new(resp);
    let _ = dec.decode::<Response>()?;
    Ok(dec.decode::<NodeStatus>()?)
}

/// Parse the returned status response
pub(crate) fn parse_transport_list(resp: &[u8]) -> Result<TransportList> {
    let mut dec = Decoder::new(resp);
    let _ = dec.decode::<Response>()?;
    Ok(dec.decode::<TransportList>()?)
}

/// Parse the returned status response
pub(crate) fn parse_transport_status(resp: &[u8]) -> Result<(Response, TransportStatus<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((response, dec.decode::<TransportStatus>()?))
}

pub(crate) fn parse_create_identity_response(resp: &[u8]) -> Result<Response> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok(response)
}

pub(crate) fn parse_create_secure_channel_response(
    resp: &[u8],
) -> Result<(Response, CreateSecureChannelResponse<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((response, dec.decode::<CreateSecureChannelResponse>()?))
}

pub(crate) fn parse_create_secure_channel_listener_response(resp: &[u8]) -> Result<Response> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok(response)
}

/// Parse the returned status response
pub(crate) fn parse_portal_status(resp: &[u8]) -> Result<(Response, PortalStatus<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((response, dec.decode::<PortalStatus>()?))
}
