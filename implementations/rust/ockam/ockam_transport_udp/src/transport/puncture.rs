use crate::{UdpBind, UdpPuncture, UdpPunctureOptions, UdpTransport};
use ockam_core::{Address, Result};

impl UdpTransport {
    /// Start a new puncture
    pub fn puncture(
        &self,
        bind: UdpBind,
        peer_udp_address: String,
        my_remote_address: Address,
        their_remote_address: Address,
        options: UdpPunctureOptions,
        redirect_first_message_to_transport: bool,
    ) -> Result<UdpPuncture> {
        UdpPuncture::create(
            &self.ctx,
            bind,
            peer_udp_address,
            my_remote_address,
            their_remote_address,
            options,
            redirect_first_message_to_transport,
        )
    }

    /// Stop a puncture
    pub fn stop_puncture(&self, puncture: UdpPuncture) -> Result<()> {
        puncture.stop(&self.ctx)
    }
}
