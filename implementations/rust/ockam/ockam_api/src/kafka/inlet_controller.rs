use core::str::FromStr;
use minicbor::Decoder;

use crate::kafka::kafka_outlet_address;
use crate::nodes::models::portal::{CreateInlet, InletStatus};
use crate::nodes::NODEMANAGER_ADDR;
use crate::port_range::PortRange;
use ockam::compat::tokio::sync::Mutex;
use ockam_abac::PolicyExpression;
use ockam_core::api::{Request, ResponseHeader, Status};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::rand::random_string;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Error};
use ockam_core::{Result, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_core::HostnamePort;

type BrokerId = i32;

/// Shared structure for every kafka worker (consumer or producer services)
/// to keep track of which brokers are being proxied with the relative inlet listener socket address.
/// Also takes care of creating inlets dynamically when they are not present yet.
#[derive(Debug, Clone)]
pub(crate) struct KafkaInletController {
    inner: Arc<Mutex<KafkaInletMapInner>>,
    policy_expression: Option<PolicyExpression>,
}

#[derive(Debug)]
struct KafkaInletMapInner {
    broker_map: HashMap<BrokerId, HostnamePort>,
    port_range: PortRange,
    current_port: u16,
    bind_hostname: String,
    outlet_node_multiaddr: MultiAddr,
    local_interceptor_route: Route,
    remote_interceptor_route: Route,
}

impl KafkaInletController {
    pub(crate) fn new(
        outlet_node_multiaddr: MultiAddr,
        local_interceptor_route: Route,
        remote_interceptor_route: Route,
        bind_hostname: String,
        port_range: PortRange,
        policy_expression: Option<PolicyExpression>,
    ) -> KafkaInletController {
        Self {
            inner: Arc::new(Mutex::new(KafkaInletMapInner {
                outlet_node_multiaddr,
                broker_map: HashMap::new(),
                current_port: port_range.start(),
                port_range,
                bind_hostname,
                local_interceptor_route,
                remote_interceptor_route,
            })),
            policy_expression,
        }
    }

    #[cfg(test)]
    pub(crate) async fn retrieve_inlet(&self, broker_id: BrokerId) -> Option<HostnamePort> {
        let inner = self.inner.lock().await;
        inner.broker_map.get(&broker_id).cloned()
    }

    /// Asserts the presence of an inlet for a broker.
    /// The first time it'll create the inlet and return the relative address.
    /// After that, it'll just return the address
    pub(crate) async fn assert_inlet_for_broker(
        &self,
        context: &Context,
        broker_id: BrokerId,
    ) -> Result<HostnamePort> {
        let mut inner = self.inner.lock().await;
        if let Some(address) = inner.broker_map.get(&broker_id) {
            Ok(address.clone())
        } else {
            if inner.current_port > inner.port_range.end() {
                // we don't have any port left for the broker!
                return Err(Error::new(
                    Origin::Transport,
                    Kind::ResourceExhausted,
                    "reached the upper port range",
                ));
            }

            let inlet_bind_address =
                HostnamePort::new(inner.bind_hostname.clone(), inner.current_port);
            Self::request_inlet_creation(
                context,
                inlet_bind_address.clone(),
                inner.outlet_node_multiaddr.clone(),
                inner.local_interceptor_route.clone(),
                route![
                    inner.remote_interceptor_route.clone(),
                    kafka_outlet_address(broker_id)
                ],
                self.policy_expression.clone(),
            )
            .await?;

            inner.current_port += 1;
            inner
                .broker_map
                .insert(broker_id, inlet_bind_address.clone());

            Ok(inlet_bind_address)
        }
    }

    async fn request_inlet_creation(
        context: &Context,
        inlet_bind_address: HostnamePort,
        to: MultiAddr,
        prefix: Route,
        suffix: Route,
        policy_expression: Option<PolicyExpression>,
    ) -> Result<SocketAddr> {
        let mut payload = CreateInlet::to_node(
            inlet_bind_address,
            to,
            format!("kafka-inlet-{}", random_string()),
            prefix,
            suffix,
            None,
            true,
            false,
            false,
        );
        if let Some(expr) = policy_expression {
            payload.set_policy_expression(expr);
        }
        let buffer: Vec<u8> = context
            .send_and_receive(
                route![NODEMANAGER_ADDR],
                Request::post("/node/inlet").body(payload).to_vec()?,
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
