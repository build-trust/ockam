use crate::kafka::{ConsumerPublishing, ConsumerResolution};
use crate::nodes::models::portal::{InletStatusView, OutletStatus};
use crate::nodes::models::services::{
    StartKafkaInletRequest, StartKafkaOutletRequest, StartServiceRequest,
};
use crate::nodes::BackgroundNodeClient;
use crate::port_range::PortRange;
use ockam_abac::PolicyExpression;
use ockam_core::api::{Reply, Request};
use ockam_core::async_trait;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_core::HostnamePort;

#[async_trait]
pub trait KafkaPortals {
    #[allow(clippy::too_many_arguments)]
    async fn create_kafka_inlet(
        &self,
        ctx: &Context,
        name: &str,
        bind_address: HostnamePort,
        brokers_port_range: PortRange,
        kafka_outlet_route: MultiAddr,
        encrypt_content: bool,
        encrypted_fields: Vec<String>,
        consumer_resolution: ConsumerResolution,
        consumer_publishing: ConsumerPublishing,
        inlet_policy_expression: Option<PolicyExpression>,
        consumer_policy_expression: Option<PolicyExpression>,
        producer_policy_expression: Option<PolicyExpression>,
    ) -> miette::Result<Reply<InletStatusView>>;

    async fn create_kafka_outlet(
        &self,
        ctx: &Context,
        name: &str,
        bootstrap_server_addr: HostnamePort,
        tls: bool,
        policy_expression: Option<PolicyExpression>,
    ) -> miette::Result<Reply<OutletStatus>>;
}

#[async_trait]
impl KafkaPortals for BackgroundNodeClient {
    async fn create_kafka_inlet(
        &self,
        ctx: &Context,
        name: &str,
        bind_address: HostnamePort,
        brokers_port_range: PortRange,
        kafka_outlet_route: MultiAddr,
        encrypt_content: bool,
        encrypted_fields: Vec<String>,
        consumer_resolution: ConsumerResolution,
        consumer_publishing: ConsumerPublishing,
        inlet_policy_expression: Option<PolicyExpression>,
        consumer_policy_expression: Option<PolicyExpression>,
        producer_policy_expression: Option<PolicyExpression>,
    ) -> miette::Result<Reply<InletStatusView>> {
        let request = {
            let payload = StartKafkaInletRequest::new(
                bind_address,
                brokers_port_range,
                kafka_outlet_route,
                encrypt_content,
                encrypted_fields,
                consumer_resolution,
                consumer_publishing,
                inlet_policy_expression,
                consumer_policy_expression,
                producer_policy_expression,
            );
            let payload = StartServiceRequest::new(payload, name);
            Request::post("/node/services/kafka_inlet").body(payload)
        };
        self.ask_and_get_reply(ctx, request).await
    }

    async fn create_kafka_outlet(
        &self,
        ctx: &Context,
        name: &str,
        bootstrap_server_addr: HostnamePort,
        tls: bool,
        policy_expression: Option<PolicyExpression>,
    ) -> miette::Result<Reply<OutletStatus>> {
        let request = {
            let payload =
                StartKafkaOutletRequest::new(bootstrap_server_addr, tls, policy_expression);
            let payload = StartServiceRequest::new(payload, name);
            Request::post("/node/services/kafka_outlet").body(payload)
        };
        self.ask_and_get_reply(ctx, request).await
    }
}
