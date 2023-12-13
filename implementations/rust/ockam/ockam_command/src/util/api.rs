//! API shim to make it nicer to interact with the ockam messaging API
use clap::Args;
use miette::miette;
// TODO: maybe we can remove this cross-dependency inside the CLI?
use minicbor::Decoder;
use regex::Regex;

use ockam::identity::Identifier;
use ockam_api::nodes::models::flow_controls::AddConsumer;
use ockam_api::nodes::models::services::{
    StartAuthenticatedServiceRequest, StartAuthenticatorRequest, StartCredentialsService,
    StartHopServiceRequest, StartOktaIdentityProviderRequest,
};
use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_api::nodes::*;
use ockam_core::api::Request;
use ockam_core::api::ResponseHeader;
use ockam_core::flow_control::FlowControlId;
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;

use crate::service::config::OktaIdentityProviderConfig;
use crate::Result;

////////////// !== generators

/// Construct a request to query node status
pub(crate) fn query_status() -> Request<()> {
    Request::get("/node")
}

/// Construct a request to query node tcp listeners
pub(crate) fn list_tcp_listeners() -> Request<()> {
    Request::get("/node/tcp/listener")
}

/// Construct a request to create node tcp connection
pub(crate) fn create_tcp_connection(
    cmd: &crate::tcp::connection::CreateCommand,
) -> Request<models::transport::CreateTcpConnection> {
    let payload = models::transport::CreateTcpConnection::new(cmd.address.clone());

    Request::post("/node/tcp/connection").body(payload)
}

/// Construct a request to print a list of services for the given node
pub(crate) fn list_services() -> Request<()> {
    Request::get("/node/services")
}

/// Construct a request to print a list of inlets for the given node
pub(crate) fn list_inlets() -> Request<()> {
    Request::get("/node/inlet")
}

/// Construct a request to print a list of outlets for the given node
pub(crate) fn list_outlets() -> Request<()> {
    Request::get("/node/outlet")
}

/// Construct a request builder to list all secure channels on the given node
pub(crate) fn list_secure_channels() -> Request<()> {
    Request::get("/node/secure_channel")
}

/// Construct a request builder to list all workers on the given node
pub(crate) fn list_workers() -> Request<()> {
    Request::get("/node/workers")
}

pub(crate) fn delete_secure_channel(
    addr: &Address,
) -> Request<models::secure_channel::DeleteSecureChannelRequest> {
    let payload = models::secure_channel::DeleteSecureChannelRequest::new(addr);
    Request::delete("/node/secure_channel").body(payload)
}

pub(crate) fn show_secure_channel(
    addr: &Address,
) -> Request<models::secure_channel::ShowSecureChannelRequest> {
    let payload = models::secure_channel::ShowSecureChannelRequest::new(addr);
    Request::get("/node/show_secure_channel").body(payload)
}

/// Construct a request to create Secure Channel Listeners
pub(crate) fn create_secure_channel_listener(
    addr: &Address,
    authorized_identifiers: Option<Vec<Identifier>>,
    identity_name: Option<String>,
) -> Result<Vec<u8>> {
    let payload = models::secure_channel::CreateSecureChannelListenerRequest::new(
        addr,
        authorized_identifiers,
        identity_name,
    );

    let mut buf = vec![];
    Request::post("/node/secure_channel_listener")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to list Secure Channel Listeners
pub(crate) fn list_secure_channel_listener() -> Request<()> {
    Request::get("/node/secure_channel_listener")
}

pub(crate) fn delete_secure_channel_listener(
    addr: &Address,
) -> Request<models::secure_channel::DeleteSecureChannelListenerRequest> {
    let payload = models::secure_channel::DeleteSecureChannelListenerRequest::new(addr);
    Request::delete("/node/secure_channel_listener").body(payload)
}

/// Construct a request to show Secure Channel Listener
pub(crate) fn show_secure_channel_listener(
    addr: &Address,
) -> Request<models::secure_channel::ShowSecureChannelListenerRequest> {
    let payload = models::secure_channel::ShowSecureChannelListenerRequest::new(addr);
    Request::get("/node/show_secure_channel_listener").body(payload)
}

/// Construct a request to start a Hop Service
pub(crate) fn start_hop_service(addr: &str) -> Request<StartHopServiceRequest> {
    let payload = StartHopServiceRequest::new(addr);
    Request::post(node_service(DefaultAddress::HOP_SERVICE)).body(payload)
}

/// Construct a request to start an Authenticated Service
pub(crate) fn start_authenticated_service(addr: &str) -> Request<StartAuthenticatedServiceRequest> {
    let payload = StartAuthenticatedServiceRequest::new(addr);
    Request::post(node_service(DefaultAddress::AUTHENTICATED_SERVICE)).body(payload)
}

/// Construct a request to start a Credential Service
pub(crate) fn start_credentials_service(
    public_identity: &str,
    addr: &str,
    oneway: bool,
) -> Request<StartCredentialsService> {
    let payload = StartCredentialsService::new(public_identity, addr, oneway);
    Request::post(node_service(DefaultAddress::CREDENTIALS_SERVICE)).body(payload)
}

/// Construct a request to start an Authenticator Service
pub(crate) fn start_authenticator_service(
    addr: &str,
    project: &str,
) -> Request<StartAuthenticatorRequest> {
    let payload = StartAuthenticatorRequest::new(addr, project.as_bytes());
    Request::post(node_service(DefaultAddress::DIRECT_AUTHENTICATOR)).body(payload)
}

pub(crate) fn add_consumer(id: FlowControlId, address: MultiAddr) -> Request<AddConsumer> {
    let payload = AddConsumer::new(id, address);
    Request::post("/node/flow_controls/add_consumer").body(payload)
}

pub(crate) fn start_okta_service(
    cfg: &OktaIdentityProviderConfig,
) -> Request<StartOktaIdentityProviderRequest> {
    let payload = StartOktaIdentityProviderRequest::new(
        &cfg.address,
        &cfg.tenant_base_url,
        &cfg.certificate,
        cfg.attributes.clone(),
        cfg.project.as_bytes(),
    );
    Request::post(format!(
        "/node/services/{}",
        DefaultAddress::OKTA_IDENTITY_PROVIDER
    ))
    .body(payload)
}

/// Return the path of a service given its name
fn node_service(service_name: &str) -> String {
    format!("/node/services/{service_name}")
}

////////////// !== parsers

pub(crate) fn parse_create_secure_channel_listener_response(resp: &[u8]) -> Result<ResponseHeader> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<ResponseHeader>()?;
    Ok(response)
}

////////////// !== share CLI args

#[derive(Clone, Debug, Args)]
pub struct CloudOpts {
    /// Run the command as the given identity name
    #[arg(global = true, value_name = "IDENTITY_NAME", long)]
    pub identity: Option<String>,
}

#[derive(Clone, Debug, Args, Default)]
pub struct TrustContextOpts {
    /// Trust Context config file
    #[arg(
        global = true,
        long,
        value_name = "TRUST_CONTEXT_NAME | TRUST_CONTEXT_JSON_PATH"
    )]
    pub trust_context: Option<String>,

    #[arg(global = true, long = "project", value_name = "PROJECT_NAME")]
    pub project_name: Option<String>,
}

impl TrustContextOpts {
    pub fn project_name(&self) -> Option<String> {
        self.project_name.clone()
    }
}

////////////// !== validators

pub(crate) fn validate_cloud_resource_name(s: &str) -> miette::Result<()> {
    let project_name_regex = Regex::new(r"^[a-zA-Z0-9]+([a-zA-Z0-9-_\.]?[a-zA-Z0-9])*$").unwrap();
    let is_project_name_valid = project_name_regex.is_match(s);
    if !is_project_name_valid {
        Err(miette!("Invalid name"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::util::api::validate_cloud_resource_name;

    #[test]
    fn test_validate_cloud_resource_name() {
        let valid_names: Vec<&str> = vec![
            "name",
            "0001",
            "321_11-11-22",
            "0_0",
            "6.9",
            "0-9",
            "name_with_underscores",
            "name-with-dashes",
            "name.with.dots",
            "name1with2numbers3",
            "11name22with33numbers00",
            "76123long.name_with-underscores.and-dashes_and3dots00and.numbers",
        ];
        for name in valid_names {
            assert!(validate_cloud_resource_name(name).is_ok());
        }

        let invalid_names: Vec<&str> = vec![
            "name with spaces in between",
            " name-with-leading-space",
            "name.with.trailing.space ",
            " name-with-leading-and-trailing-space ",
            "     name_with_multiple_leading_space",
            "name__with_consecutive_underscore",
            "_name_with_leading_underscore",
            "name-with-trailing-underscore_",
            "name_with_consecutive---dashes",
            "name_with_trailing_dashes--",
            "---name_with_leading_dashes",
            "name-with-consecutive...dots",
            "name.with.trailing.dots....",
            ".name_with-leading.dot",
            "name_.with._consecutive-_-dots.-.dashes-._underscores",
            "1 2 3 4",
            "  1234",
            "_",
            "__",
            ". _ .",
        ];
        for name in invalid_names {
            assert!(validate_cloud_resource_name(name).is_err());
        }
    }
}
