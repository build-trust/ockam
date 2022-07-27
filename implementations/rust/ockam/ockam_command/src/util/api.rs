//! API shim to make it nicer to interact with the ockam messaging API

use crate::util::DEFAULT_CLOUD_ADDRESS;
// TODO: maybe we can remove this cross-dependency inside the CLI?
use crate::transport;
use minicbor::Decoder;

use clap::Args;
use ockam::identity::IdentityIdentifier;
use ockam::Result;
use ockam_api::nodes::*;
use ockam_api::{cloud::CloudRequestWrapper, Method, Request, Response};
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
        transport::CreateTypeCommand::TcpConnector { addr } => {
            (models::transport::TransportMode::Connect, addr)
        }
        transport::CreateTypeCommand::TcpListener { bind } => {
            (models::transport::TransportMode::Listen, bind)
        }
    };

    let payload =
        models::transport::CreateTransport::new(models::transport::TransportType::Tcp, tt, addr);

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
        .body(models::transport::DeleteTransport::new(&cmd.id, cmd.force))
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create a forwarder
pub(crate) fn create_forwarder(cmd: &crate::forwarder::CreateCommand) -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Post, "/node/forwarder")
        .body(models::forwarder::CreateForwarder::new(
            cmd.address(),
            cmd.alias(),
        ))
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
pub(crate) fn export_identity() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Post, "/node/identity/actions/export").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to print Identity Id
pub(crate) fn print_identity() -> Result<Vec<u8>> {
    let mut buf = vec![];
    Request::builder(Method::Post, "/node/identity/actions/print").encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create Secure Channels
pub(crate) fn create_secure_channel(
    addr: MultiAddr,
    known_identifier: Option<IdentityIdentifier>,
) -> Result<Vec<u8>> {
    let payload = models::secure_channel::CreateSecureChannelRequest::new(
        &addr,
        known_identifier.map(|x| x.to_string()),
    );

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/secure_channel")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create Secure Channel Listeners
pub(crate) fn create_secure_channel_listener(
    addr: &Address,
    known_identifier: Option<IdentityIdentifier>,
) -> Result<Vec<u8>> {
    let payload = models::secure_channel::CreateSecureChannelListenerRequest::new(
        addr,
        known_identifier.map(|x| x.to_string()),
    );

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/secure_channel_listener")
        .body(payload)
        .encode(&mut buf)?;
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

/// Construct a request to create a tcp inlet
pub(crate) fn create_inlet(
    bind_addr: &str,
    outlet_route: &MultiAddr,
    alias: &Option<String>,
) -> Result<Vec<u8>> {
    let payload = models::portal::CreateInlet::new(
        bind_addr,
        outlet_route.to_string(),
        alias.as_ref().map(|x| x.as_str().into()),
    );

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/inlet")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to create a tcp outlet
pub(crate) fn create_outlet(
    tcp_addr: &str,
    worker_addr: String,
    alias: &Option<String>,
) -> Result<Vec<u8>> {
    let payload = models::portal::CreateOutlet::new(
        tcp_addr,
        worker_addr,
        alias.as_ref().map(|x| x.as_str().into()),
    );

    let mut buf = vec![];
    Request::builder(Method::Post, "/node/outlet")
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
    use ockam_api::cloud::space::*;

    use super::*;

    pub(crate) fn create(cmd: CreateCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let b = CreateSpace::new(cmd.name.as_str(), &cmd.admins);
        let mut buf = vec![];
        Request::builder(Method::Post, "v0/spaces")
            .body(CloudRequestWrapper::new(b, cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn list(_cmd: ListCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Get, "v0/spaces")
            .body(CloudRequestWrapper::bare(cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn show(cmd: ShowCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Get, format!("v0/spaces/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn delete(cmd: DeleteCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Delete, format!("v0/spaces/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cloud_opts.route()))
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
        let b = CreateProject::new(cmd.project_name.as_str(), &[], &cmd.services);
        let mut buf = vec![];
        Request::builder(Method::Post, format!("v0/projects/{}", cmd.space_id))
            .body(CloudRequestWrapper::new(b, cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn list(_cmd: ListCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Get, "v0/projects")
            .body(CloudRequestWrapper::bare(cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn show(cmd: ShowCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Get, format!("v0/projects/{}", cmd.project_id))
            .body(CloudRequestWrapper::bare(cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn delete(cmd: DeleteCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(
            Method::Delete,
            format!("v0/projects/{}/{}", cmd.space_id, cmd.project_id),
        )
        .body(CloudRequestWrapper::bare(cloud_opts.route()))
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
            .body(CloudRequestWrapper::new(b, cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn list(_cmd: ListCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Get, "v0/invitations")
            .body(CloudRequestWrapper::bare(cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn accept(cmd: AcceptCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Put, format!("v0/invitations/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cloud_opts.route()))
            .encode(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn reject(cmd: RejectCommand, cloud_opts: CloudOpts) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        Request::builder(Method::Delete, format!("v0/invitations/{}", cmd.id))
            .body(CloudRequestWrapper::bare(cloud_opts.route()))
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
pub(crate) fn parse_transport_list(resp: &[u8]) -> Result<models::transport::TransportList> {
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

pub(crate) fn parse_export_identity_response(
    resp: &[u8],
) -> Result<(Response, models::identity::ExportIdentityResponse<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((
        response,
        dec.decode::<models::identity::ExportIdentityResponse>()?,
    ))
}

pub(crate) fn parse_print_identity_response(
    resp: &[u8],
) -> Result<(Response, models::identity::PrintIdentityResponse<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((
        response,
        dec.decode::<models::identity::PrintIdentityResponse>()?,
    ))
}

pub(crate) fn parse_create_secure_channel_listener_response(resp: &[u8]) -> Result<Response> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok(response)
}

/// Parse the returned status response
pub(crate) fn parse_inlet_status(
    resp: &[u8],
) -> Result<(Response, models::portal::InletStatus<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((response, dec.decode::<models::portal::InletStatus>()?))
}

/// Parse the returned status response
pub(crate) fn parse_outlet_status(
    resp: &[u8],
) -> Result<(Response, models::portal::OutletStatus<'_>)> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok((response, dec.decode::<models::portal::OutletStatus>()?))
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
