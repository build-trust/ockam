//! API shim to make it nicer to interact with the ockam messaging API

use crate::util::DEFAULT_ORCHESTRATOR_ADDRESS;
// TODO: maybe we can remove this cross-dependency inside the CLI?
use minicbor::Decoder;

use clap::Args;
use ockam::identity::IdentityIdentifier;
use ockam::Result;
use ockam_api::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
use ockam_api::nodes::*;
use ockam_core::api::RequestBuilder;
use ockam_core::api::{Request, Response};
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;

////////////// !== generators

pub(crate) mod node {
    use super::*;

    /// Construct a request to query node status
    pub(crate) fn query_status() -> RequestBuilder<'static, ()> {
        Request::get("/node")
    }
}
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
pub(crate) fn list_tcp_listeners() -> Result<Vec<u8>> {
    let mut buf = vec![];
    let builder = Request::get("/node/tcp/listener");
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

/// Construct a request to create Identity
pub(crate) fn create_identity() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::post("/node/identity").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to export Identity
pub(crate) fn long_identity() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::post("/node/identity/actions/show/long").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to print Identity Id
pub(crate) fn short_identity() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::post("/node/identity/actions/show/short").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request builder to list all secure channels on the given node
pub(crate) fn list_secure_channels() -> RequestBuilder<'static, ()> {
    Request::get("/node/secure_channel")
}

/// Construct a request to create Secure Channels
pub(crate) fn create_secure_channel(
    addr: &MultiAddr,
    authorized_identifiers: Option<Vec<IdentityIdentifier>>,
) -> RequestBuilder<'static, models::secure_channel::CreateSecureChannelRequest<'static>> {
    let payload = models::secure_channel::CreateSecureChannelRequest::new(
        addr.clone(),
        authorized_identifiers,
    );
    Request::post("/node/secure_channel").body(payload)
}

pub(crate) fn delete_secure_channel(
    addr: &Address,
) -> RequestBuilder<'static, models::secure_channel::DeleteSecureChannelRequest<'static>> {
    let payload = models::secure_channel::DeleteSecureChannelRequest::new(addr);
    Request::delete("/node/secure_channel").body(payload)
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
pub(crate) fn start_vault_service(addr: &str) -> Result<Vec<u8>> {
    let payload = models::services::StartVaultServiceRequest::new(addr);

    let mut buf = vec![];
    Request::post("/node/services/vault")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to start an Identity Service
pub(crate) fn start_identity_service(addr: &str) -> Result<Vec<u8>> {
    let payload = models::services::StartIdentityServiceRequest::new(addr);

    let mut buf = vec![];
    Request::post("/node/services/identity")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to start an Authenticated Service
pub(crate) fn start_authenticated_service(addr: &str) -> Result<Vec<u8>> {
    let payload = models::services::StartAuthenticatedServiceRequest::new(addr);

    let mut buf = vec![];
    Request::post("/node/services/authenticated")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

pub(crate) mod credentials {
    use super::*;
    use hex::FromHexError;
    use ockam_api::nodes::models::credentials::{
        GetCredentialRequest, PresentCredentialRequest, SetAuthorityRequest,
    };

    pub(crate) fn present_credential(
        to: &MultiAddr,
        oneway: bool,
    ) -> RequestBuilder<PresentCredentialRequest> {
        let b = PresentCredentialRequest::new(to.clone(), oneway);
        Request::post("/node/credentials/actions/present").body(b)
    }

    pub(crate) fn set_authority(authorities: &[String]) -> RequestBuilder<SetAuthorityRequest> {
        let authorities: std::result::Result<Vec<Vec<u8>>, FromHexError> =
            authorities.iter().map(hex::decode).collect();
        let authorities = authorities.unwrap();

        let b = SetAuthorityRequest::new(authorities);
        Request::post("/node/credentials/authority").body(b)
    }

    pub(crate) fn get_credential<'r>(
        from: MultiAddr,
        overwrite: bool,
    ) -> RequestBuilder<'r, GetCredentialRequest> {
        let b = GetCredentialRequest::new(from, overwrite);
        Request::post("/node/credentials/actions/get").body(b)
    }
}

/// Helpers to create enroll API requests
pub(crate) mod enroll {
    use crate::enroll::*;
    use ockam_api::cloud::enroll::auth0::{Auth0Token, AuthenticateAuth0Token};

    use super::*;

    pub(crate) fn auth0(
        cmd: EnrollCommand,
        token: Auth0Token,
    ) -> RequestBuilder<CloudRequestWrapper<AuthenticateAuth0Token>> {
        let token = AuthenticateAuth0Token::new(token);
        Request::post("v0/enroll/auth0").body(CloudRequestWrapper::new(
            token,
            cmd.cloud_opts.route().clone(),
        ))
    }
}

/// Helpers to create spaces API requests
pub(crate) mod space {
    use crate::space::*;
    use ockam_api::cloud::space::*;

    use super::*;

    pub(crate) fn create(cmd: &CreateCommand) -> RequestBuilder<CloudRequestWrapper<CreateSpace>> {
        let b = CreateSpace::new(cmd.name.as_str(), &cmd.admins);
        Request::post("v0/spaces").body(CloudRequestWrapper::new(b, cmd.cloud_opts.route().clone()))
    }

    pub(crate) fn list(cmd: &ListCommand) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get("v0/spaces").body(CloudRequestWrapper::bare(cmd.cloud_opts.route().clone()))
    }

    pub(crate) fn show(cmd: &ShowCommand) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get(format!("v0/spaces/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cmd.cloud_opts.route().clone()))
    }

    pub(crate) fn delete(cmd: &DeleteCommand) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::delete(format!("v0/spaces/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cmd.cloud_opts.route().clone()))
    }
}

/// Helpers to create projects API requests
pub(crate) mod project {
    use crate::project::*;
    use ockam_api::cloud::project::*;

    use super::*;

    pub(crate) fn create(
        cmd: &CreateCommand,
    ) -> RequestBuilder<CloudRequestWrapper<CreateProject>> {
        let b = CreateProject::new(cmd.project_name.as_str(), &[], &cmd.services);
        Request::post(format!("v0/projects/{}", cmd.space_id))
            .body(CloudRequestWrapper::new(b, cmd.cloud_opts.route().clone()))
    }

    pub(crate) fn list(cmd: &ListCommand) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get("v0/projects").body(CloudRequestWrapper::bare(cmd.cloud_opts.route().clone()))
    }

    pub(crate) fn show(cmd: &ShowCommand) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get(format!("v0/projects/{}", cmd.project_id))
            .body(CloudRequestWrapper::bare(cmd.cloud_opts.route().clone()))
    }

    pub(crate) fn delete(cmd: DeleteCommand) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::delete(format!("v0/projects/{}/{}", cmd.space_id, cmd.project_id))
            .body(CloudRequestWrapper::bare(cmd.cloud_opts.route().clone()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn add_enroller(
        cmd: &AddEnrollerCommand,
    ) -> RequestBuilder<CloudRequestWrapper<AddEnroller>> {
        let b = AddEnroller::new(
            cmd.enroller_identity_id.as_str(),
            cmd.description.as_deref(),
        );
        Request::post(format!("v0/project-enrollers/{}", cmd.project_id))
            .body(CloudRequestWrapper::new(b, cmd.cloud_opts.route().clone()))
    }

    pub(crate) fn list_enrollers(
        cmd: &ListEnrollersCommand,
    ) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get(format!("v0/project-enrollers/{}", cmd.project_id))
            .body(CloudRequestWrapper::bare(cmd.cloud_opts.route().clone()))
    }

    pub(crate) fn delete_enroller(
        cmd: &DeleteEnrollerCommand,
    ) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::delete(format!(
            "v0/project-enrollers/{}/{}",
            cmd.project_id, cmd.enroller_identity_id
        ))
        .body(CloudRequestWrapper::bare(cmd.cloud_opts.route().clone()))
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

////////////// !== share CLI args

#[derive(Clone, Debug, Args)]
pub struct CloudOpts {
    /// Ockam orchestrator address
    #[clap(global = true, hide = true, default_value = DEFAULT_ORCHESTRATOR_ADDRESS)]
    pub addr: MultiAddr,
}

impl CloudOpts {
    pub fn route(&self) -> &MultiAddr {
        &self.addr
    }
}
