//! API shim to make it nicer to interact with the ockam messaging API

use crate::util::DEFAULT_CLOUD_ADDRESS;
// TODO: maybe we can remove this cross-dependency inside the CLI?
use minicbor::Decoder;

use clap::Args;
use ockam::identity::IdentityIdentifier;
use ockam::Result;
use ockam_api::cloud::CloudRequestWrapper;
use ockam_api::nodes::*;
use ockam_core::api::{Method, Request, Response};
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;

////////////// !== generators

/// Construct a request to query node status
pub(crate) fn query_status() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Get, "/node").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to query node tcp connections
pub(crate) fn list_tcp_connections() -> Result<Vec<u8>> {
    let mut buf = vec![];
    let builder = Request::builder(Method::Get, "/node/tcp/connection");
    builder.encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to query node tcp listeners
pub(crate) fn list_tcp_listeners() -> Result<Vec<u8>> {
    let mut buf = vec![];
    let builder = Request::builder(Method::Get, "/node/tcp/listener");
    builder.encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create node tcp connection
pub(crate) fn create_tcp_connection(
    cmd: &crate::tcp::connection::CreateCommand,
) -> Result<Vec<u8>> {
    let (tt, addr) = (
        models::transport::TransportMode::Connect,
        cmd.address.clone(),
    );

    let payload =
        models::transport::CreateTransport::new(models::transport::TransportType::Tcp, tt, addr);
    let mut buf = vec![];
    Request::builder(Method::Post, "/node/tcp/connection")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create node tcp listener
pub(crate) fn create_tcp_listener(cmd: &crate::tcp::listener::CreateCommand) -> Result<Vec<u8>> {
    let (tt, addr) = (
        models::transport::TransportMode::Listen,
        cmd.address.clone(),
    );

    let payload =
        models::transport::CreateTransport::new(models::transport::TransportType::Tcp, tt, addr);
    let mut buf = vec![];
    Request::builder(Method::Post, "/node/tcp/listener")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to delete node tcp connection
pub(crate) fn delete_tcp_connection(
    cmd: &crate::tcp::connection::DeleteCommand,
) -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Delete, "/node/tcp/connection")
        .body(models::transport::DeleteTransport::new(&cmd.id, cmd.force))
        .encode(&mut buf)?;

    Ok(buf)
}

/// Construct a request to delete node tcp connection
pub(crate) fn delete_tcp_listener(cmd: &crate::tcp::listener::DeleteCommand) -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Delete, "/node/tcp/listener")
        .body(models::transport::DeleteTransport::new(&cmd.id, cmd.force))
        .encode(&mut buf)?;

    Ok(buf)
}

/// Construct a request to create a Vault
pub(crate) fn create_vault(path: Option<String>) -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Post, "/node/vault")
        .body(models::vault::CreateVaultRequest::new(path))
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create Identity
pub(crate) fn create_identity() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Post, "/node/identity").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to export Identity
pub(crate) fn long_identity() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Post, "/node/identity/actions/show/long").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to print Identity Id
pub(crate) fn short_identity() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Post, "/node/identity/actions/show/short").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create Secure Channels
pub(crate) fn create_secure_channel(
    addr: MultiAddr,
    authorized_identifiers: Option<Vec<IdentityIdentifier>>,
) -> Result<Vec<u8>> {
    let payload =
        models::secure_channel::CreateSecureChannelRequest::new(&addr, authorized_identifiers);

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/secure_channel")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create Secure Channel Listeners
pub(crate) fn create_secure_channel_listener(
    addr: &Address,
    authorized_identifiers: Option<Vec<IdentityIdentifier>>,
) -> Result<Vec<u8>> {
    let payload = models::secure_channel::CreateSecureChannelListenerRequest::new(
        addr,
        authorized_identifiers,
    );

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/secure_channel_listener")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to list Secure Channel Listeners
pub(crate) fn list_secure_channel_listener() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Get, "/node/secure_channel_listener").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to start a Vault Service
pub(crate) fn start_vault_service(addr: &str) -> Result<Vec<u8>> {
    let payload = models::services::StartVaultServiceRequest::new(addr);

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/services/vault")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to start an Identity Service
pub(crate) fn start_identity_service(addr: &str) -> Result<Vec<u8>> {
    let payload = models::services::StartIdentityServiceRequest::new(addr);

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/services/identity")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to start an Authenticated Service
pub(crate) fn start_authenticated_service(addr: &str) -> Result<Vec<u8>> {
    let payload = models::services::StartAuthenticatedServiceRequest::new(addr);

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/services/authenticated")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Helpers to create enroll API requests
pub(crate) mod enroll {
    use crate::enroll::*;
    use anyhow::anyhow;
    use ockam_api::auth::types::Attributes;
    use ockam_api::cloud::enroll::auth0::{Auth0Token, AuthenticateAuth0Token};
    use ockam_api::cloud::enroll::*;

    use super::*;

    pub(crate) fn auth0(cmd: EnrollCommand, token: Auth0Token) -> anyhow::Result<Vec<u8>> {
        let token = AuthenticateAuth0Token::new(token);
        let mut buf = vec![];
        Request::builder(Method::Post, "v0/enroll/auth0")
            .body(CloudRequestWrapper::new(token, cmd.cloud_opts.route()))
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
            .body(CloudRequestWrapper::new(attributes, cmd.cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn token_authenticate(cmd: EnrollCommand) -> anyhow::Result<Vec<u8>> {
        // Option checked that is Some at enroll/mod/EnrollCommand::run
        let token = cmd.token.as_ref().expect("required");
        let b = Token::new(token);
        let mut buf = vec![];
        Request::builder(Method::Put, "v0/enroll/token")
            .body(CloudRequestWrapper::new(b, cmd.cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }
}

/// Helpers to create spaces API requests
pub(crate) mod space {
    use crate::space::*;
    use ockam_api::cloud::{space::*, BareCloudRequestWrapper};
    use ockam_core::api::RequestBuilder;

    use super::*;

    pub(crate) fn create(cmd: &CreateCommand) -> RequestBuilder<CloudRequestWrapper<CreateSpace>> {
        let b = CreateSpace::new(cmd.name.as_str(), &cmd.admins);
        Request::builder(Method::Post, "v0/spaces")
            .body(CloudRequestWrapper::new(b, cmd.cloud_opts.route()))
    }

    pub(crate) fn list(cmd: &ListCommand) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::builder(Method::Get, "v0/spaces")
            .body(CloudRequestWrapper::bare(cmd.cloud_opts.route()))
    }

    pub(crate) fn show(cmd: &ShowCommand) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::builder(Method::Get, format!("v0/spaces/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cmd.cloud_opts.route()))
    }

    pub(crate) fn delete(cmd: &DeleteCommand) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::builder(Method::Delete, format!("v0/spaces/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cmd.cloud_opts.route()))
    }
}

/// Helpers to create projects API requests
pub(crate) mod project {
    use crate::project::*;
    use ockam_api::cloud::project::*;

    use super::*;

    pub(crate) fn create(cmd: CreateCommand) -> anyhow::Result<Vec<u8>> {
        let b = CreateProject::new(cmd.project_name.as_str(), &[], &cmd.services);
        let mut buf = vec![];
        Request::builder(Method::Post, format!("v0/projects/{}", cmd.space_id))
            .body(CloudRequestWrapper::new(b, cmd.cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn list(cmd: ListCommand) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Get, "v0/projects")
            .body(CloudRequestWrapper::bare(cmd.cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn show(cmd: ShowCommand) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Get, format!("v0/projects/{}", cmd.project_id))
            .body(CloudRequestWrapper::bare(cmd.cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn delete(cmd: DeleteCommand) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(
            Method::Delete,
            format!("v0/projects/{}/{}", cmd.space_id, cmd.project_id),
        )
        .body(CloudRequestWrapper::bare(cmd.cloud_opts.route()))
        .encode(&mut buf)?;
        Ok(buf)
    }
}

pub(crate) mod message {
    use crate::message::*;
    use ockam_api::nodes::service::message::*;

    use super::*;

    pub(crate) fn send(cmd: SendCommand) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Post, "v0/message")
            .body(SendMessage::new(&cmd.to, cmd.message.as_bytes()))
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
pub(crate) fn parse_status(resp: &[u8]) -> Result<models::base::NodeStatus> {
    let mut dec = Decoder::new(resp);
    let _ = dec.decode::<Response>()?;
    Ok(dec.decode::<models::base::NodeStatus>()?)
}

/// Parse the returned status response
pub(crate) fn parse_tcp_list(resp: &[u8]) -> Result<models::transport::TransportList> {
    let mut dec = Decoder::new(resp);
    let _ = dec.decode::<Response>()?;
    Ok(dec.decode::<models::transport::TransportList>()?)
}

/// Parse the returned status response
pub(crate) fn parse_transport_status(
    resp: &[u8],
) -> Result<(Response, models::transport::TransportStatus<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((
        response,
        dec.decode::<models::transport::TransportStatus>()?,
    ))
}

pub(crate) fn parse_create_vault_response(resp: &[u8]) -> Result<Response> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok(response)
}

pub(crate) fn parse_create_identity_response(
    resp: &[u8],
) -> Result<(Response, models::identity::CreateIdentityResponse<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((
        response,
        dec.decode::<models::identity::CreateIdentityResponse>()?,
    ))
}

pub(crate) fn parse_long_identity_response(
    resp: &[u8],
) -> Result<(Response, models::identity::LongIdentityResponse<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((
        response,
        dec.decode::<models::identity::LongIdentityResponse>()?,
    ))
}

pub(crate) fn parse_short_identity_response(
    resp: &[u8],
) -> Result<(Response, models::identity::ShortIdentityResponse<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((
        response,
        dec.decode::<models::identity::ShortIdentityResponse>()?,
    ))
}

pub(crate) fn parse_create_secure_channel_listener_response(resp: &[u8]) -> Result<Response> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok(response)
}

pub(crate) fn parse_list_secure_channel_listener_response(
    resp: &[u8],
) -> Result<models::secure_channel::SecureChannelListenerAddrList> {
    let mut dec = Decoder::new(resp);
    let _ = dec.decode::<Response>()?;
    Ok(dec.decode::<models::secure_channel::SecureChannelListenerAddrList>()?)
}

////////////// !== share CLI args

#[derive(Clone, Debug, Args)]
pub struct CloudOpts {
    /// Ockam cloud node's address
    #[clap(global = true, default_value = DEFAULT_CLOUD_ADDRESS)]
    pub addr: MultiAddr,
}

impl CloudOpts {
    pub fn route(&self) -> &MultiAddr {
        &self.addr
    }
}
