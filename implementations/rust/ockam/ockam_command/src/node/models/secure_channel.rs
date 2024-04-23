use serde::Serialize;

use crate::Error;
use ockam_api::{
    nodes::models::secure_channel::ShowSecureChannelListenerResponse, try_address_to_multiaddr,
};
use ockam_multiaddr::MultiAddr;

/// Information to display of the secure channel listeners in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowSecureChannelListener {
    pub address: MultiAddr,
}

impl TryFrom<ShowSecureChannelListenerResponse> for ShowSecureChannelListener {
    type Error = Error;

    fn try_from(value: ShowSecureChannelListenerResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            address: try_address_to_multiaddr(&value.addr)?,
        })
    }
}
