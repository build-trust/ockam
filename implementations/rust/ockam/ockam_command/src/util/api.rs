//! API shim to make it nicer to interact with the ockam messaging API

use std::path::PathBuf;

use anyhow::{anyhow, Context};
use clap::Args;
// TODO: maybe we can remove this cross-dependency inside the CLI?
use minicbor::Decoder;
use regex::Regex;
use tracing::trace;

use ockam::identity::IdentityIdentifier;
use ockam_api::cli_state::{CliState, StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::Project;
use ockam_api::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
use ockam_api::config::cli::TrustContextConfig;
use ockam_api::nodes::models::services::{
    StartAuthenticatedServiceRequest, StartAuthenticatorRequest, StartCredentialsService,
    StartIdentityServiceRequest, StartOktaIdentityProviderRequest, StartVerifierService,
};
use ockam_api::nodes::*;
use ockam_api::DefaultAddress;
use ockam_core::api::RequestBuilder;
use ockam_core::api::{Request, Response};
use ockam_core::env::{get_env_with_default, FromString};
use ockam_core::{Address, CowStr};
use ockam_multiaddr::MultiAddr;

use crate::identity::{default_identity_name, identity_name_parser};
use crate::project::ProjectInfo;
use crate::service::config::OktaIdentityProviderConfig;
use crate::util::DEFAULT_CONTROLLER_ADDRESS;
use crate::Result;

use super::OckamConfig;

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
) -> RequestBuilder<'static, models::transport::CreateTcpConnection> {
    let payload = models::transport::CreateTcpConnection::new(
        cmd.address.clone(),
        cmd.exposed_to.clone().unwrap_or(vec![]),
    );

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

/// Construct a request to start a Credential Service
pub(crate) fn start_credentials_service<'a>(
    public_identity: &'a str,
    addr: &'a str,
    oneway: bool,
) -> RequestBuilder<'static, StartCredentialsService<'a>> {
    let payload = StartCredentialsService::new(public_identity, addr, oneway);
    Request::post(node_service(DefaultAddress::CREDENTIALS_SERVICE)).body(payload)
}

/// Construct a request to start an Authenticator Service
pub(crate) fn start_authenticator_service<'a>(
    addr: &'a str,
    project: &'a str,
) -> RequestBuilder<'static, StartAuthenticatorRequest<'a>> {
    let payload = StartAuthenticatorRequest::new(addr, project.as_bytes());
    Request::post(node_service(DefaultAddress::DIRECT_AUTHENTICATOR)).body(payload)
}

pub(crate) fn start_okta_service(
    cfg: &'_ OktaIdentityProviderConfig,
) -> RequestBuilder<'static, StartOktaIdentityProviderRequest<'_>> {
    let payload = StartOktaIdentityProviderRequest::new(
        &cfg.address,
        &cfg.tenant_base_url,
        &cfg.certificate,
        cfg.attributes.iter().map(|s| s as &str).collect(),
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
    #[arg(global = true, value_name = "IDENTITY", long, default_value_t = default_identity_name(), value_parser = identity_name_parser)]
    pub identity: String,
}

#[derive(Clone, Debug, Args, Default)]
pub struct TrustContextOpts {
    /// Project config file (DEPRECATED)
    #[arg(global = true, long = "project-path", value_name = "PROJECT_JSON_PATH")]
    pub project_path: Option<PathBuf>,

    /// Trust Context config file
    #[arg(global = true, long, value_name = "TRUST_CONTEXT_NAME | TRUST_CONTEXT_JSON_PATH", value_parser = parse_trust_context)]
    pub trust_context: Option<TrustContextConfig>,

    #[arg(global = true, long = "project")]
    pub project: Option<String>,
}

pub struct TrustContextConfigBuilder {
    project_path: Option<PathBuf>,
    trust_context: Option<TrustContextConfig>,
    project: Option<String>,
    authority_identity: Option<String>,
    credential_name: Option<String>,
    use_default_trust_context: bool,
}

impl TrustContextConfigBuilder {
    pub fn new(tco: &TrustContextOpts) -> Self {
        Self {
            project_path: tco.project_path.clone(),
            trust_context: tco.trust_context.clone(),
            project: tco.project.clone(),
            authority_identity: None,
            credential_name: None,
            use_default_trust_context: true,
        }
    }

    // with_authority_identity
    pub fn with_authority_identity(&mut self, authority_identity: Option<&String>) -> &mut Self {
        self.authority_identity = authority_identity.map(|s| s.to_string());
        self
    }

    // with_credential_name
    pub fn with_credential_name(&mut self, credential_name: Option<&String>) -> &mut Self {
        self.credential_name = credential_name.map(|s| s.to_string());
        self
    }

    pub fn use_default_trust_context(&mut self, use_default_trust_context: bool) -> &mut Self {
        self.use_default_trust_context = use_default_trust_context;
        self
    }

    pub fn build(&self) -> Option<TrustContextConfig> {
        self.trust_context
            .clone()
            .or_else(|| self.get_from_project_path(self.project_path.as_ref()?))
            .or_else(|| self.get_from_project_name())
            .or_else(|| self.get_from_authority_identity())
            .or_else(|| self.get_from_credential())
            .or_else(|| self.get_from_default_trust_context())
            .or_else(|| self.get_from_default_project())
    }

    fn get_from_project_path(&self, path: &PathBuf) -> Option<TrustContextConfig> {
        let s = std::fs::read_to_string(path)
            .context("Failed to read project file")
            .ok()?;
        let proj_info = serde_json::from_str::<ProjectInfo>(&s)
            .context("Failed to parse project info")
            .ok()?;
        let proj: Project = (&proj_info).into();

        proj.try_into().ok()
    }

    fn get_from_project_name(&self) -> Option<TrustContextConfig> {
        let config = OckamConfig::load().expect("Failed to load config");
        let lookup = config.lookup();
        let name = self.project.as_ref()?;
        let project_lookup = lookup.get_project(name)?;

        project_lookup.clone().try_into().ok()
    }

    fn get_from_authority_identity(&self) -> Option<TrustContextConfig> {
        let authority_identity = self.authority_identity.clone();
        let state = CliState::try_default().ok()?;
        let credential = match &self.credential_name {
            Some(c) => Some(state.credentials.get(c).ok()?),
            None => None,
        };

        TrustContextConfig::from_authority_identity(&authority_identity?, credential).ok()
    }

    fn get_from_credential(&self) -> Option<TrustContextConfig> {
        let state = CliState::try_default().ok()?;
        let cred_name = self.credential_name.clone()?;
        let cred_state = state.credentials.get(&cred_name).ok()?;

        cred_state.try_into().ok()
    }

    fn get_from_default_trust_context(&self) -> Option<TrustContextConfig> {
        if !self.use_default_trust_context {
            return None;
        }

        let state = CliState::try_default().ok()?;
        let tc = state.trust_contexts.default().ok()?.config().clone();
        Some(tc)
    }

    fn get_from_default_project(&self) -> Option<TrustContextConfig> {
        let state = CliState::try_default().ok()?;
        let proj = state.projects.default().ok()?;

        self.get_from_project_path(proj.path())
    }
}

pub fn parse_trust_context(trust_context_input: &str) -> Result<TrustContextConfig> {
    let tcc = match std::fs::read_to_string(trust_context_input) {
        Ok(s) => {
            let mut tc = serde_json::from_str::<TrustContextConfig>(&s)
                .context("Failed to parse trust context")?;
            tc.set_path(PathBuf::from(trust_context_input));
            tc
        }
        Err(_) => {
            let state = CliState::try_default()
                .ok()
                .and_then(|state| state.trust_contexts.get(trust_context_input).ok());
            let state = state.context("Invalid Trust Context name or path")?;
            let mut tcc = state.config().clone();
            tcc.set_path(state.path().clone());
            tcc
        }
    };

    Ok(tcc)
}

impl CloudOpts {
    pub fn route(&self) -> MultiAddr {
        let default_addr = MultiAddr::from_string(DEFAULT_CONTROLLER_ADDRESS)
            .context(format!(
                "invalid Controller route: {DEFAULT_CONTROLLER_ADDRESS}"
            ))
            .unwrap();

        let route = get_env_with_default::<MultiAddr>(OCKAM_CONTROLLER_ADDR, default_addr).unwrap();
        trace!(%route, "Controller route");

        route
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
