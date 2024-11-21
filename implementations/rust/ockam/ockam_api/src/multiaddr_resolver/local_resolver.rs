use crate::multiaddr_resolver::invalid_multiaddr_error;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Address, Error, Result, Route, LOCAL};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Node, Secure, Service, Worker};
use ockam_multiaddr::{MultiAddr, Protocol};

pub struct LocalMultiaddrResolver {}

impl LocalMultiaddrResolver {
    /// Try to convert a local multi-address to an Ockam route.
    pub fn resolve(ma: &MultiAddr) -> Result<Route> {
        let mut rb = Route::new();
        for p in ma.iter() {
            match p.code() {
                // Only hops that are directly translated to existing workers are allowed here
                Worker::CODE => {
                    let local = p
                        .cast::<Worker>()
                        .ok_or_else(|| invalid_multiaddr_error(ma))?;
                    rb = rb.append(Address::new_with_string(LOCAL, &*local))
                }
                Service::CODE => {
                    let local = p
                        .cast::<Service>()
                        .ok_or_else(|| invalid_multiaddr_error(ma))?;
                    rb = rb.append(Address::new_with_string(LOCAL, &*local))
                }
                Secure::CODE => {
                    let local = p
                        .cast::<Secure>()
                        .ok_or_else(|| invalid_multiaddr_error(ma))?;
                    rb = rb.append(Address::new_with_string(LOCAL, &*local))
                }

                Node::CODE => {
                    return Err(Error::new(
                        Origin::Api,
                        Kind::Invalid,
                        "unexpected code: node. clean_multiaddr should have been called",
                    ));
                }

                code @ (Ip4::CODE | Ip6::CODE | DnsAddr::CODE) => {
                    return Err(Error::new(
                        Origin::Api,
                        Kind::Invalid,
                        format!(
                            "unexpected code: {code}. The address must be a local address {ma}"
                        ),
                    ));
                }

                _ => {
                    return Err(invalid_multiaddr_error(ma));
                }
            }
        }

        Ok(rb.into())
    }
}
