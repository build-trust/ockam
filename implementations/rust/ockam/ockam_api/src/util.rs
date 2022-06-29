use core::str::FromStr;
use ockam::{Address, TCP};
use ockam_core::{Route, LOCAL};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Ockam, Tcp};
use ockam_multiaddr::{MultiAddr, Protocol};
use std::net::{SocketAddrV4, SocketAddrV6};

/// Try to convert a multi-address to an Ockam route.
pub fn multiaddr_to_route(ma: &MultiAddr) -> Option<Route> {
    let mut rb = Route::new();
    let mut it = ma.iter().peekable();
    while let Some(p) = it.next() {
        match p.code() {
            Ip4::CODE => {
                let ip4 = p.cast::<Ip4>()?;
                let tcp = it.next()?.cast::<Tcp>()?;
                let add = Address::new(TCP, SocketAddrV4::new(*ip4, *tcp).to_string());
                rb = rb.append(add)
            }
            Ip6::CODE => {
                let ip6 = p.cast::<Ip6>()?;
                let tcp = it.next()?.cast::<Tcp>()?;
                let add = Address::new(TCP, SocketAddrV6::new(*ip6, *tcp, 0, 0).to_string());
                rb = rb.append(add)
            }
            DnsAddr::CODE => {
                let host = p.cast::<DnsAddr>()?;
                if let Some(p) = it.peek() {
                    if p.code() == Tcp::CODE {
                        let tcp = p.cast::<Tcp>()?;
                        rb = rb.append(Address::new(TCP, format!("{}:{}", &*host, *tcp)));
                        let _ = it.next();
                        continue;
                    }
                }
                rb = rb.append(Address::new(TCP, &*host))
            }
            Ockam::CODE => {
                let local = p.cast::<Ockam>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            other => {
                error!(target: "ockam_api", code = %other, "unsupported protocol");
                return None;
            }
        }
    }
    Some(rb.into())
}

/// Try to convert a multiaddr to an Ockam Address
pub fn multiaddr_to_addr(ma: &MultiAddr) -> Option<Address> {
    let mut it = ma.iter().peekable();

    let proto = it.next()?;
    match proto.code() {
        DnsAddr::CODE => {
            let host = proto.cast::<DnsAddr>()?;
            if let Some(p) = it.peek() {
                if p.code() == Tcp::CODE {
                    let tcp = proto.cast::<Tcp>()?;
                    return Some(Address::new(TCP, format!("{}:{}", &*host, *tcp)));
                }
            }
        }
        Ockam::CODE => {
            let local = proto.cast::<Ockam>()?;
            return Some(Address::new(LOCAL, &*local));
        }
        _ => {}
    };

    None
}

/// Try to convert an Ockam Route into a MultiAddr.
pub fn route_to_multiaddr(r: &Route) -> Option<MultiAddr> {
    let mut ma = MultiAddr::default();
    for a in r.iter() {
        match a.transport_type() {
            TCP => {
                if let Ok(sa) = SocketAddrV4::from_str(a.address()) {
                    ma.push_back(Ip4::new(*sa.ip())).ok()?;
                    ma.push_back(Tcp::new(sa.port())).ok()?
                } else if let Ok(sa) = SocketAddrV6::from_str(a.address()) {
                    ma.push_back(Ip6::new(*sa.ip())).ok()?;
                    ma.push_back(Tcp::new(sa.port())).ok()?
                } else if let Some((host, port)) = a.address().split_once(':') {
                    ma.push_back(DnsAddr::new(host)).ok()?;
                    ma.push_back(Tcp::new(u16::from_str(port).ok()?)).ok()?
                } else {
                    ma.push_back(DnsAddr::new(a.address())).ok()?
                }
            }
            LOCAL => ma.push_back(Ockam::new(a.address())).ok()?,
            other => {
                error!(target: "ockam_api", transport = %other, "unsupported transport type");
                return None;
            }
        }
    }
    Some(ma)
}

#[cfg(test)]
pub mod tests {
    use ockam::authenticated_storage::InMemoryStorage;
    use ockam::identity::{Identity, TrustEveryonePolicy};
    use ockam_core::Worker;
    use ockam_node::Context;
    use ockam_vault::Vault;

    pub async fn start_api_listener<W: Worker<Context = Context>>(
        ctx: &mut Context,
        vault: &Vault,
        address: &str,
        worker: W,
    ) -> ockam_core::Result<()> {
        // Create an Identity to represent Receiver.
        let receiver = Identity::create(ctx, vault).await?;

        // Create an AuthenticatedStorage to store info about Receiver's known Identities.
        let storage = InMemoryStorage::new();

        // Create a secure channel listener for Receiver that will wait for requests to
        // initiate an Authenticated Key Exchange.
        receiver
            .create_secure_channel_listener("api", TrustEveryonePolicy, &storage)
            .await?;

        //TODO: ensure that worker can't be accessed if not throught the secure channel
        //      would help catch up errors on badly setup test cases.
        ctx.start_worker(address, worker).await?;
        ockam_core::Result::Ok(())
    }
}
