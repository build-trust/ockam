#[cfg(test)]
mod test {
    use crate::kafka::inlet_controller::KafkaInletController;
    use crate::kafka::key_exchange::controller::KafkaKeyExchangeControllerImpl;
    use crate::kafka::protocol_aware::inlet::InletInterceptorImpl;
    use crate::kafka::protocol_aware::utils::{encode_request, encode_response};
    use crate::kafka::protocol_aware::{
        KafkaMessageRequestInterceptor, KafkaMessageResponseInterceptor,
    };
    use crate::kafka::{ConsumerPublishing, ConsumerResolution};
    use crate::port_range::PortRange;
    use crate::test_utils::TestNode;
    use kafka_protocol::messages::ApiKey;
    use kafka_protocol::messages::BrokerId;
    use kafka_protocol::messages::{ApiVersionsRequest, MetadataRequest, MetadataResponse};
    use kafka_protocol::messages::{ApiVersionsResponse, RequestHeader, ResponseHeader};
    use kafka_protocol::protocol::StrBytes;
    use ockam_abac::{Action, Env, Resource, ResourceType};
    use ockam_core::route;
    use ockam_multiaddr::MultiAddr;
    use ockam_node::Context;
    use std::sync::Arc;

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5_000)]
    async fn interceptor__basic_messages_with_several_api_versions__parsed_correctly(
        context: &mut Context,
    ) -> ockam::Result<()> {
        TestNode::clean().await?;
        let handle = crate::test_utils::start_manager_for_tests(context, None, None).await?;

        let inlet_map = KafkaInletController::new(
            (*handle.node_manager).clone(),
            MultiAddr::default(),
            route![],
            route![],
            "127.0.0.1".to_string(),
            PortRange::new(0, 0).unwrap(),
            None,
        );

        let secure_channels = handle.node_manager.secure_channels();
        let policies = handle.node_manager.policies();

        let consumer_policy_access_control = policies.make_policy_access_control(
            secure_channels.identities().identities_attributes(),
            Resource::new("arbitrary-resource-name", ResourceType::KafkaConsumer),
            Action::HandleMessage,
            Env::new(),
            Some(handle.node_manager.identifier()),
        );

        let producer_policy_access_control = policies.make_policy_access_control(
            secure_channels.identities().identities_attributes(),
            Resource::new("arbitrary-resource-name", ResourceType::KafkaProducer),
            Action::HandleMessage,
            Env::new(),
            Some(handle.node_manager.identifier()),
        );

        let secure_channel_controller = KafkaKeyExchangeControllerImpl::new(
            (*handle.node_manager).clone(),
            secure_channels,
            ConsumerResolution::None,
            ConsumerPublishing::None,
            consumer_policy_access_control,
            producer_policy_access_control,
        );

        let interceptor = InletInterceptorImpl::new(
            Arc::new(secure_channel_controller),
            Default::default(),
            inlet_map,
            true,
            vec![],
        );

        let mut correlation_id = 0;

        for api_version in 0..14 {
            let result = interceptor
                .intercept_request(
                    context,
                    encode_request(
                        &RequestHeader::default()
                            .with_request_api_version(api_version)
                            .with_correlation_id(correlation_id)
                            .with_request_api_key(ApiKey::ApiVersions as i16),
                        &ApiVersionsRequest::default()
                            .with_client_software_name(StrBytes::from_static_str("mr. software"))
                            .with_client_software_version(StrBytes::from_static_str("1.0.0")),
                        api_version,
                        ApiKey::ApiVersions,
                    )
                    .unwrap(),
                )
                .await;

            if let Err(error) = result {
                panic!("unexpected error: {error:?}");
            }

            let result = interceptor
                .intercept_response(
                    context,
                    encode_response(
                        &ResponseHeader::default().with_correlation_id(correlation_id),
                        &ApiVersionsResponse::default(),
                        api_version,
                        ApiKey::ApiVersions,
                    )
                    .unwrap(),
                )
                .await;

            if let Err(error) = result {
                panic!("unexpected error: {error:?}");
            }

            correlation_id += 1;
        }

        for api_version in 0..14 {
            let result = interceptor
                .intercept_request(
                    context,
                    encode_request(
                        &RequestHeader::default()
                            .with_request_api_version(api_version)
                            .with_correlation_id(correlation_id)
                            .with_request_api_key(ApiKey::Metadata as i16),
                        &MetadataRequest::default(),
                        api_version,
                        ApiKey::Metadata,
                    )
                    .unwrap(),
                )
                .await;

            if let Err(error) = result {
                panic!("unexpected error: {error:?}");
            }

            let result = interceptor
                .intercept_response(
                    context,
                    encode_response(
                        &ResponseHeader::default().with_correlation_id(correlation_id),
                        &MetadataResponse::default().with_controller_id(BrokerId::from(0_i32)),
                        api_version,
                        ApiKey::Metadata,
                    )
                    .unwrap(),
                )
                .await;

            if let Err(error) = result {
                panic!("unexpected error: {error:?}");
            }

            correlation_id += 1;
        }
        Ok(())
    }
}
