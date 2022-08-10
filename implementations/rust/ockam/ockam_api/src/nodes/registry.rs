use crate::nodes::service::Alias;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::{Address, Result, Route};
use ockam_identity::Identity;
use ockam_vault::Vault;

#[derive(Default)]
pub(crate) struct SecureChannelInfo {}

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

#[derive(Default, Hash, Ord, PartialOrd, Eq, PartialEq)]
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
pub(crate) struct AuthenticatorServiceInfo {}

pub(crate) struct InletInfo {
    pub(crate) bind_addr: String,
    pub(crate) worker_address: Address,
}

pub(crate) struct OutletInfo {
    pub(crate) tcp_addr: String,
    pub(crate) worker_addr: Address,
}

#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) secure_channels: BTreeMap<Address, SecureChannelInfo>,
    // Registry to keep track of secure channels between the node and the orchestrator (controller node + project nodes).
    pub(crate) orchestrator_secure_channels:
        BTreeMap<IdentityRouteKey, OrchestratorSecureChannelInfo>,
    pub(crate) secure_channel_listeners: BTreeMap<Address, SecureChannelListenerInfo>,
    pub(crate) vault_services: BTreeMap<Address, VaultServiceInfo>,
    pub(crate) identity_services: BTreeMap<Address, IdentityServiceInfo>,
    pub(crate) authenticated_services: BTreeMap<Address, AuthenticatedServiceInfo>,
    pub(crate) uppercase_services: BTreeMap<Address, UppercaseServiceInfo>,
    pub(crate) echoer_services: BTreeMap<Address, EchoerServiceInfo>,
    pub(crate) signer_service: Option<Address>,
    #[cfg(feature = "direct-authenticator")]
    pub(crate) authenticator_service: BTreeMap<Address, AuthenticatorServiceInfo>,

    // FIXME: wow this is a terrible way to store data
    pub(crate) inlets: BTreeMap<Alias, InletInfo>,
    pub(crate) outlets: BTreeMap<Alias, OutletInfo>,
}
