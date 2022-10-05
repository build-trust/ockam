use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Secure, Tcp};
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use std::time::Duration;

pub(crate) const MAX_RECOVERY_TIME: Duration = Duration::from_secs(10);
pub(crate) const MAX_CONNECT_TIME: Duration = Duration::from_secs(5);

pub(crate) fn starts_with_host_tcp_secure(addr: &MultiAddr) -> Option<usize> {
    let host_match = Match::any([DnsAddr::CODE, Ip4::CODE, Ip6::CODE]);
    if addr.matches(0, &[host_match, Tcp::CODE.into(), Secure::CODE.into()]) {
        Some(3)
    } else {
        None
    }
}
