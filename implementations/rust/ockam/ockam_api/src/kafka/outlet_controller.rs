use crate::kafka::kafka_outlet_address;
use crate::nodes::models::portal::{CreateOutlet, OutletStatus};
use crate::nodes::NODEMANAGER_ADDR;
use minicbor::Decoder;
use ockam::compat::tokio::sync::Mutex;
use ockam::transport::HostnamePort;
use ockam_abac::PolicyExpression;
use ockam_core::api::{Request, ResponseHeader, Status};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Error};
use ockam_core::{Address, Result};
use ockam_node::Context;
use std::net::SocketAddr;
use std::str::FromStr;

type BrokerId = i32;

/// Shared structure for every kafka worker (in the outlet service)
/// to keep track of which brokers are being proxied with the relative outlet socket address.
/// Also takes care of creating outlet dynamically when they are not present yet.
#[derive(Debug, Clone)]
pub(crate) struct KafkaOutletController {
    inner: Arc<Mutex<KafkaOutletMapInner>>,
    policy_expression: Option<PolicyExpression>,
    tls: bool,
}

#[derive(Debug)]
struct KafkaOutletMapInner {
    broker_map: HashMap<BrokerId, SocketAddr>,
}

impl KafkaOutletController {
    pub(crate) fn new(
        policy_expression: Option<PolicyExpression>,
        tls: bool,
    ) -> KafkaOutletController {
        Self {
            inner: Arc::new(Mutex::new(KafkaOutletMapInner {
                broker_map: HashMap::new(),
            })),
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
            let socket_address = Self::request_outlet_creation(
                context,
                address,
                kafka_outlet_address(broker_id),
                self.policy_expression.clone(),
                self.tls,
            )
            .await?;
            inner.broker_map.insert(broker_id, socket_address);
        }
        Ok(outlet_address)
    }

    async fn request_outlet_creation(
        context: &Context,
        kafka_address: String,
        worker_address: Address,
        policy_expression: Option<PolicyExpression>,
        tls: bool,
    ) -> Result<SocketAddr> {
        let hostname_port = HostnamePort::from_str(&kafka_address)?;
        let mut payload = CreateOutlet::new(hostname_port, tls, Some(worker_address), false);
        if let Some(expr) = policy_expression {
            payload.set_policy_expression(expr);
        }
        let buffer: Vec<u8> = context
            .send_and_receive(
                route![NODEMANAGER_ADDR],
                Request::post("/node/outlet").body(payload).to_vec()?,
            )
            .await?;

        let mut decoder = Decoder::new(&buffer);
        let response: ResponseHeader = decoder.decode()?;

        let status = response.status().unwrap_or(Status::InternalServerError);
        if status != Status::Ok {
            return Err(Error::new(
                Origin::Transport,
                Kind::Invalid,
                format!("cannot create outlet: {}", status),
            ));
        }
        if !response.has_body() {
            Err(Error::new(
                Origin::Transport,
                Kind::Unknown,
                "invalid create outlet response",
            ))
        } else {
            let status: OutletStatus = decoder.decode()?;
            Ok(status.socket_addr)
        }
    }
}
