mod local_resolver;
mod remote_resolver;
mod reverse_local_converter;
mod transport_route_resolver;

pub use local_resolver::*;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_multiaddr::MultiAddr;
pub use remote_resolver::*;
pub use reverse_local_converter::*;
pub use transport_route_resolver::*;

fn invalid_multiaddr_error(ma: &MultiAddr) -> Error {
    Error::new(
        Origin::Api,
        Kind::Misuse,
        format!("Invalid multiaddr {}", ma),
    )
}

fn multiple_transport_hops_error(ma: &MultiAddr) -> Error {
    Error::new(
        Origin::Api,
        Kind::Unsupported,
        format!("Only one hop is allowed in a multiaddr. Multiaddr={}", ma),
    )
}
