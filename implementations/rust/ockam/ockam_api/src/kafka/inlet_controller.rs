use core::str::FromStr;
use minicbor::Decoder;
use ockam_core::compat::net::IpAddr;

use ockam::compat::tokio::sync::Mutex;
use ockam_core::api::{Request, ResponseHeader, Status};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Error};
use ockam_core::{Result, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::kafka::kafka_outlet_address;
use crate::nodes::models::portal::{CreateInlet, InletStatus};
use crate::nodes::NODEMANAGER_ADDR;
use crate::port_range::PortRange;

type BrokerId = i32;

/// Shared structure for every kafka worker (consumer or producer services)
/// to keep track of which brokers are being proxied with the relative inlet listener socket address.
/// Also takes care of creating inlets dynamically when they are not present yet.
#[derive(Debug, Clone)]
pub(crate) struct KafkaInletController {
    inner: Arc<Mutex<KafkaInletMapInner>>,
}

#[derive(Debug)]
struct KafkaInletMapInner {
    broker_map: HashMap<BrokerId, SocketAddr>,
    port_range: PortRange,
    current_port: u16,
    bind_ip: IpAddr,
    outlet_node_multiaddr: MultiAddr,
    local_interceptor_route: Route,
    remote_interceptor_route: Route,
}

impl KafkaInletController {
    pub(crate) fn new(
        outlet_node_multiaddr: MultiAddr,
        local_interceptor_route: Route,
        remote_interceptor_route: Route,
        bind_ip: IpAddr,
        port_range: PortRange,
    ) -> KafkaInletController {
        Self {
            inner: Arc::new(Mutex::new(KafkaInletMapInner {
                outlet_node_multiaddr,
                broker_map: HashMap::new(),
                current_port: port_range.start(),
                port_range,
                bind_ip,
                local_interceptor_route,
                remote_interceptor_route,
            })),
        }
    }

    #[cfg(test)]
    pub(crate) async fn retrieve_inlet(&self, broker_id: BrokerId) -> Option<SocketAddr> {
        let inner = self.inner.lock().await;
        inner.broker_map.get(&broker_id).copied()
    }

    /// Asserts the presence of an inlet for a broker
    /// on first time it'll create the inlet and return the relative address
    /// on the second one it'll just return the address
    pub(crate) async fn assert_inlet_for_broker(
        &self,
        context: &Context,
        broker_id: BrokerId,
    ) -> Result<SocketAddr> {
        let mut inner = self.inner.lock().await;
        if let Some(address) = inner.broker_map.get(&broker_id) {
            Ok(*address)
        } else {
            if inner.current_port > inner.port_range.end() {
                //we don't have any port left for the broker!
                return Err(Error::new(
                    Origin::Transport,
                    Kind::ResourceExhausted,
                    "reached the upper port range",
                ));
            }

            let socket_address = SocketAddr::new(inner.bind_ip, inner.current_port);
            Self::request_inlet_creation(
                context,
                socket_address,
                inner.outlet_node_multiaddr.clone(),
                inner.local_interceptor_route.clone(),
                route![
                    inner.remote_interceptor_route.clone(),
                    kafka_outlet_address(broker_id)
                ],
            )
            .await?;

            inner.current_port += 1;
            inner.broker_map.insert(broker_id, socket_address);

            Ok(socket_address)
        }
    }

    async fn request_inlet_creation(
        context: &Context,
        socket_address: SocketAddr,
        to: MultiAddr,
        prefix: Route,
        suffix: Route,
    ) -> Result<SocketAddr> {
        let buffer: Vec<u8> = context
            .send_and_receive(
                route![NODEMANAGER_ADDR],
                Request::post("/node/inlet")
                    .body(CreateInlet::to_node(
                        socket_address.to_string(),
                        to,
                        prefix,
                        suffix,
                        None,
                    ))
                    .to_vec()?,
            )
            .await?;

        let mut decoder = Decoder::new(&buffer);
        let response: ResponseHeader = decoder.decode()?;

        let status = response.status().unwrap_or(Status::InternalServerError);
        if status != Status::Ok {
            return Err(Error::new(
                Origin::Transport,
                Kind::Invalid,
                format!("cannot create inlet: {}", status),
            ));
        }
        if !response.has_body() {
            Err(Error::new(
                Origin::Transport,
                Kind::Unknown,
                "invalid create inlet response",
            ))
        } else {
            let status: InletStatus = decoder.decode()?;
            Ok(SocketAddr::from_str(&status.bind_addr)
                .map_err(|err| Error::new(Origin::Transport, Kind::Invalid, err))?)
        }
    }
}
