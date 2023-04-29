use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Secure, Tcp};
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use std::time::Duration;

pub(crate) const MAX_RECOVERY_TIME: Duration = Duration::from_secs(10);
pub(crate) const MAX_CONNECT_TIME: Duration = Duration::from_secs(5);

pub(crate) fn starts_with_host_tcp(addr: &MultiAddr) -> Option<(MultiAddr, MultiAddr)> {
    let host_match = Match::any([DnsAddr::CODE, Ip4::CODE, Ip6::CODE]);
    if addr.matches(0, &[host_match, Tcp::CODE.into()]) {
        Some(addr.split(2))
    } else {
        None
    }
}

pub(crate) fn starts_with_secure(addr: &MultiAddr) -> Option<(MultiAddr, MultiAddr)> {
    if addr.matches(0, &[Secure::CODE.into()]) {
        Some(addr.split(2))
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use ockam_multiaddr::MultiAddr;
    use crate::session::util::starts_with_host_tcp;

    #[test]
    fn starts_with_host_tcp_returns_split_address() {


        let m = MultiAddr::try_from("/dnsaddr/localhost/tcp/4000/service/api").unwrap();
        let (m1, m2) = starts_with_host_tcp(&m).unwrap();

        assert!(m1.to_string() == "/dnsaddr/localhost/tcp/4000" && m2.to_string() ==
            "/service/api");

    }

    #[test]
    fn starts_with_host_tcp_returns_none_when_address_is_not_tcp() {

        use ockam_multiaddr::MultiAddr;
        let m = MultiAddr::try_from("worker/1234").unwrap();

        assert!( starts_with_host_tcp(&m).is_none());

    }
}
