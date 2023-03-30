//! API shim to make it nicer to interact with the ockam messaging API

use regex::Regex;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{anyhow, Context};
use clap::Args;
// TODO: maybe we can remove this cross-dependency inside the CLI?
use minicbor::Decoder;
use ockam_api::nodes::models::services::{
    StartAuthenticatedServiceRequest, StartAuthenticatorRequest, StartIdentityServiceRequest,
    StartVaultServiceRequest, StartVerifierService,
};
use tracing::trace;

use ockam::identity::IdentityIdentifier;
use ockam_api::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
use ockam_api::nodes::*;
use ockam_api::DefaultAddress;
use ockam_core::api::RequestBuilder;
use ockam_core::api::{Request, Response};
use ockam_core::{Address, CowStr};
use ockam_multiaddr::MultiAddr;

use crate::project::ProjectInfo;
use crate::util::DEFAULT_CONTROLLER_ADDRESS;
use crate::OckamConfig;
use crate::Result;
use ockam::TcpTransport;
use ockam_api::config::lookup::ProjectLookup;
use ockam_api::credential_retrievers::FromCredentialIssuer;
use ockam_identity::credential::Credential;
use ockam_identity::trust_context::{
    AuthorityInfo, CredentialRetriever, FromMemoryCredentialRetriever, TrustContext,
};
use ockam_identity::PublicIdentity;
use ockam_vault::Vault;
use serde::{Deserialize, Serialize};

////////////// !== generators

/// Construct a request to query node status
pub(crate) fn query_status() -> RequestBuilder<'static, ()> {
    Request::get("/node")
}

/// Construct a request to query node tcp listeners
pub(crate) fn list_tcp_listeners() -> RequestBuilder<'static, ()> {
    Request::get("/node/tcp/listener")
}

/// Construct a request to create node tcp connection
pub(crate) fn create_tcp_connection(
    cmd: &crate::tcp::connection::CreateCommand,
) -> RequestBuilder<'static, models::transport::CreateTransport<'static>> {
    let (tt, addr) = (
        models::transport::TransportMode::Connect,
        cmd.address.clone(),
    );

    let payload =
        models::transport::CreateTransport::new(models::transport::TransportType::Tcp, tt, addr);
    Request::post("/node/tcp/connection").body(payload)
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

/// Construct a request builder to list all workers on the given node
pub(crate) fn list_workers() -> RequestBuilder<'static, ()> {
    Request::get("/node/workers")
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
pub(crate) fn list_secure_channel_listener() -> RequestBuilder<'static, ()> {
    Request::get("/node/secure_channel_listener")
}

pub(crate) fn delete_secure_channel_listener(
    addr: &Address,
) -> RequestBuilder<'static, models::secure_channel::DeleteSecureChannelListenerRequest<'static>> {
    let payload = models::secure_channel::DeleteSecureChannelListenerRequest::new(addr);
    Request::delete("/node/secure_channel_listener").body(payload)
}

/// Construct a request to show Secure Channel Listener
pub(crate) fn show_secure_channel_listener(
    addr: &Address,
) -> RequestBuilder<'static, models::secure_channel::ShowSecureChannelListenerRequest<'static>> {
    let payload = models::secure_channel::ShowSecureChannelListenerRequest::new(addr);
    Request::get("/node/show_secure_channel_listener").body(payload)
}

/// Construct a request to start a Vault Service
pub(crate) fn start_vault_service(addr: &str) -> RequestBuilder<'static, StartVaultServiceRequest> {
    let payload = StartVaultServiceRequest::new(addr);
    Request::post(node_service(DefaultAddress::VAULT_SERVICE)).body(payload)
}

/// Construct a request to start an Identity Service
pub(crate) fn start_identity_service(
    addr: &str,
) -> RequestBuilder<'static, StartIdentityServiceRequest> {
    let payload = StartIdentityServiceRequest::new(addr);
    Request::post(node_service(DefaultAddress::IDENTITY_SERVICE)).body(payload)
}

/// Construct a request to start an Authenticated Service
pub(crate) fn start_authenticated_service(
    addr: &str,
) -> RequestBuilder<'static, StartAuthenticatedServiceRequest> {
    let payload = StartAuthenticatedServiceRequest::new(addr);
    Request::post(node_service(DefaultAddress::AUTHENTICATED_SERVICE)).body(payload)
}

/// Construct a request to start a Verifier Service
pub(crate) fn start_verifier_service(addr: &str) -> RequestBuilder<'static, StartVerifierService> {
    let payload = StartVerifierService::new(addr);
    Request::post(node_service(DefaultAddress::VERIFIER)).body(payload)
}

/// Construct a request to start an Authenticator Service
pub(crate) fn start_authenticator_service<'a>(
    addr: &'a str,
    project: &'a str,
) -> RequestBuilder<'static, StartAuthenticatorRequest<'a>> {
    let payload = StartAuthenticatorRequest::new(addr, project.as_bytes());
    Request::post(node_service(DefaultAddress::DIRECT_AUTHENTICATOR)).body(payload)
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

    pub(crate) fn get_credential<'r>(
        overwrite: bool,
        identity_name: Option<String>,
    ) -> RequestBuilder<'r, GetCredentialRequest> {
        let b = GetCredentialRequest::new(overwrite, identity_name);
        Request::post("/node/credentials/actions/get").body(b)
    }
}

/// Return the path of a service given its name
fn node_service(service_name: &str) -> String {
    format!("/node/services/{service_name}")
}

/// Helpers to create enroll API requests
pub(crate) mod enroll {
    use ockam_api::cloud::enroll::auth0::{Auth0Token, AuthenticateAuth0Token};

    use crate::enroll::*;

    use super::*;

    pub(crate) fn auth0<'a>(
        cmd: EnrollCommand,
        token: Auth0Token,
    ) -> RequestBuilder<'a, CloudRequestWrapper<'a, AuthenticateAuth0Token>> {
        let token = AuthenticateAuth0Token::new(token);
        Request::post("v0/enroll/auth0").body(CloudRequestWrapper::new(
            token,
            &cmd.cloud_opts.route(),
            None::<CowStr>,
        ))
    }
}

/// Helpers to create spaces API requests
pub(crate) mod space {
    use ockam_api::cloud::space::*;

    use crate::space::*;

    use super::*;

    pub(crate) fn create(cmd: &CreateCommand) -> RequestBuilder<CloudRequestWrapper<CreateSpace>> {
        let b = CreateSpace::new(&cmd.name, &cmd.admins);
        Request::post("v0/spaces").body(CloudRequestWrapper::new(
            b,
            &cmd.cloud_opts.route(),
            None::<CowStr>,
        ))
    }

    pub(crate) fn list(cloud_route: &MultiAddr) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get("v0/spaces").body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn show<'a>(
        id: &str,
        cloud_route: &'a MultiAddr,
    ) -> RequestBuilder<'a, BareCloudRequestWrapper<'a>> {
        Request::get(format!("v0/spaces/{id}")).body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn delete<'a>(
        id: &str,
        cloud_route: &'a MultiAddr,
    ) -> RequestBuilder<'a, BareCloudRequestWrapper<'a>> {
        Request::delete(format!("v0/spaces/{id}")).body(CloudRequestWrapper::bare(cloud_route))
    }
}

/// Helpers to create projects API requests
pub(crate) mod project {
    use ockam_api::cloud::project::*;

    use super::*;

    pub(crate) fn create<'a>(
        project_name: &'a str,
        space_id: &'a str,
        cloud_route: &'a MultiAddr,
    ) -> RequestBuilder<'a, CloudRequestWrapper<'a, CreateProject<'a>>> {
        let b = CreateProject::new::<&str, &str>(project_name, &[], &[]);
        Request::post(format!("v0/projects/{space_id}")).body(CloudRequestWrapper::new(
            b,
            cloud_route,
            None::<CowStr>,
        ))
    }

    pub(crate) fn list(cloud_route: &MultiAddr) -> RequestBuilder<BareCloudRequestWrapper> {
        Request::get("v0/projects").body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn show<'a>(
        id: &str,
        cloud_route: &'a MultiAddr,
    ) -> RequestBuilder<'a, BareCloudRequestWrapper<'a>> {
        Request::get(format!("v0/projects/{id}")).body(CloudRequestWrapper::bare(cloud_route))
    }

    pub(crate) fn delete<'a>(
        space_id: &'a str,
        project_id: &'a str,
        cloud_route: &'a MultiAddr,
    ) -> RequestBuilder<'a, BareCloudRequestWrapper<'a>> {
        Request::delete(format!("v0/projects/{space_id}/{project_id}"))
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

pub(crate) const OCKAM_CONTROLLER_ADDR: &str = "OCKAM_CONTROLLER_ADDR";

#[derive(Clone, Debug, Args)]
pub struct CloudOpts {
    #[arg(global = true, value_name = "IDENTITY", long)]
    pub identity: Option<String>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum CredentialRetrieverConfig {
    File(PathBuf),
    Online(MultiAddr),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TrustContextAuthorityConfig {
    pub identity: String,
    pub credential_retriever: Option<CredentialRetrieverConfig>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TrustContextConfig {
    pub id: String,
    pub authority: Option<TrustContextAuthorityConfig>,

    // This is for backwards compatibility
    project_addr: Option<(MultiAddr, IdentityIdentifier)>,
    // This is ugly.. we coud store the OktaConfig at least.. until we have a better way
    // to carry that info.  But that will mean dealing with lifecycles here and elsewhere.
    // I'd rather not.
    okta_config: Option<String>,
}

pub fn parse_trust_context(trust_context: &str) -> Result<TrustContextConfig> {
    let tc: TrustContextConfig = serde_json::from_str(trust_context)?;
    Ok(tc)
}

// TODO: these three are mutually exclusive
#[derive(Clone, Debug, Args, Default)]
pub struct ProjectOpts {
    /// Project config file
    #[arg(
        global = true,
        long = "project-path",
        value_name = "Project json (deprecated)"
    )]
    pub project_path: Option<PathBuf>,

    #[arg(global = true, long = "trust-context", value_parser = parse_trust_context)]
    pub trust_context: Option<TrustContextConfig>,

    #[arg(global = true, long = "project")]
    pub project: Option<String>,
}

impl<'a> From<ProjectInfo<'a>> for TrustContextConfig {
    fn from(p: ProjectInfo) -> Self {
        TrustContextConfig {
            id: p.id.to_string(),
            authority: Some(TrustContextAuthorityConfig {
                identity: p.authority_identity.unwrap().to_string(),
                credential_retriever: Some(CredentialRetrieverConfig::Online(
                    MultiAddr::from_str(&p.authority_access_route.unwrap()).unwrap(),
                )),
            }),
            project_addr: Some((
                MultiAddr::from_str(&p.access_route).unwrap(),
                p.identity.unwrap(),
            )),
            okta_config: p
                .okta_config
                .as_ref()
                .map(|c| serde_json::to_string(c).unwrap()),
        }
    }
}

impl From<&ProjectLookup> for TrustContextConfig {
    fn from(p: &ProjectLookup) -> Self {
        let authority = p.authority.as_ref().unwrap();
        TrustContextConfig {
            id: p.id.to_string(),
            authority: Some(TrustContextAuthorityConfig {
                identity: hex::encode(authority.identity()),
                credential_retriever: Some(CredentialRetrieverConfig::Online(
                    authority.address().clone(),
                )),
            }),
            project_addr: Some((
                p.node_route.clone().unwrap(),
                p.identity_id.clone().unwrap(),
            )),
            okta_config: p.okta.as_ref().map(|c| serde_json::to_string(c).unwrap()),
        }
    }
}

impl TrustContextConfig {
    pub async fn build(&self, tcp_transport: TcpTransport) -> TrustContext {
        let auth = match self.authority.as_ref() {
            None => None,
            Some(a) => Some(a.build(tcp_transport).await),
        };
        TrustContext::new(self.id.clone(), auth)
    }

    pub fn project_addr(&self) -> Option<(MultiAddr, IdentityIdentifier)> {
        self.project_addr.clone()
    }
    pub fn okta_config(&self) -> Option<String> {
        self.okta_config.clone()
    }
}

impl TrustContextAuthorityConfig {
    pub async fn identity(&self) -> PublicIdentity {
        let data = hex::decode(&self.identity).unwrap();
        PublicIdentity::import(&data, Vault::create())
            .await
            .unwrap()
    }

    pub async fn build(&self, tcp_transport: TcpTransport) -> AuthorityInfo {
        let identity = self.identity().await;
        let identifier = identity.identifier().clone();
        AuthorityInfo::new(
            identity,
            self.credential_retriever
                .as_ref()
                .map(|cr| cr.build(&identifier, tcp_transport)),
        )
    }
}
impl CredentialRetrieverConfig {
    pub fn build(
        &self,
        identity: &IdentityIdentifier,
        tcp_transport: TcpTransport,
    ) -> Box<dyn CredentialRetriever> {
        match self {
            CredentialRetrieverConfig::Online(addr) => Box::new(FromCredentialIssuer::new(
                identity.clone(),
                addr.clone(),
                tcp_transport,
            )),
            CredentialRetrieverConfig::File(path) => {
                let encoded_cred = std::fs::read_to_string(path).unwrap();
                let bytes = hex::decode(encoded_cred).unwrap();
                let cred: Credential = minicbor::decode(&bytes).unwrap();
                Box::new(FromMemoryCredentialRetriever::new(cred))
            }
        }
    }
}

impl ProjectOpts {
    pub fn trust_context(&self, default_proj: Option<PathBuf>) -> Option<TrustContextConfig> {
        self.trust_context
            .clone()
            .or_else(|| {
                self.project_path.as_ref().map(|path| {
                    let s = std::fs::read_to_string(path).unwrap();
                    let proj_info: ProjectInfo = serde_json::from_str(&s).unwrap();
                    TrustContextConfig::from(proj_info)
                })
            })
            .or_else(|| {
                let config = OckamConfig::load().expect("Failed to load config");
                let lookup = config.lookup();
                self.project
                    .as_ref()
                    .map(|name| {
                        let project_lookup = lookup.get_project(name).unwrap();
                        TrustContextConfig::from(project_lookup)
                    })
                    .or_else(|| {
                        default_proj.map(|path| {
                            let s = std::fs::read_to_string(path).unwrap();
                            let proj_info: ProjectInfo = serde_json::from_str(&s).unwrap();
                            TrustContextConfig::from(proj_info)
                        })
                    })
            })
    }
}

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

////////////// !== validators

pub(crate) fn validate_cloud_resource_name(s: &str) -> Result<()> {
    let project_name_regex = Regex::new(r"^[a-zA-Z0-9]+([a-zA-Z0-9-_\.]?[a-zA-Z0-9])*$").unwrap();
    let is_project_name_valid = project_name_regex.is_match(s);
    if !is_project_name_valid {
        Err(anyhow!("Invalid name").into())
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
            "name-with-traling-underscore_",
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
