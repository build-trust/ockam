use crate::kafka::kafka_outlet_address;
use crate::nodes::models::portal::OutletAccessControl;
use crate::nodes::NodeManager;
use ockam::compat::tokio::sync::Mutex;
use ockam::transport::HostnamePort;
use ockam_abac::PolicyExpression;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, Result};
use ockam_node::Context;
use std::fmt::Debug;
use std::str::FromStr;

type BrokerId = i32;

/// Shared structure for every kafka worker (in the outlet service)
/// to keep track of which brokers are being proxied with the relative outlet socket address.
/// Also takes care of creating outlet dynamically when they are not present yet.
#[derive(Clone)]
pub(crate) struct KafkaOutletController {
    inner: Arc<Mutex<KafkaOutletMapInner>>,
    policy_expression: Option<PolicyExpression>,
    tls: bool,
    node_manager: Arc<NodeManager>,
}

impl Debug for KafkaOutletController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KafkaOutletController {{ tls: {}, policy_expression: {:?}, outlets: {:?} }}",
            self.tls, self.policy_expression, self.inner
        )
    }
}

#[derive(Debug)]
struct KafkaOutletMapInner {
    broker_map: HashMap<BrokerId, HostnamePort>,
}

impl KafkaOutletController {
    pub(crate) fn new(
        node_manager: Arc<NodeManager>,
        policy_expression: Option<PolicyExpression>,
        tls: bool,
    ) -> KafkaOutletController {
        Self {
            inner: Arc::new(Mutex::new(KafkaOutletMapInner {
                broker_map: HashMap::new(),
            })),
            node_manager,
            policy_expression,
            tls,
        }
    }

    /// Asserts the presence of an outlet for a specific broker.
    /// The first time it'll create the inlet and return the relative address.
    /// After that, it'll just return the address
    pub(crate) async fn assert_outlet_for_broker(
        &self,
        context: &Context,
        broker_id: BrokerId,
        address: String,
    ) -> Result<Address> {
        let outlet_address = kafka_outlet_address(broker_id);
        let mut inner = self.inner.lock().await;
        if !inner.broker_map.contains_key(&broker_id) {
            let hostname_port = self
                .node_manager
                .create_outlet(
                    context,
                    HostnamePort::from_str(&address)?,
                    self.tls,
                    Some(kafka_outlet_address(broker_id)),
                    false,
                    OutletAccessControl::WithPolicyExpression(self.policy_expression.clone()),
                )
                .await
                .map(|info| info.to)?;

            inner.broker_map.insert(broker_id, hostname_port);
        }
        Ok(outlet_address)
    }
}
