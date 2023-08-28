//! API shim to make it nicer to interact with the ockam messaging API

use std::path::PathBuf;

use clap::Args;
use miette::miette;
// TODO: maybe we can remove this cross-dependency inside the CLI?
use minicbor::Decoder;
use regex::Regex;

use ockam::identity::IdentityIdentifier;
use ockam_api::address::controller_route;
use ockam_api::cli_state::CliState;
use ockam_api::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
use ockam_api::nodes::models::flow_controls::AddConsumer;
use ockam_api::nodes::models::services::{
    StartAuthenticatedServiceRequest, StartAuthenticatorRequest, StartCredentialsService,
    StartHopServiceRequest, StartIdentityServiceRequest, StartOktaIdentityProviderRequest,
    StartVerifierService,
};
use ockam_api::nodes::*;
use ockam_api::trust_context::TrustContextConfigBuilder;
use ockam_api::DefaultAddress;
use ockam_core::api::RequestBuilder;
use ockam_core::api::{Request, Response};
use ockam_core::flow_control::FlowControlId;
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;

use crate::service::config::OktaIdentityProviderConfig;
use crate::Result;

////////////// !== generators

/// Construct a request to query node status
pub(crate) fn query_status() -> RequestBuilder<()> {
    Request::get("/node")
}

/// Construct a request to query node tcp listeners
pub(crate) fn list_tcp_listeners() -> RequestBuilder<()> {
    Request::get("/node/tcp/listener")
}

/// Construct a request to create node tcp connection
pub(crate) fn create_tcp_connection(
    cmd: &crate::tcp::connection::CreateCommand,
) -> RequestBuilder<models::transport::CreateTcpConnection> {
    let payload = models::transport::CreateTcpConnection::new(cmd.address.clone());

    Request::post("/node/tcp/connection").body(payload)
}

/// Construct a request to print a list of services for the given node
pub(crate) fn list_services() -> RequestBuilder<()> {
    Request::get("/node/services")
}

/// Construct a request to print a list of inlets for the given node
pub(crate) fn list_inlets() -> RequestBuilder<()> {
    Request::get("/node/inlet")
}

/// Construct a request to print a list of outlets for the given node
pub(crate) fn list_outlets() -> RequestBuilder<()> {
    Request::get("/node/outlet")
}

/// Construct a request builder to list all secure channels on the given node
pub(crate) fn list_secure_channels() -> RequestBuilder<()> {
    Request::get("/node/secure_channel")
}

/// Construct a request builder to list all workers on the given node
pub(crate) fn list_workers() -> RequestBuilder<()> {
    Request::get("/node/workers")
}

pub(crate) fn delete_secure_channel(
    addr: &Address,
) -> RequestBuilder<models::secure_channel::DeleteSecureChannelRequest> {
    let payload = models::secure_channel::DeleteSecureChannelRequest::new(addr);
    Request::delete("/node/secure_channel").body(payload)
}

pub(crate) fn show_secure_channel(
    addr: &Address,
) -> RequestBuilder<models::secure_channel::ShowSecureChannelRequest> {
    let payload = models::secure_channel::ShowSecureChannelRequest::new(addr);
    Request::get("/node/show_secure_channel").body(payload)
}

/// Construct a request to create Secure Channel Listeners
pub(crate) fn create_secure_channel_listener(
    addr: &Address,
    authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    identity: Option<String>,
) -> Result<Vec<u8>> {
    let payload = models::secure_channel::CreateSecureChannelListenerRequest::new(
        addr,
        authorized_identifiers,
        None,
        identity,
    );

    let mut buf = vec![];
    Request::post("/node/secure_channel_listener")
        .body(payload)
        .encode(&mut buf)?;
    Ok(buf)
}

/// Construct a request to list Secure Channel Listeners
pub(crate) fn list_secure_channel_listener() -> RequestBuilder<()> {
    Request::get("/node/secure_channel_listener")
}

pub(crate) fn delete_secure_channel_listener(
    addr: &Address,
) -> RequestBuilder<models::secure_channel::DeleteSecureChannelListenerRequest> {
    let payload = models::secure_channel::DeleteSecureChannelListenerRequest::new(addr);
    Request::delete("/node/secure_channel_listener").body(payload)
}

/// Construct a request to show Secure Channel Listener
pub(crate) fn show_secure_channel_listener(
    addr: &Address,
) -> RequestBuilder<models::secure_channel::ShowSecureChannelListenerRequest> {
    let payload = models::secure_channel::ShowSecureChannelListenerRequest::new(addr);
    Request::get("/node/show_secure_channel_listener").body(payload)
}

/// Construct a request to start a Hop Service
pub(crate) fn start_hop_service(addr: &str) -> RequestBuilder<StartHopServiceRequest> {
    let payload = StartHopServiceRequest::new(addr);
    Request::post(node_service(DefaultAddress::HOP_SERVICE)).body(payload)
}

/// Construct a request to start an Identity Service
pub(crate) fn start_identity_service(addr: &str) -> RequestBuilder<StartIdentityServiceRequest> {
    let payload = StartIdentityServiceRequest::new(addr);
    Request::post(node_service(DefaultAddress::IDENTITY_SERVICE)).body(payload)
}

/// Construct a request to start an Authenticated Service
pub(crate) fn start_authenticated_service(
    addr: &str,
) -> RequestBuilder<StartAuthenticatedServiceRequest> {
    let payload = StartAuthenticatedServiceRequest::new(addr);
    Request::post(node_service(DefaultAddress::AUTHENTICATED_SERVICE)).body(payload)
}

/// Construct a request to start a Verifier Service
pub(crate) fn start_verifier_service(addr: &str) -> RequestBuilder<StartVerifierService> {
    let payload = StartVerifierService::new(addr);
    Request::post(node_service(DefaultAddress::VERIFIER)).body(payload)
}

/// Construct a request to start a Credential Service
pub(crate) fn start_credentials_service(
    public_identity: &str,
    addr: &str,
    oneway: bool,
) -> RequestBuilder<StartCredentialsService> {
    let payload = StartCredentialsService::new(public_identity, addr, oneway);
    Request::post(node_service(DefaultAddress::CREDENTIALS_SERVICE)).body(payload)
}

/// Construct a request to start an Authenticator Service
pub(crate) fn start_authenticator_service(
    addr: &str,
    project: &str,
) -> RequestBuilder<StartAuthenticatorRequest> {
    let payload = StartAuthenticatorRequest::new(addr, project.as_bytes());
    Request::post(node_service(DefaultAddress::DIRECT_AUTHENTICATOR)).body(payload)
}

pub(crate) fn add_consumer(id: FlowControlId, address: MultiAddr) -> RequestBuilder<AddConsumer> {
    let payload = AddConsumer::new(id, address);
    Request::post("/node/flow_controls/add_consumer").body(payload)
}

pub(crate) fn start_okta_service(
    cfg: &OktaIdentityProviderConfig,
) -> RequestBuilder<StartOktaIdentityProviderRequest> {
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

    pub(crate) fn get_credential(
        overwrite: bool,
        identity_name: Option<String>,
    ) -> RequestBuilder<GetCredentialRequest> {
        let b = GetCredentialRequest::new(overwrite, identity_name);
        Request::post("/node/credentials/actions/get").body(b)
    }
}

/// Return the path of a service given its name
fn node_service(service_name: &str) -> String {
    format!("/node/services/{service_name}")
}

/// Helpers to create enroll API requests
pub mod enroll {
    use ockam_api::cloud::enroll::auth0::{AuthenticateOidcToken, OidcToken};

    use super::*;

    pub fn auth0(
        route: &MultiAddr,
        token: OidcToken,
    ) -> RequestBuilder<CloudRequestWrapper<AuthenticateOidcToken>> {
        let token = AuthenticateOidcToken::new(token);
        Request::post("v0/enroll/auth0").body(CloudRequestWrapper::new(token, route, None))
    }
}

/// Helpers to create spaces API requests
pub(crate) mod space {
    use ockam_api::cloud::space::*;

    use crate::space::*;

    use super::*;

    pub(crate) fn create(cmd: CreateCommand) -> RequestBuilder<CloudRequestWrapper<CreateSpace>> {
        let b = CreateSpace::new(cmd.name, cmd.admins);
        Request::post("v0/spaces").body(CloudRequestWrapper::new(b, &CloudOpts::route(), None))
    }

    pub(crate) fn list(cloud_route: &MultiAddr) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get("v0/spaces").body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn show(
        id: &str,
        cloud_route: &MultiAddr,
    ) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get(format!("v0/spaces/{id}")).body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn delete(
        id: &str,
        cloud_route: &MultiAddr,
    ) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::delete(format!("v0/spaces/{id}")).body(CloudRequestWrapper::bare(cloud_route))
    }
}

/// Helpers to create projects API requests
pub(crate) mod project {
    use ockam_api::cloud::project::*;

    use super::*;

    pub(crate) fn create(
        project_name: &str,
        space_id: &str,
        cloud_route: &MultiAddr,
    ) -> RequestBuilder<CloudRequestWrapper<CreateProject>> {
        let b = CreateProject::new(project_name.to_string(), vec![]);
        Request::post(format!("v1/spaces/{space_id}/projects")).body(CloudRequestWrapper::new(
            b,
            cloud_route,
            None,
        ))
    }

    pub(crate) fn list(cloud_route: &MultiAddr) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get("v0/projects").body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn show(
        id: &str,
        cloud_route: &MultiAddr,
    ) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get(format!("v0/projects/{id}")).body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn version(cloud_route: &MultiAddr) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get("v0/projects/version_info").body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn delete(
        space_id: &str,
        project_id: &str,
        cloud_route: &MultiAddr,
    ) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::delete(format!("v0/projects/{space_id}/{project_id}"))
            .body(CloudRequestWrapper::bare(cloud_route))
    }
}

/// Helpers to create operations API requests
pub(crate) mod operation {
    use super::*;

    pub(crate) fn show(
        id: &str,
        cloud_route: &MultiAddr,
    ) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get(format!("v1/operations/{id}")).body(CloudRequestWrapper::bare(cloud_route))
    }
}

/// Helpers to create share API requests
#[cfg(feature = "orchestrator")]
pub(crate) mod share {
    use ockam_api::cloud::share::{
        AcceptInvitation, CreateInvitation, CreateServiceInvitation, InvitationListKind,
        ListInvitations,
    };

    use super::*;

    pub(crate) fn accept(
        req: AcceptInvitation,
        cloud_route: &MultiAddr,
    ) -> RequestBuilder<CloudRequestWrapper<AcceptInvitation>> {
        Request::post("v0/accept_invitation".to_string()).body(CloudRequestWrapper::new(
            req,
            cloud_route,
            None,
        ))
    }

    pub(crate) fn create(
        req: CreateInvitation,
        cloud_route: &MultiAddr,
    ) -> RequestBuilder<CloudRequestWrapper<CreateInvitation>> {
        Request::post("v0/invitations".to_string()).body(CloudRequestWrapper::new(
            req,
            cloud_route,
            None,
        ))
    }

    pub(crate) fn create_service_invitation(
        req: CreateServiceInvitation,
        cloud_route: &MultiAddr,
    ) -> RequestBuilder<CloudRequestWrapper<CreateServiceInvitation>> {
        Request::post("v0/invitations/service".to_string()).body(CloudRequestWrapper::new(
            req,
            cloud_route,
            None,
        ))
    }

    pub(crate) fn list(
        kind: InvitationListKind,
        cloud_route: &MultiAddr,
    ) -> RequestBuilder<CloudRequestWrapper<ListInvitations>> {
        let req = ListInvitations { kind };
        Request::get("v0/invitations".to_string()).body(CloudRequestWrapper::new(
            req,
            cloud_route,
            None,
        ))
    }

    pub(crate) fn show(
        invitation_id: String,
        cloud_route: &MultiAddr,
    ) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get(format!("v0/invitations/{invitation_id}"))
            .body(CloudRequestWrapper::bare(cloud_route))
    }
}

////////////// !== parsers

pub(crate) fn parse_create_secure_channel_listener_response(resp: &[u8]) -> Result<Response> {
    let mut dec = Decoder::new(resp);
    let response = dec.decode::<Response>()?;
    Ok(response)
}

////////////// !== share CLI args

#[derive(Clone, Debug, Args)]
pub struct CloudOpts {
    #[arg(global = true, value_name = "IDENTITY_NAME", long)]
    pub identity: Option<String>,
}

#[derive(Clone, Debug, Args, Default)]
pub struct TrustContextOpts {
    /// Project config file (DEPRECATED)
    #[arg(global = true, long = "project-path", value_name = "PROJECT_JSON_PATH")]
    pub project_path: Option<PathBuf>,

    /// Trust Context config file
    #[arg(
        global = true,
        long,
        value_name = "TRUST_CONTEXT_NAME | TRUST_CONTEXT_JSON_PATH"
    )]
    pub trust_context: Option<String>,

    #[arg(global = true, long = "project", value_name = "PROJECT_NAME")]
    pub project: Option<String>,
}

impl TrustContextOpts {
    pub fn to_config(&self, cli_state: &CliState) -> Result<TrustContextConfigBuilder> {
        let trust_context = match &self.trust_context {
            Some(tc) => Some(cli_state.trust_contexts.read_config_from_path(tc)?),
            None => None,
        };
        Ok(TrustContextConfigBuilder {
            cli_state: cli_state.clone(),
            project_path: self.project_path.clone(),
            trust_context,
            project: self.project.clone(),
            authority_identity: None,
            credential_name: None,
            use_default_trust_context: true,
        })
    }
}

impl CloudOpts {
    pub fn route() -> MultiAddr {
        controller_route()
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
