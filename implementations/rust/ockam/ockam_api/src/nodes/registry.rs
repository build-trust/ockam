use crate::nodes::service::Alias;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::{Address, Result, Route};
use ockam_identity::{Identity, IdentityIdentifier};
use ockam_vault::Vault;
use std::sync::Arc;

#[derive(Clone)]
pub struct SecureChannelInfo {
    // Target route of the channel
    route: Route,
    // Local address of the created channel
    addr: Address,
    id: IdentityIdentifier,
    authorized_identifiers: Option<Vec<IdentityIdentifier>>,
}

impl SecureChannelInfo {
    pub fn new(
        route: Route,
        addr: Address,
        id: IdentityIdentifier,
        authorized_identifiers: Option<Vec<IdentityIdentifier>>,
    ) -> Self {
        Self {
            addr,
            route,
            id,
            authorized_identifiers,
        }
    }

    pub fn route(&self) -> &Route {
        &self.route
    }

    pub fn addr(&self) -> &Address {
        &self.addr
    }

    pub fn id(&self) -> &IdentityIdentifier {
        &self.id
    }

    pub fn authorized_identifiers(&self) -> Option<&Vec<IdentityIdentifier>> {
        self.authorized_identifiers.as_ref()
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
pub(crate) struct CredentialsServiceInfo {}

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
    pub(crate) secure_channel_listeners: BTreeMap<Address, SecureChannelListenerInfo>,
    pub(crate) vault_services: BTreeMap<Address, VaultServiceInfo>,
    pub(crate) identity_services: BTreeMap<Address, IdentityServiceInfo>,
    pub(crate) authenticated_services: BTreeMap<Address, AuthenticatedServiceInfo>,
    pub(crate) uppercase_services: BTreeMap<Address, UppercaseServiceInfo>,
    pub(crate) echoer_services: BTreeMap<Address, EchoerServiceInfo>,
    pub(crate) verifier_services: BTreeMap<Address, VerifierServiceInfo>,
    pub(crate) credentials_services: BTreeMap<Address, CredentialsServiceInfo>,
    #[cfg(feature = "direct-authenticator")]
    pub(crate) authenticator_service: BTreeMap<Address, AuthenticatorServiceInfo>,

    // FIXME: wow this is a terrible way to store data
    pub(crate) inlets: BTreeMap<Alias, InletInfo>,
    pub(crate) outlets: BTreeMap<Alias, OutletInfo>,
}
