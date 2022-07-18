use ockam_core::compat::collections::BTreeMap;
use ockam_core::Address;

#[derive(Default)]
pub(crate) struct SecureChannelInfo {}

#[derive(Default)]
pub(crate) struct SecureChannelListenerInfo {}

#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) secure_channels: BTreeMap<Address, SecureChannelInfo>,
    pub(crate) secure_channel_listeners: BTreeMap<Address, SecureChannelListenerInfo>,
}
