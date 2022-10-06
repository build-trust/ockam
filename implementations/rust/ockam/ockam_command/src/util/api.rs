//! API shim to make it nicer to interact with the ockam messaging API

use std::path::Path;
use std::str::FromStr;

use anyhow::Context;
use clap::Args;
// TODO: maybe we can remove this cross-dependency inside the CLI?
use minicbor::Decoder;
use ockam_api::nodes::models::services::{
    StartAuthenticatedServiceRequest, StartAuthenticatorRequest, StartCredentialsService,
    StartIdentityServiceRequest, StartVaultServiceRequest, StartVerifierService,
};
use tracing::trace;

use ockam::identity::IdentityIdentifier;
use ockam::Result;
use ockam_api::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
use ockam_api::nodes::models::secure_channel::CredentialExchangeMode;
use ockam_api::nodes::*;
use ockam_core::api::RequestBuilder;
use ockam_core::api::{Request, Response};
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;

use crate::util::DEFAULT_CONTROLLER_ADDRESS;

////////////// !== generators

/// Construct a request to query node status
pub(crate) fn query_status() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::get("/node").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to query node tcp connections
pub(crate) fn list_tcp_connections() -> Result<Vec<u8>> {
    let mut buf = vec![];
    let builder = Request::get("/node/tcp/connection");
    builder.encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to query node tcp listeners
pub(crate) fn list_tcp_listeners() -> RequestBuilder<'static, ()> {
    Request::get("/node/tcp/listener")
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
    Request::post("/node/tcp/connection")
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
    Request::post("/node/tcp/listener")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to delete node tcp connection
pub(crate) fn delete_tcp_connection(
    cmd: &crate::tcp::connection::DeleteCommand,
) -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::delete("/node/tcp/connection")
        .body(models::transport::DeleteTransport::new(&cmd.id, cmd.force))
        .encode(&mut buf)?;

    Ok(buf)
}

/// Construct a request to delete node tcp connection
pub(crate) fn delete_tcp_listener(cmd: &crate::tcp::listener::DeleteCommand) -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::delete("/node/tcp/listener")
        .body(models::transport::DeleteTransport::new(&cmd.id, cmd.force))
        .encode(&mut buf)?;

    Ok(buf)
}

/// Construct a request to create a Vault
pub(crate) fn create_vault(path: Option<String>) -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::post("/node/vault")
        .body(models::vault::CreateVaultRequest::new(path))
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to export Identity
pub(crate) fn long_identity() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::post("/node/identity/actions/show/long").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to print Identity Id
pub(crate) fn short_identity() -> RequestBuilder<'static, ()> {
    Request::post("/node/identity/actions/show/short")
}

/// Construct a request to print a list of services for the given node
pub(crate) fn list_services() -> RequestBuilder<'static, ()> {
    Request::get("/node/services")
}

/// Construct a request to print a list of inlets for the given node
pub(crate) fn list_inlets() -> RequestBuilder<'static, ()> {
    Request::get("/node/inlet")
}

/// Construct a request to print a list of outlets for the given node
pub(crate) fn list_outlets() -> RequestBuilder<'static, ()> {
    Request::get("/node/outlet")
}

/// Construct a request builder to list all secure channels on the given node
pub(crate) fn list_secure_channels() -> RequestBuilder<'static, ()> {
    Request::get("/node/secure_channel")
}

/// Construct a request to create Secure Channels
pub(crate) fn create_secure_channel(
    addr: &MultiAddr,
    authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    credential_exchange_mode: CredentialExchangeMode,
) -> RequestBuilder<'static, models::secure_channel::CreateSecureChannelRequest<'static>> {
    let payload = models::secure_channel::CreateSecureChannelRequest::new(
        addr,
        authorized_identifiers,
        credential_exchange_mode,
    );
    Request::post("/node/secure_channel").body(payload)
}

pub(crate) fn delete_secure_channel(
    addr: &Address,
) -> RequestBuilder<'static, models::secure_channel::DeleteSecureChannelRequest<'static>> {
    let payload = models::secure_channel::DeleteSecureChannelRequest::new(addr);
    Request::delete("/node/secure_channel").body(payload)
}

pub(crate) fn show_secure_channel(
    addr: &Address,
) -> RequestBuilder<'static, models::secure_channel::ShowSecureChannelRequest<'static>> {
    let payload = models::secure_channel::ShowSecureChannelRequest::new(addr);
    Request::get("/node/show_secure_channel").body(payload)
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
    Request::post("/node/secure_channel_listener")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to list Secure Channel Listeners
pub(crate) fn list_secure_channel_listener() -> RequestBuilder<'static, ()> {
    Request::get("/node/secure_channel_listener")
}

/// Construct a request to start a Vault Service
pub(crate) fn start_vault_service(addr: &str) -> RequestBuilder<'static, StartVaultServiceRequest> {
    let payload = StartVaultServiceRequest::new(addr);
    Request::post("/node/services/vault").body(payload)
}

/// Construct a request to start an Identity Service
pub(crate) fn start_identity_service(
    addr: &str,
) -> RequestBuilder<'static, StartIdentityServiceRequest> {
    let payload = StartIdentityServiceRequest::new(addr);
    Request::post("/node/services/identity").body(payload)
}

/// Construct a request to start an Authenticated Service
pub(crate) fn start_authenticated_service(
    addr: &str,
) -> RequestBuilder<'static, StartAuthenticatedServiceRequest> {
    let payload = StartAuthenticatedServiceRequest::new(addr);
    Request::post("/node/services/authenticated").body(payload)
}

/// Construct a request to start a Verifier Service
pub(crate) fn start_verifier_service(addr: &str) -> RequestBuilder<'static, StartVerifierService> {
    let payload = StartVerifierService::new(addr);
    Request::post("/node/services/verifier").body(payload)
}

/// Construct a request to start a Credentials Service
pub(crate) fn start_credentials_service(
    addr: &str,
    oneway: bool,
) -> RequestBuilder<'static, StartCredentialsService> {
    let payload = StartCredentialsService::new(addr, oneway);
    Request::post("/node/services/credentials").body(payload)
}

/// Construct a request to start an Authenticator Service
pub(crate) fn start_authenticator_service<'a>(
    addr: &'a str,
    enrollers: &'a Path,
    project: &'a str,
) -> RequestBuilder<'static, StartAuthenticatorRequest<'a>> {
    let payload = StartAuthenticatorRequest::new(addr, enrollers, project.as_bytes());
    Request::post("/node/services/authenticator").body(payload)
}

pub(crate) mod credentials {
    use ockam_api::nodes::models::credentials::{GetCredentialRequest, PresentCredentialRequest};

    use super::*;

    pub(crate) fn present_credential(
        to: &MultiAddr,
        oneway: bool,
    ) -> RequestBuilder<PresentCredentialRequest> {
        let b = PresentCredentialRequest::new(to, oneway);
        Request::post("/node/credentials/actions/present").body(b)
    }

    pub(crate) fn get_credential<'r>(overwrite: bool) -> RequestBuilder<'r, GetCredentialRequest> {
        let b = GetCredentialRequest::new(overwrite);
        Request::post("/node/credentials/actions/get").body(b)
    }
}

/// Helpers to create enroll API requests
pub(crate) mod enroll {
    use ockam_api::cloud::enroll::auth0::{Auth0Token, AuthenticateAuth0Token};

    use crate::enroll::*;

    use super::*;

    pub(crate) fn auth0(
        cmd: EnrollCommand,
        token: Auth0Token,
    ) -> RequestBuilder<CloudRequestWrapper<AuthenticateAuth0Token>> {
        let token = AuthenticateAuth0Token::new(token);
        Request::post("v0/enroll/auth0")
            .body(CloudRequestWrapper::new(token, &cmd.cloud_opts.route()))
    }
}

/// Helpers to create spaces API requests
pub(crate) mod space {
    use ockam_api::cloud::space::*;

    use crate::space::*;

    use super::*;

    pub(crate) fn create(cmd: &CreateCommand) -> RequestBuilder<CloudRequestWrapper<CreateSpace>> {
        let b = CreateSpace::new(cmd.name.as_str(), &cmd.admins);
        Request::post("v0/spaces").body(CloudRequestWrapper::new(b, &cmd.cloud_opts.route()))
    }

    pub(crate) fn list(cloud_route: &MultiAddr) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get("v0/spaces").body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn show<'a>(
        id: &str,
        cloud_route: &'a MultiAddr,
    ) -> RequestBuilder<'a, BareCloudRequestWrapper<'a>> {
        Request::get(format!("v0/spaces/{}", id)).body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn delete<'a>(
        id: &str,
        cloud_route: &'a MultiAddr,
    ) -> RequestBuilder<'a, BareCloudRequestWrapper<'a>> {
        Request::delete(format!("v0/spaces/{}", id)).body(CloudRequestWrapper::bare(cloud_route))
    }
}

/// Helpers to create projects API requests
pub(crate) mod project {
    use ockam_api::cloud::project::*;

    use crate::project::*;

    use super::*;

    pub(crate) fn create<'a>(
        project_name: &'a str,
        space_id: &'a str,
        enforce_credentials: Option<bool>,
        cloud_route: &'a MultiAddr,
    ) -> RequestBuilder<'a, CloudRequestWrapper<'a, CreateProject<'a>>> {
        let b = CreateProject::new::<&str, &str>(project_name, enforce_credentials, &[], &[]);
        Request::post(format!("v0/projects/{}", space_id))
            .body(CloudRequestWrapper::new(b, cloud_route))
    }

    pub(crate) fn list(cloud_route: &MultiAddr) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get("v0/projects").body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn show<'a>(
        id: &str,
        cloud_route: &'a MultiAddr,
    ) -> RequestBuilder<'a, BareCloudRequestWrapper<'a>> {
        Request::get(format!("v0/projects/{}", id)).body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn delete<'a>(
        space_id: &'a str,
        project_id: &'a str,
        cloud_route: &'a MultiAddr,
    ) -> RequestBuilder<'a, BareCloudRequestWrapper<'a>> {
        Request::delete(format!("v0/projects/{}/{}", space_id, project_id))
            .body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn add_enroller(
        cmd: &AddEnrollerCommand,
    ) -> RequestBuilder<CloudRequestWrapper<AddEnroller>> {
        let b = AddEnroller::new(
            cmd.enroller_identity_id.as_str(),
            cmd.description.as_deref(),
        );
        Request::post(format!("v0/project-enrollers/{}", cmd.project_id))
            .body(CloudRequestWrapper::new(b, &cmd.cloud_opts.route()))
    }

    pub(crate) fn list_enrollers(
        cmd: &ListEnrollersCommand,
    ) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get(format!("v0/project-enrollers/{}", cmd.project_id))
            .body(CloudRequestWrapper::bare(&cmd.cloud_opts.route()))
    }

    pub(crate) fn delete_enroller(
        cmd: &DeleteEnrollerCommand,
    ) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::delete(format!(
            "v0/project-enrollers/{}/{}",
            cmd.project_id, cmd.enroller_identity_id
        ))
        .body(CloudRequestWrapper::bare(&cmd.cloud_opts.route()))
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

pub(crate) fn parse_list_services_response(resp: &[u8]) -> Result<models::services::ServiceList> {
    let mut dec = Decoder::new(resp);
    let _ = dec.decode::<Response>()?;
    Ok(dec.decode::<models::services::ServiceList>()?)
}

pub(crate) fn parse_list_inlets_response(resp: &[u8]) -> Result<models::portal::InletList> {
    let mut dec = Decoder::new(resp);
    let _ = dec.decode::<Response>()?;
    Ok(dec.decode::<models::portal::InletList>()?)
}

pub(crate) fn parse_list_outlets_response(resp: &[u8]) -> Result<models::portal::OutletList> {
    let mut dec = Decoder::new(resp);
    let _ = dec.decode::<Response>()?;
    Ok(dec.decode::<models::portal::OutletList>()?)
}

pub(crate) fn parse_create_secure_channel_listener_response(resp: &[u8]) -> Result<Response> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok(response)
}

////////////// !== share CLI args

pub(crate) const OCKAM_CONTROLLER_ADDR: &str = "OCKAM_CONTROLLER_ADDR";

#[derive(Clone, Debug, Args)]
pub struct CloudOpts;

impl CloudOpts {
    pub fn route(&self) -> MultiAddr {
        let route = if let Ok(s) = std::env::var(OCKAM_CONTROLLER_ADDR) {
            s
        } else {
            DEFAULT_CONTROLLER_ADDRESS.to_string()
        };
        trace!(%route, "Controller route");
        MultiAddr::from_str(&route)
            .context(format!("invalid Controller route: {route}"))
            .unwrap()
    }
}
