use serde::{Deserialize, Serialize};

use ockam_core::{Address, Route, TransportType};

use crate::models::CredentialAndPurposeKey;
use crate::{
    Identifier, RemoteCredentialRefresherTimingOptions, RemoteCredentialRetrieverTimingOptions,
};

/// Options for retrieving credentials
#[derive(Debug, Clone)]
pub enum CredentialRetrieverOptions {
    /// No credential retrieval is required
    None,
    /// Credentials must be retrieved from cache, for a given issuer
    CacheOnly(Identifier),
    /// Credentials are retrieved via a remote authority
    Remote {
        /// Routing information to the issuer
        retriever_info: RemoteCredentialRetrieverInfo,
        /// Timing options for retrieving credentials
        retriever_timing_options: RemoteCredentialRetrieverTimingOptions,
        /// Timing options for refreshing credentials
        refresher_timing_options: RemoteCredentialRefresherTimingOptions,
    },
    /// Credentials have been provided in-memory
    InMemory(CredentialAndPurposeKey),
}

impl CredentialRetrieverOptions {
    /// Create remote retriever options with default timing options
    pub fn remote_default(
        retriever_info: RemoteCredentialRetrieverInfo,
    ) -> CredentialRetrieverOptions {
        CredentialRetrieverOptions::Remote {
            retriever_info,
            retriever_timing_options: Default::default(),
            refresher_timing_options: Default::default(),
        }
    }
}

/// Information necessary to connect to a remote credential retriever
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCredentialRetrieverInfo {
    /// Issuer identity, used to validate retrieved credentials
    pub issuer: Identifier,
    /// Route used to establish a secure channel to the remote node
    pub route: Route,
    /// Address of the credentials service on the remote node
    pub service_address: Address,
    /// Transport used by the SecureClient
    pub transport_type: TransportType,
}

impl RemoteCredentialRetrieverInfo {
    /// Create new information for a credential retriever
    pub fn new(
        issuer: Identifier,
        route: Route,
        service_address: Address,
        transport_type: TransportType,
    ) -> Self {
        Self {
            issuer,
            route,
            service_address,
            transport_type,
        }
    }
}
