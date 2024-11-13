use crate::kafka::kafka_outlet_address;
use crate::nodes::NodeManager;
use crate::port_range::PortRange;
use ockam::compat::tokio::sync::Mutex;
use ockam_abac::PolicyExpression;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::rand::random_string;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_core::{Result, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_core::HostnamePort;
use std::fmt::Debug;
use std::sync::Weak;

type BrokerId = i32;

/// Shared structure for every kafka worker (consumer or producer services)
/// to keep track of which brokers are being proxied with the relative inlet listener socket address.
/// Also takes care of creating inlets dynamically when they are not present yet.
#[derive(Clone)]
pub(crate) struct KafkaInletController {
    inner: Arc<Mutex<KafkaInletMapInner>>,
    policy_expression: Option<PolicyExpression>,
    node_manager: Weak<NodeManager>,
}

impl Debug for KafkaInletController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KafkaInletController {{ policy_expression: {:?}, inlets: {:?} }}",
            self.policy_expression, self.inner
        )
    }
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
        node_manager: Arc<NodeManager>,
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
            node_manager: Arc::downgrade(&node_manager),
            policy_expression,
        }
    }

    #[cfg(test)]
    pub(crate) fn stub() -> KafkaInletController {
        Self {
            inner: Arc::new(Mutex::new(KafkaInletMapInner {
                outlet_node_multiaddr: Default::default(),
                broker_map: Default::default(),
                current_port: Default::default(),
                port_range: PortRange::new(0, 0).unwrap(),
                bind_hostname: Default::default(),
                local_interceptor_route: Default::default(),
                remote_interceptor_route: Default::default(),
            })),
            node_manager: Weak::new(),
            policy_expression: Default::default(),
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
                HostnamePort::new(inner.bind_hostname.clone(), inner.current_port)?;

            let node_manager = self.node_manager.upgrade().ok_or_else(|| {
                Error::new(Origin::Node, Kind::Internal, "node manager was shut down")
            })?;

            node_manager
                .create_inlet(
                    context,
                    inlet_bind_address.clone(),
                    inner.local_interceptor_route.clone(),
                    inner.remote_interceptor_route.clone() + kafka_outlet_address(broker_id),
                    inner.outlet_node_multiaddr.clone(),
                    format!("kafka-inlet-{}", random_string()),
                    self.policy_expression.clone(),
                    None,
                    None,
                    false,
                    None,
                    false,
                    false,
                    false,
                    None,
                )
                .await?;

            inner.current_port += 1;
            inner
                .broker_map
                .insert(broker_id, inlet_bind_address.clone());

            Ok(inlet_bind_address)
        }
    }
}
