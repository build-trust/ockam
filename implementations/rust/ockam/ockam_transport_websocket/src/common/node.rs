use ockam_core::lib::net::SocketAddr;
use ockam_core::{async_trait, Address};

#[async_trait::async_trait]
pub trait TransportNode {
    const ADDR_ID: u8;

    fn peer(&self) -> SocketAddr;
    fn tx_addr(&self) -> Address;
    fn rx_addr(&self) -> Address;
    fn build_addr(peer: SocketAddr) -> Address {
        format!("{}#{}", Self::ADDR_ID, peer).into()
    }
}
