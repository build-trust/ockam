use crate::proto::{DnsAddr, Ip4, Ip6, Secure, Service, Tcp, Worker};
use crate::{MultiAddr, Protocol};
use ockam_core::flow_control::{FlowControlId, FlowControls};
use ockam_core::{Address, Route, LOCAL};
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpTransport};
use std::net::{SocketAddrV4, SocketAddrV6};
use tracing::error;

impl MultiAddr {
    pub async fn to_route(
        &self,
        ctx: &Context,
        flow_controls: &FlowControls,
    ) -> Option<MultiAddrToRouteResult> {
        let tcp = TcpTransport::create(ctx).await.unwrap();

        let mut rb = Route::new();
        let mut it = self.iter().peekable();

        let mut flow_control_id = None;
        let mut number_of_tcp_hops = 0;

        while let Some(p) = it.next() {
            match p.code() {
                Ip4::CODE => {
                    if number_of_tcp_hops >= 1 {
                        return None; // Only 1 TCP hop is allowed
                    }

                    let ip4 = p.cast::<Ip4>()?;
                    let port = it.next()?.cast::<Tcp>()?;
                    let socket_addr = SocketAddrV4::new(*ip4, *port);

                    let options = if socket_addr.ip().is_loopback() {
                        // TODO: Enable FlowControl for loopback addresses as well
                        TcpConnectionOptions::insecure()
                    } else {
                        let id = flow_controls.generate_id();
                        flow_control_id = Some(id.clone());
                        TcpConnectionOptions::as_producer(flow_controls, &id)
                    };

                    let addr = tcp.connect(socket_addr.to_string(), options).await.ok()?;
                    number_of_tcp_hops += 1;
                    rb = rb.append(addr)
                }
                Ip6::CODE => {
                    if number_of_tcp_hops >= 1 {
                        return None; // Only 1 TCP hop is allowed
                    }

                    let ip6 = p.cast::<Ip6>()?;
                    let port = it.next()?.cast::<Tcp>()?;
                    let socket_addr = SocketAddrV6::new(*ip6, *port, 0, 0);

                    let options = if socket_addr.ip().is_loopback() {
                        // TODO: Enable FlowControl for loopback addresses as well
                        TcpConnectionOptions::insecure()
                    } else {
                        let id = flow_controls.generate_id();
                        flow_control_id = Some(id.clone());
                        TcpConnectionOptions::as_producer(flow_controls, &id)
                    };

                    let addr = tcp.connect(socket_addr.to_string(), options).await.ok()?;
                    number_of_tcp_hops += 1;
                    rb = rb.append(addr)
                }
                DnsAddr::CODE => {
                    if number_of_tcp_hops >= 1 {
                        return None; // Only 1 TCP hop is allowed
                    }

                    let host = p.cast::<DnsAddr>()?;
                    if let Some(p) = it.peek() {
                        if p.code() == Tcp::CODE {
                            let port = p.cast::<Tcp>()?;

                            let id = flow_controls.generate_id();
                            flow_control_id = Some(id.clone());
                            let options = TcpConnectionOptions::as_producer(flow_controls, &id);

                            let addr = tcp
                                .connect(format!("{}:{}", &*host, *port), options)
                                .await
                                .ok()?;
                            number_of_tcp_hops += 1;
                            rb = rb.append(addr);
                            let _ = it.next();
                            continue;
                        }
                    }
                }
                Worker::CODE => {
                    let local = p.cast::<Worker>()?;
                    rb = rb.append(Address::new(LOCAL, &*local))
                }
                Service::CODE => {
                    let local = p.cast::<Service>()?;
                    rb = rb.append(Address::new(LOCAL, &*local))
                }
                Secure::CODE => {
                    let local = p.cast::<Secure>()?;
                    rb = rb.append(Address::new(LOCAL, &*local))
                }
                other => {
                    error!(target: "ockam_api", code = %other, "unsupported protocol");
                    return None;
                }
            }
        }

        Some(MultiAddrToRouteResult {
            flow_control_id,
            route: rb.into(),
        })
    }
}

pub struct MultiAddrToRouteResult {
    pub flow_control_id: Option<FlowControlId>,
    pub route: Route,
}
