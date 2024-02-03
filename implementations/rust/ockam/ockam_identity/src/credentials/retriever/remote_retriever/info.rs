use serde::{Deserialize, Serialize};

use ockam_core::{Address, Route};

use crate::Identifier;

/// Information necessary to connect to a remote credential retriever
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCredentialRetrieverInfo {
    /// Issuer identity, used to validate retrieved credentials
    pub issuer: Identifier,
    /// Route used to establish a secure channel to the remote node
    pub route: Route,
    /// Address of the credentials service on the remote node
    pub service_address: Address,
}

impl RemoteCredentialRetrieverInfo {
    /// Create new information for a credential retriever
    pub fn new(issuer: Identifier, route: Route, service_address: Address) -> Self {
        Self {
            issuer,
            route,
            service_address,
        }
    }
}
