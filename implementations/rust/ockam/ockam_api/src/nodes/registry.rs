use crate::nodes::service::Alias;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::{Address, Result, Route};
use ockam_identity::Identity;
use ockam_vault::Vault;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SecureChannelInfo {
    // Target route of the channel
    route: Route,
    // Local address of the created channel
    addr: Address,
}

impl SecureChannelInfo {
    pub(crate) fn new(route: Route, addr: Address) -> Self {
        Self { addr, route }
    }

    pub(crate) fn route(&self) -> &Route {
        &self.route
    }

    pub(crate) fn addr(&self) -> &Address {
        &self.addr
    }
}

pub(crate) struct OrchestratorSecureChannelInfo {
    addr: Address,
}

impl OrchestratorSecureChannelInfo {
    pub(crate) fn new(addr: Address) -> Self {
        Self { addr }
    }

    pub(crate) fn addr(&self) -> &Address {
        &self.addr
    }
}

#[derive(Default, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct IdentityRouteKey(Vec<u8>);

impl IdentityRouteKey {
    pub(crate) async fn new(identity: &Identity<Vault>, route: &Route) -> Result<Self> {
        let mut key = identity.export().await?;
        key.extend_from_slice(route.to_string().as_bytes());
        Ok(Self(key))
    }
}

#[derive(Default)]
pub(crate) struct SecureChannelListenerInfo {}

#[derive(Default)]
pub(crate) struct VaultServiceInfo {}

#[derive(Default)]
pub(crate) struct IdentityServiceInfo {}

#[derive(Default)]
pub(crate) struct AuthenticatedServiceInfo {}

#[derive(Default)]
pub(crate) struct UppercaseServiceInfo {}

#[derive(Default)]
pub(crate) struct EchoerServiceInfo {}

#[derive(Default)]
pub(crate) struct VerifierServiceInfo {}

#[derive(Default)]
pub(crate) struct AuthenticatorServiceInfo {}

pub(crate) struct InletInfo {
    pub(crate) bind_addr: String,
    pub(crate) worker_addr: Address,
}

impl InletInfo {
    pub(crate) fn new(bind_addr: &str, worker_addr: Option<&Address>) -> Self {
        let worker_addr = match worker_addr {
            Some(addr) => addr.clone(),
            None => Address::from_string(""),
        };
        Self {
            bind_addr: bind_addr.to_owned(),
            worker_addr,
        }
    }
}

pub(crate) struct OutletInfo {
    pub(crate) tcp_addr: String,
    pub(crate) worker_addr: Address,
}

impl OutletInfo {
    pub(crate) fn new(tcp_addr: &str, worker_addr: Option<&Address>) -> Self {
        let worker_addr = match worker_addr {
            Some(addr) => addr.clone(),
            None => Address::from_string(""),
        };
        Self {
            tcp_addr: tcp_addr.to_owned(),
            worker_addr,
        }
    }
}

#[derive(Default)]
pub(crate) struct Registry {
    // Registry to keep track of secure channels. It uses an Arc to store the channel info because we
    // generally add two entries to the map: one using the target route as the key to avoid creating
    // duplicated secure channels, and another using the secure channel address to be able to remove them.
    pub(crate) secure_channels: BTreeMap<IdentityRouteKey, Arc<SecureChannelInfo>>,
    // Registry to keep track of secure channels between the node and the orchestrator (controller node + project nodes).
    // TODO: refactor to use `secure_channels` where `orchestrator_secure_channels` is being used
    pub(crate) orchestrator_secure_channels:
        BTreeMap<IdentityRouteKey, OrchestratorSecureChannelInfo>,
    pub(crate) secure_channel_listeners: BTreeMap<Address, SecureChannelListenerInfo>,
    pub(crate) vault_services: BTreeMap<Address, VaultServiceInfo>,
    pub(crate) identity_services: BTreeMap<Address, IdentityServiceInfo>,
    pub(crate) authenticated_services: BTreeMap<Address, AuthenticatedServiceInfo>,
    pub(crate) uppercase_services: BTreeMap<Address, UppercaseServiceInfo>,
    pub(crate) echoer_services: BTreeMap<Address, EchoerServiceInfo>,
    pub(crate) verifier_services: BTreeMap<Address, VerifierServiceInfo>,
    #[cfg(feature = "direct-authenticator")]
    pub(crate) authenticator_service: BTreeMap<Address, AuthenticatorServiceInfo>,

    // FIXME: wow this is a terrible way to store data
    pub(crate) inlets: BTreeMap<Alias, InletInfo>,
    pub(crate) outlets: BTreeMap<Alias, OutletInfo>,
}
