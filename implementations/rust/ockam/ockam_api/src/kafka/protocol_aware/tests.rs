#[cfg(test)]
mod test {
    use crate::kafka::inlet_controller::KafkaInletController;
    use crate::kafka::protocol_aware::utils::{encode_request, encode_response};
    use crate::kafka::protocol_aware::InletInterceptorImpl;
    use crate::kafka::protocol_aware::KafkaMessageInterceptor;
    use crate::kafka::secure_channel_map::{KafkaEncryptedContent, KafkaSecureChannelController};
    use crate::port_range::PortRange;
    use kafka_protocol::messages::ApiKey;
    use kafka_protocol::messages::BrokerId;
    use kafka_protocol::messages::{ApiVersionsRequest, MetadataRequest, MetadataResponse};
    use kafka_protocol::messages::{ApiVersionsResponse, RequestHeader, ResponseHeader};
    use kafka_protocol::protocol::{Builder, StrBytes};
    use ockam_core::compat::sync::Arc;
    use ockam_core::route;
    use ockam_core::{async_trait, Address};
    use ockam_multiaddr::MultiAddr;
    use ockam_node::Context;

    struct DummySecureChannelController;

    #[async_trait]
    impl KafkaSecureChannelController for DummySecureChannelController {
        async fn encrypt_content_for(
            &self,
            _context: &mut Context,
            _topic_name: &str,
            _partition_id: i32,
            content: Vec<u8>,
        ) -> ockam_core::Result<KafkaEncryptedContent> {
            Ok(KafkaEncryptedContent {
                content,
                consumer_decryptor_address: Address::from_string("arbitrary string"),
            })
        }

        async fn decrypt_content_for(
            &self,
            _context: &mut Context,
            _consumer_decryptor_address: &Address,
            encrypted_content: Vec<u8>,
        ) -> ockam_core::Result<Vec<u8>> {
            Ok(encrypted_content)
        }

        async fn start_relays_for(
            &self,
            _context: &mut Context,
            _topic_id: &str,
            _partitions: Vec<i32>,
        ) -> ockam_core::Result<()> {
            Ok(())
        }
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5_000)]
    async fn interceptor__basic_messages_with_several_api_versions__parsed_correctly(
        context: &mut Context,
    ) -> ockam::Result<()> {
        let inlet_map = KafkaInletController::new(
            MultiAddr::default(),
            route![],
            route![],
            [127, 0, 0, 1].into(),
            PortRange::new(0, 0).unwrap(),
        );

        let interceptor = InletInterceptorImpl::new(
            Arc::new(DummySecureChannelController {}),
            Default::default(),
            inlet_map,
        );

        let mut correlation_id = 0;

        for api_version in 0..14 {
            let result = interceptor
                .intercept_request(
                    context,
                    encode_request(
                        &RequestHeader::builder()
                            .request_api_version(api_version)
                            .correlation_id(correlation_id)
                            .request_api_key(ApiKey::ApiVersionsKey as i16)
                            .unknown_tagged_fields(Default::default())
                            .client_id(None)
                            .build()
                            .unwrap(),
                        &ApiVersionsRequest::builder()
                            .client_software_name(StrBytes::from_str("mr. software"))
                            .client_software_version(StrBytes::from_str("1.0.0"))
                            .unknown_tagged_fields(Default::default())
                            .build()
                            .unwrap(),
                        api_version,
                        ApiKey::ApiVersionsKey,
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
                        &ResponseHeader::builder()
                            .correlation_id(correlation_id)
                            .unknown_tagged_fields(Default::default())
                            .build()
                            .unwrap(),
                        &ApiVersionsResponse::builder()
                            .error_code(0)
                            .api_keys(Default::default())
                            .throttle_time_ms(0)
                            .supported_features(Default::default())
                            .finalized_features_epoch(0)
                            .finalized_features(Default::default())
                            .unknown_tagged_fields(Default::default())
                            .build()
                            .unwrap(),
                        api_version,
                        ApiKey::ApiVersionsKey,
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
                        &RequestHeader::builder()
                            .request_api_version(api_version)
                            .correlation_id(correlation_id)
                            .request_api_key(ApiKey::MetadataKey as i16)
                            .unknown_tagged_fields(Default::default())
                            .client_id(None)
                            .build()
                            .unwrap(),
                        &MetadataRequest::builder()
                            .topics(None)
                            .allow_auto_topic_creation(true)
                            .include_cluster_authorized_operations(false)
                            .include_topic_authorized_operations(false)
                            .unknown_tagged_fields(Default::default())
                            .build()
                            .unwrap(),
                        api_version,
                        ApiKey::MetadataKey,
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
                        &ResponseHeader::builder()
                            .correlation_id(correlation_id)
                            .unknown_tagged_fields(Default::default())
                            .build()
                            .unwrap(),
                        &MetadataResponse::builder()
                            .throttle_time_ms(0)
                            .brokers(Default::default())
                            .cluster_id(None)
                            .controller_id(BrokerId::from(0_i32))
                            .cluster_authorized_operations(-2147483648)
                            .topics(Default::default())
                            .unknown_tagged_fields(Default::default())
                            .build()
                            .unwrap(),
                        api_version,
                        ApiKey::MetadataKey,
                    )
                    .unwrap(),
                )
                .await;

            if let Err(error) = result {
                panic!("unexpected error: {error:?}");
            }

            correlation_id += 1;
        }

        context.stop().await
    }
}
