use crate::remote::lifecycle::RelayType;
use ockam_core::Address;

#[derive(Clone, Debug)]
pub(super) struct Addresses {
    // Used to talk to the service
    pub(super) main_remote: Address,
    // Used to forward messages inside the node
    pub(super) main_internal: Address,
    // Used to receive heartbeats
    pub(super) heartbeat: Address,
    // Used to receive completion callback
    pub(super) completion_callback: Address,
}

impl Addresses {
    pub(super) fn generate(ftype: RelayType) -> Self {
        let type_str = ftype.str();
        let main_remote = Address::random_tagged(&format!("RemoteRelay.{}.main_remote", type_str));
        let main_internal =
            Address::random_tagged(&format!("RemoteRelay.{}.main_internal", type_str));
        let heartbeat = Address::random_tagged(&format!("RemoteRelay.{}.heartbeat", type_str));
        let completion_callback =
            Address::random_tagged(&format!("RemoteRelay.{}.child", type_str));

        Self {
            main_remote,
            main_internal,
            heartbeat,
            completion_callback,
        }
    }
}
