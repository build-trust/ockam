use core::fmt::Display;
use ockam_core::api::Method;
use ockam_core::compat::string::{String, ToString};
use ockam_core::Route;

use crate::Identifier;

enum CredentialIssuerServiceAddress {
    Controller,
    AuthorityNode,
}

impl Display for CredentialIssuerServiceAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CredentialIssuerServiceAddress::Controller => write!(f, "accounts"),
            CredentialIssuerServiceAddress::AuthorityNode => write!(f, "credential_issuer"),
        }
    }
}

enum CredentialIssuerApiServiceAddress {
    ControllerAccount,
    ControllerProject { project_id: String },
    AuthorityNode,
}

impl Display for CredentialIssuerApiServiceAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CredentialIssuerApiServiceAddress::ControllerAccount => write!(f, "/v0/account"),
            CredentialIssuerApiServiceAddress::ControllerProject { project_id } => {
                write!(f, "/v0/project/{}", project_id)
            }
            CredentialIssuerApiServiceAddress::AuthorityNode => write!(f, "/"),
        }
    }
}

/// Information necessary to connect to a remote credential retriever
#[derive(Debug, Clone)]
pub struct RemoteCredentialRetrieverInfo {
    /// Issuer identity, used to validate retrieved credentials
    pub issuer: Identifier,
    /// Route used to establish a secure channel to the remote node
    pub route: Route,
    /// Address of the credentials service on the remote node, e.g. "credential_issuer" or "accounts"
    pub service_address: String,
    /// Request path, e.g. "/" or "/v0/project/$project_id"
    pub api_service_address: String,
    /// Request method, e.g. Post or Get
    pub request_method: Method,
}

impl RemoteCredentialRetrieverInfo {
    /// Create info for a project member credential that we get from Project Membership Authority
    pub fn create_for_project_member(issuer: Identifier, route: Route) -> Self {
        Self::new(
            issuer,
            route,
            CredentialIssuerServiceAddress::AuthorityNode.to_string(),
            CredentialIssuerApiServiceAddress::AuthorityNode.to_string(),
            Method::Post,
        )
    }

    /// Create info for a project admin credential that we get from the Orchestrator
    pub fn create_for_project_admin(issuer: Identifier, route: Route, project_id: String) -> Self {
        Self::new(
            issuer,
            route,
            CredentialIssuerServiceAddress::Controller.to_string(),
            CredentialIssuerApiServiceAddress::ControllerProject { project_id }.to_string(),
            Method::Get,
        )
    }

    /// Create info for a account admin credential that we get from the Orchestrator
    pub fn create_for_account_admin(issuer: Identifier, route: Route) -> Self {
        Self::new(
            issuer,
            route,
            CredentialIssuerServiceAddress::Controller.to_string(),
            CredentialIssuerApiServiceAddress::ControllerAccount.to_string(),
            Method::Get,
        )
    }

    /// Constructor
    pub fn new(
        issuer: Identifier,
        route: Route,
        service_address: String,
        api_service_address: String,
        request_method: Method,
    ) -> Self {
        Self {
            issuer,
            route,
            service_address,
            api_service_address,
            request_method,
        }
    }
}
