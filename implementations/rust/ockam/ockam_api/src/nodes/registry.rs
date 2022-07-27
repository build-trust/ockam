use crate::nodes::service::Alias;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::Address;

#[derive(Default)]
pub(crate) struct SecureChannelInfo {}

#[derive(Default)]
pub(crate) struct SecureChannelListenerInfo {}

#[derive(Default)]
pub(crate) struct VaultServiceInfo {}

#[derive(Default)]
pub(crate) struct IdentityServiceInfo {}

#[derive(Default)]
pub(crate) struct AuthenticatedServiceInfo {}

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
    pub(crate) secure_channel_listeners: BTreeMap<Address, SecureChannelListenerInfo>,
    pub(crate) vault_services: BTreeMap<Address, VaultServiceInfo>,
    pub(crate) identity_services: BTreeMap<Address, IdentityServiceInfo>,
    pub(crate) authenticated_services: BTreeMap<Address, AuthenticatedServiceInfo>,

    // FIXME: wow this is a terrible way to store data
    pub(crate) inlets: BTreeMap<Alias, InletInfo>,
    pub(crate) outlets: BTreeMap<Alias, OutletInfo>,
}
