//! API shim to make it nicer to interact with the ockam messaging API

// TODO: maybe we can remove this cross-dependency inside the CLI?
use crate::{portal, transport};
use minicbor::Decoder;

use clap::Args;
use ockam::{Error, OckamError, Result};
use ockam_api::{
    cloud::CloudRequestWrapper, multiaddr_to_route, nodes::types::*, Method, Request, Response,
};
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;

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
pub(crate) fn create_secure_channel(addr: MultiAddr) -> Result<Vec<u8>> {
    let payload = CreateSecureChannelRequest::new(addr);

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/secure_channel")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create Secure Channel Listeners
pub(crate) fn create_secure_channel_listener(addr: &str) -> Result<Vec<u8>> {
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

/// Helpers to create enroll API requests
pub(crate) mod enroll {
    use crate::enroll::*;
    use anyhow::anyhow;
    use ockam_api::auth::types::Attributes;
    use ockam_api::cloud::enroll::*;

    use super::*;

    pub(crate) fn auth0(cmd: EnrollCommand) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Post, "v0/enroll/auth0")
            .body(CloudRequestWrapper::bare(cmd.cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn token_generate(cmd: GenerateEnrollmentTokenCommand) -> anyhow::Result<Vec<u8>> {
        let mut attributes = Attributes::new();
        for entry in cmd.attrs.chunks(2) {
            if let [k, v] = entry {
                attributes.put(k, v.as_bytes());
            } else {
                return Err(anyhow!("{entry:?} is not a key-value pair"));
            }
        }

        let mut buf = vec![];
        Request::builder(Method::Get, "v0/enroll/token")
            .body(CloudRequestWrapper::new(attributes, cmd.cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn token_authenticate(cmd: EnrollCommand) -> anyhow::Result<Vec<u8>> {
        // Option checked that is Some at enroll/mod/EnrollCommand::run
        let token = cmd.token.as_ref().expect("required");
        let b = Token::new(token);
        let mut buf = vec![];
        Request::builder(Method::Put, "v0/enroll/token")
            .body(CloudRequestWrapper::new(b, cmd.cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }
}

/// Helpers to create spaces API requests
pub(crate) mod space {
    use crate::space::*;
    use ockam_api::cloud::space::*;

    use super::*;

    pub(crate) fn create(cmd: CreateCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let b = CreateSpace::new(cmd.name.as_str());
        let mut buf = vec![];
        Request::builder(Method::Post, "v0/spaces")
            .body(CloudRequestWrapper::new(b, cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn list(_cmd: ListCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Get, "v0/spaces")
            .body(CloudRequestWrapper::bare(cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn show(cmd: ShowCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Get, format!("v0/spaces/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn delete(cmd: DeleteCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Delete, format!("v0/spaces/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }
}

/// Helpers to create projects API requests
pub(crate) mod project {
    use crate::project::*;
    use ockam_api::cloud::project::*;

    use super::*;

    pub(crate) fn create(cmd: CreateCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let b = CreateProject::new(cmd.project_name.as_str(), &cmd.services);
        let mut buf = vec![];
        Request::builder(Method::Post, format!("v0/spaces/{}/projects", cmd.space_id))
            .body(CloudRequestWrapper::new(b, cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn list(cmd: ListCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Get, format!("v0/spaces/{}/projects", cmd.space_id))
            .body(CloudRequestWrapper::bare(cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn show(cmd: ShowCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(
            Method::Get,
            format!("v0/spaces/{}/projects/{}", cmd.space_id, cmd.project_id),
        )
        .body(CloudRequestWrapper::bare(cloud_opts.addr))
        .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn delete(cmd: DeleteCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(
            Method::Delete,
            format!("v0/spaces/{}/projects/{}", cmd.space_id, cmd.project_id),
        )
        .body(CloudRequestWrapper::bare(cloud_opts.addr))
        .encode(&mut buf)?;
        Ok(buf)
    }
}

/// Helpers to create invitations API requests
pub(crate) mod invitations {
    use crate::invitation::*;
    use ockam_api::cloud::invitation::*;

    use super::*;

    pub(crate) fn create(cmd: CreateCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        let b = CreateInvitation::new(cmd.email, cmd.space_id, cmd.project_id);
        Request::builder(Method::Post, "v0/invitations")
            .body(CloudRequestWrapper::new(b, cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn list(_cmd: ListCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Get, "v0/invitations")
            .body(CloudRequestWrapper::bare(cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn accept(cmd: AcceptCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Put, format!("v0/invitations/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn reject(cmd: RejectCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Delete, format!("v0/invitations/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cloud_opts.addr))
            .encode(&mut buf)?;
        Ok(buf)
    }
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

pub(crate) fn parse_create_identity_response(
    resp: &[u8],
) -> Result<(Response, CreateIdentityResponse<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((response, dec.decode::<CreateIdentityResponse>()?))
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

////////////// !== share CLI args

#[derive(Clone, Debug, Args)]
pub struct CloudOpts {
    /// Ockam cloud node's secure channel address
    #[clap(long, default_value = "listener")]
    pub addr: Address,
}
