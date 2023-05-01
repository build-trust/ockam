#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use bytes::{Buf, BufMut, BytesMut};
    use indexmap::IndexMap;
    use kafka_protocol::messages::produce_request::{PartitionProduceData, TopicProduceData};
    use kafka_protocol::messages::{
        fetch_request::{FetchPartition, FetchTopic},
        fetch_response::FetchableTopicResponse,
        fetch_response::PartitionData,
        ApiKey, BrokerId, FetchRequest, FetchResponse, ProduceRequest, RequestHeader,
        ResponseHeader, TopicName,
    };
    use kafka_protocol::protocol::Builder;
    use kafka_protocol::protocol::Decodable as KafkaDecodable;
    use kafka_protocol::protocol::Encodable as KafkaEncodable;
    use kafka_protocol::protocol::StrBytes;
    use kafka_protocol::records::Record;
    use kafka_protocol::records::{
        Compression, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions, TimestampType,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::net::TcpStream;
    use tokio::sync::Mutex;
    use tokio::task::JoinHandle;
    use uuid::Uuid;

    use ockam::compat::tokio::io::DuplexStream;
    use ockam::Context;
    use ockam_core::async_trait;
    use ockam_core::compat::sync::Arc;
    use ockam_core::flow_control::FlowControlPolicy;
    use ockam_core::route;
    use ockam_core::{Address, AllowAll};
    use ockam_identity::SecureChannelListenerOptions;
    use ockam_multiaddr::proto::Service;
    use ockam_multiaddr::MultiAddr;
    use ockam_node::compat::tokio;
    use ockam_transport_tcp::{TcpInletOptions, TcpOutletOptions};

    use crate::hop::Hop;
    use crate::kafka::protocol_aware::utils::{encode_request, encode_response};
    use crate::kafka::secure_channel_map::ForwarderCreator;
    use crate::kafka::{
        KafkaInletController, KafkaPortalListener, KafkaSecureChannelControllerImpl,
    };
    use crate::nodes::registry::KafkaServiceKind;
    use crate::test::NodeManagerHandle;
    use crate::DefaultAddress;

    //TODO: upgrade to 13 by adding a metadata request to map uuid<=>topic_name
    const TEST_KAFKA_API_VERSION: i16 = 12;

    struct HopForwarderCreator {}

    #[async_trait]
    impl ForwarderCreator for HopForwarderCreator {
        async fn create_forwarder(&self, context: &Context, alias: String) -> ockam::Result<()> {
            trace!("creating mock forwarder for: {alias}");
            //replicating the same logic of the orchestrator by adding consumer__
            context
                .start_worker(
                    Address::from_string(format!("consumer__{alias}")),
                    Hop,
                    AllowAll,
                    AllowAll,
                )
                .await?;
            Ok(())
        }
    }

    async fn create_kafka_service(
        context: &Context,
        handle: &NodeManagerHandle,
        listener_address: Address,
        outlet_address: Address,
        kind: KafkaServiceKind,
    ) -> ockam::Result<u16> {
        let secure_channel_controller = KafkaSecureChannelControllerImpl::new_extended(
            handle.secure_channels.clone(),
            MultiAddr::try_from("/service/api")?,
            HopForwarderCreator {},
        );

        //the possibility to accept secure channels is the only real
        //difference between consumer and producer
        if let KafkaServiceKind::Consumer = kind {
            secure_channel_controller
                .create_consumer_listener(context)
                .await?;

            let options = SecureChannelListenerOptions::new();
            context.flow_controls().add_consumer(
                crate::kafka::KAFKA_SECURE_CHANNEL_CONTROLLER_ADDRESS,
                &options.spawner_flow_control_id(),
                FlowControlPolicy::SpawnerAllowMultipleMessages,
            );

            // in a normal setup the secure channel listener is already created
            handle
                .secure_channels
                .create_secure_channel_listener(
                    context,
                    &handle.identifier,
                    DefaultAddress::SECURE_CHANNEL_LISTENER,
                    options,
                )
                .await?;
        }

        let mut interceptor_multiaddr = MultiAddr::default();
        interceptor_multiaddr.push_back(Service::new(listener_address.address()))?;

        let inlet_controller = KafkaInletController::new(
            interceptor_multiaddr,
            route![],
            route![],
            "127.0.0.1".parse().unwrap(),
            (0, 0).try_into().unwrap(),
        );

        let (socket_address, _) = handle
            .tcp
            .create_inlet(
                "127.0.0.1:0",
                route![listener_address.clone(), outlet_address.clone()],
                TcpInletOptions::new(),
            )
            .await?;

        KafkaPortalListener::create(
            context,
            inlet_controller,
            secure_channel_controller.into_trait(),
            listener_address,
        )
        .await?;

        Ok(socket_address.port())
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5_000)]
    async fn producer__flow_with_mock_kafka__content_encryption_and_decryption(
        context: &mut Context,
    ) -> ockam::Result<()> {
        let handler = crate::util::test::start_manager_for_tests(context).await?;

        let consumer_bootstrap_port = create_kafka_service(
            context,
            &handler,
            "kafka_consumer_listener".into(),
            "kafka_consumer_outlet".into(),
            KafkaServiceKind::Consumer,
        )
        .await?;

        let producer_bootstrap_port = create_kafka_service(
            context,
            &handler,
            "kafka_producer_listener".into(),
            "kafka_producer_outlet".into(),
            KafkaServiceKind::Producer,
        )
        .await?;

        //before produce a new key, the consumer has to issue a Fetch request
        // so the sidecar can react by creating the forwarder for the partition 1 of 'my-topic'
        {
            let mut consumer_mock_kafka = TcpServerSimulator::start("127.0.0.1:0").await;
            handler
                .tcp
                .create_outlet(
                    "kafka_consumer_outlet",
                    format!("127.0.0.1:{}", consumer_mock_kafka.port),
                    TcpOutletOptions::new(),
                )
                .await?;

            simulate_first_kafka_consumer_empty_reply_and_ignore_result(
                consumer_bootstrap_port,
                &mut consumer_mock_kafka,
            )
            .await;
            drop(consumer_mock_kafka);
            //drop the outlet and re-create it when we need it later
            context.stop_worker("kafka_consumer_outlet").await?;
        }

        let mut producer_mock_kafka = TcpServerSimulator::start("127.0.0.1:0").await;
        handler
            .tcp
            .create_outlet(
                "kafka_producer_outlet",
                format!("127.0.0.1:{}", producer_mock_kafka.port),
                TcpOutletOptions::new(),
            )
            .await?;
        let request = simulate_kafka_producer_and_read_request(
            producer_bootstrap_port,
            &mut producer_mock_kafka,
        )
        .await;

        let encrypted_body = request
            .topic_data
            .iter()
            .next()
            .as_ref()
            .unwrap()
            .1
            .partition_data
            .get(0)
            .unwrap()
            .records
            .as_ref()
            .unwrap();

        let mut encrypted_body = BytesMut::from(encrypted_body.as_ref());
        let records = RecordBatchDecoder::decode(&mut encrypted_body).unwrap();

        assert_ne!(
            records.get(0).unwrap().value.as_ref().unwrap(),
            "hello world!".as_bytes()
        );

        let mut consumer_mock_kafka = TcpServerSimulator::start("127.0.0.1:0").await;
        handler
            .tcp
            .create_outlet(
                "kafka_consumer_outlet",
                format!("127.0.0.1:{}", consumer_mock_kafka.port),
                TcpOutletOptions::new(),
            )
            .await?;
        let plain_fetch_response = simulate_kafka_consumer_and_read_response(
            consumer_bootstrap_port,
            &mut consumer_mock_kafka,
            &request,
        )
        .await;

        let plain_content = plain_fetch_response
            .responses
            .get(0)
            .as_ref()
            .unwrap()
            .partitions
            .get(0)
            .as_ref()
            .unwrap()
            .records
            .as_ref()
            .unwrap();

        let mut plain_content = BytesMut::from(plain_content.as_ref());
        let records = RecordBatchDecoder::decode(&mut plain_content).unwrap();

        assert_eq!(
            records.get(0).as_ref().unwrap().value.as_ref().unwrap(),
            "hello world!".as_bytes()
        );

        context.stop().await?;
        consumer_mock_kafka.destroy_and_wait().await;
        producer_mock_kafka.destroy_and_wait().await;
        Ok(())
    }

    async fn simulate_kafka_producer_and_read_request(
        producer_bootstrap_port: u16,
        producer_mock_kafka: &mut TcpServerSimulator,
    ) -> ProduceRequest {
        let mut kafka_client_connection =
            TcpStream::connect(format!("127.0.0.1:{producer_bootstrap_port}"))
                .await
                .unwrap();
        send_kafka_produce_request(&mut kafka_client_connection).await;
        read_kafka_request::<&mut DuplexStream, RequestHeader, ProduceRequest>(
            producer_mock_kafka.stream(),
            ApiKey::ProduceKey,
        )
        .await
    }

    async fn send_kafka_produce_request(stream: &mut TcpStream) {
        let header = RequestHeader::builder()
            .request_api_key(ApiKey::ProduceKey as i16)
            .request_api_version(TEST_KAFKA_API_VERSION)
            .correlation_id(1)
            .client_id(Some(StrBytes::from_str("my-client-id")))
            .unknown_tagged_fields(Default::default())
            .build()
            .unwrap();

        let mut encoded = BytesMut::new();
        RecordBatchEncoder::encode(
            &mut encoded,
            vec![Record {
                transactional: false,
                control: false,
                partition_leader_epoch: 0,
                producer_id: 0,
                producer_epoch: 0,
                timestamp_type: TimestampType::Creation,
                offset: 0,
                sequence: 0,
                timestamp: 0,
                key: None,
                value: Some(BytesMut::from("hello world!").freeze()),
                headers: Default::default(),
            }]
            .iter(),
            &RecordEncodeOptions {
                version: 2,
                compression: Compression::None,
            },
        )
        .unwrap();

        let mut topic_data = IndexMap::new();
        topic_data.insert(
            TopicName::from(StrBytes::from_str("my-topic-name")),
            TopicProduceData::builder()
                .partition_data(vec![PartitionProduceData::builder()
                    .index(1)
                    .records(Some(encoded.freeze()))
                    .unknown_tagged_fields(Default::default())
                    .build()
                    .unwrap()])
                .unknown_tagged_fields(Default::default())
                .build()
                .unwrap(),
        );
        let request = ProduceRequest::builder()
            .transactional_id(None)
            .acks(0)
            .timeout_ms(0)
            .topic_data(topic_data)
            .unknown_tagged_fields(Default::default())
            .build()
            .unwrap();

        send_kafka_request(stream, header, request, ApiKey::ProduceKey).await;
    }

    //this is needed in order to make the consumer create the forwarders to the secure
    //channel
    async fn simulate_first_kafka_consumer_empty_reply_and_ignore_result(
        consumer_bootstrap_port: u16,
        mock_kafka_connection: &mut TcpServerSimulator,
    ) {
        let mut kafka_client_connection =
            TcpStream::connect(format!("127.0.0.1:{consumer_bootstrap_port}"))
                .await
                .unwrap();
        send_kafka_fetch_request(&mut kafka_client_connection).await;
        //we don't want the answer, but we need to be sure the
        // message passed through and the forwarder had been created
        mock_kafka_connection
            .stream
            .read_exact(&mut [0; 4])
            .await
            .unwrap();
    }

    //we use the encrypted producer request to generate the encrypted fetch response
    async fn simulate_kafka_consumer_and_read_response(
        consumer_bootstrap_port: u16,
        mock_kafka_connection: &mut TcpServerSimulator,
        producer_request: &ProduceRequest,
    ) -> FetchResponse {
        let mut kafka_client_connection =
            TcpStream::connect(format!("127.0.0.1:{consumer_bootstrap_port}"))
                .await
                .unwrap();
        send_kafka_fetch_request(&mut kafka_client_connection).await;
        let _fetch_request: FetchRequest =
            read_kafka_request::<&mut DuplexStream, RequestHeader, FetchRequest>(
                mock_kafka_connection.stream(),
                ApiKey::FetchKey,
            )
            .await;

        send_kafka_fetch_response(mock_kafka_connection.stream(), producer_request).await;
        read_kafka_response::<&mut TcpStream, ResponseHeader, FetchResponse>(
            &mut kafka_client_connection,
            ApiKey::FetchKey,
        )
        .await
    }

    async fn send_kafka_fetch_response<S: AsyncWriteExt + Unpin>(
        stream: S,
        producer_request: &ProduceRequest,
    ) {
        let topic_name = TopicName::from(StrBytes::from_str("my-topic-name"));
        let producer_content = producer_request
            .topic_data
            .get(&topic_name)
            .unwrap()
            .partition_data
            .get(0)
            .unwrap()
            .records
            .clone();

        send_kafka_response(
            stream,
            ResponseHeader::builder()
                .correlation_id(1)
                .unknown_tagged_fields(Default::default())
                .build()
                .unwrap(),
            FetchResponse::builder()
                .throttle_time_ms(Default::default())
                .error_code(Default::default())
                .session_id(Default::default())
                .responses(vec![FetchableTopicResponse::builder()
                    .topic(topic_name)
                    .topic_id(Default::default())
                    .partitions(vec![PartitionData::builder()
                        .partition_index(1)
                        .error_code(Default::default())
                        .high_watermark(Default::default())
                        .last_stable_offset(Default::default())
                        .log_start_offset(Default::default())
                        .diverging_epoch(Default::default())
                        .current_leader(Default::default())
                        .snapshot_id(Default::default())
                        .aborted_transactions(Default::default())
                        .preferred_read_replica(Default::default())
                        .records(producer_content)
                        .unknown_tagged_fields(Default::default())
                        .build()
                        .unwrap()])
                    .unknown_tagged_fields(Default::default())
                    .build()
                    .unwrap()])
                .unknown_tagged_fields(Default::default())
                .build()
                .unwrap(),
            ApiKey::FetchKey,
        )
        .await;
    }

    async fn send_kafka_fetch_request(stream: &mut TcpStream) {
        send_kafka_request(
            stream,
            RequestHeader::builder()
                .request_api_key(ApiKey::FetchKey as i16)
                .request_api_version(TEST_KAFKA_API_VERSION)
                .correlation_id(1)
                .client_id(Some(StrBytes::from_str("my-client-id")))
                .unknown_tagged_fields(Default::default())
                .build()
                .unwrap(),
            FetchRequest::builder()
                .cluster_id(None)
                .replica_id(BrokerId::default())
                .max_wait_ms(0)
                .min_bytes(0)
                .max_bytes(0)
                .isolation_level(0)
                .session_id(0)
                .session_epoch(0)
                .topics(vec![FetchTopic::builder()
                    .topic(TopicName::from(StrBytes::from_str("my-topic-name")))
                    .topic_id(Uuid::from_slice(b"my-topic-name___").unwrap())
                    .partitions(vec![FetchPartition::builder()
                        .partition(1)
                        .current_leader_epoch(0)
                        .fetch_offset(0)
                        .last_fetched_epoch(0)
                        .log_start_offset(0)
                        .partition_max_bytes(0)
                        .unknown_tagged_fields(Default::default())
                        .build()
                        .unwrap()])
                    .unknown_tagged_fields(Default::default())
                    .build()
                    .unwrap()])
                .forgotten_topics_data(Default::default())
                .rack_id(Default::default())
                .unknown_tagged_fields(Default::default())
                .build()
                .unwrap(),
            ApiKey::FetchKey,
        )
        .await;
    }

    async fn send_kafka_request<S: AsyncWriteExt + Unpin, H: KafkaEncodable, T: KafkaEncodable>(
        mut stream: S,
        header: H,
        body: T,
        api_key: ApiKey,
    ) {
        let encoded = encode_request(&header, &body, TEST_KAFKA_API_VERSION, api_key).unwrap();

        let mut request_buffer = BytesMut::new();
        request_buffer.put_u32(encoded.len() as u32);
        request_buffer.put_slice(&encoded);

        trace!("send_kafka_request...");
        stream.write_all(&request_buffer).await.unwrap();
        stream.flush().await.unwrap();
        trace!("send_kafka_request...done");
    }

    async fn send_kafka_response<S: AsyncWriteExt + Unpin, H: KafkaEncodable, T: KafkaEncodable>(
        mut stream: S,
        header: H,
        body: T,
        api_key: ApiKey,
    ) {
        let encoded = encode_response(&header, &body, TEST_KAFKA_API_VERSION, api_key).unwrap();

        let mut request_buffer = BytesMut::new();
        request_buffer.put_u32(encoded.len() as u32);
        request_buffer.put_slice(&encoded);

        trace!("send_kafka_response...");
        stream.write_all(&request_buffer).await.unwrap();
        stream.flush().await.unwrap();
        trace!("send_kafka_response...done");
    }

    async fn read_kafka_request<S: AsyncReadExt + Unpin, H: KafkaDecodable, T: KafkaDecodable>(
        mut stream: S,
        api_key: ApiKey,
    ) -> T {
        trace!("read_kafka_request...");
        let header_and_request_buffer = read_packet(&mut stream).await;
        let mut header_and_request_buffer = BytesMut::from(header_and_request_buffer.as_slice());

        let _header = H::decode(
            &mut header_and_request_buffer,
            api_key.request_header_version(TEST_KAFKA_API_VERSION),
        )
        .unwrap();
        let request = T::decode(&mut header_and_request_buffer, TEST_KAFKA_API_VERSION).unwrap();
        trace!("read_kafka_request...done");
        request
    }

    async fn read_kafka_response<S: AsyncReadExt + Unpin, H: KafkaDecodable, T: KafkaDecodable>(
        mut stream: S,
        api_key: ApiKey,
    ) -> T {
        trace!("read_kafka_response...");
        let header_and_request_buffer = read_packet(&mut stream).await;
        let mut header_and_request_buffer = BytesMut::from(header_and_request_buffer.as_slice());

        let _header = H::decode(
            &mut header_and_request_buffer,
            api_key.response_header_version(TEST_KAFKA_API_VERSION),
        )
        .unwrap();
        let request = T::decode(&mut header_and_request_buffer, TEST_KAFKA_API_VERSION).unwrap();
        trace!("read_kafka_response...done");
        request
    }

    async fn read_packet<S: AsyncReadExt + Unpin>(stream: &mut S) -> [u8; 1024] {
        trace!("read_packet...");
        let size = {
            let mut length_buffer = [0; 4];
            let read = stream.read_exact(&mut length_buffer).await.unwrap();

            assert_eq!(4, read);
            BytesMut::from(length_buffer.as_slice()).get_u32()
        };
        info!("incoming message size: {size}");

        let mut header_and_request_buffer = [0; 1024];
        let read = stream
            .read_exact(&mut header_and_request_buffer[0..size as usize])
            .await
            .unwrap();
        assert_eq!(size as usize, read);

        trace!("read_kafka_request...done");

        header_and_request_buffer
    }

    struct TcpServerSimulator {
        stream: DuplexStream,
        join_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
        is_stopping: Arc<AtomicBool>,
        port: u16,
    }

    impl TcpServerSimulator {
        pub fn stream(&mut self) -> &mut DuplexStream {
            &mut self.stream
        }

        /// Stops every async task running and wait for completion
        /// must be called to avoid leaks to be sure everything is closed before
        /// moving on the next test
        pub async fn destroy_and_wait(self) {
            self.is_stopping.store(true, Ordering::SeqCst);
            //we want to close the channel _before_ joining current handles to interrupt them
            drop(self.stream);
            let mut guard = self.join_handles.lock().await;
            for handle in guard.iter_mut() {
                //we don't care about failures
                let _ = handle.await;
            }
        }

        /// Starts a tcp listener for one connection and returns a virtual buffer
        /// linked to the first socket
        pub async fn start(address: &str) -> Self {
            let listener = TcpListener::bind(address).await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let join_handles: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::new(Mutex::new(Vec::new()));
            let is_stopping = Arc::new(AtomicBool::new(false));

            let (test_side_duplex, simulator_side_duplex) = tokio::io::duplex(4096);
            let (simulator_read_half, simulator_write_half) =
                tokio::io::split(simulator_side_duplex);

            let handle: JoinHandle<()> = {
                let is_stopping = is_stopping.clone();
                let join_handles = join_handles.clone();
                tokio::spawn(async move {
                    let socket;
                    loop {
                        //tokio would block on the listener forever, we need to poll a little in
                        //order to interrupt it
                        let timeout_future =
                            tokio::time::timeout(Duration::from_millis(200), listener.accept());
                        if let Ok(result) = timeout_future.await {
                            match result {
                                Ok((current_socket, _)) => {
                                    socket = current_socket;
                                    break;
                                }
                                Err(_) => {
                                    return;
                                }
                            }
                        }
                        if is_stopping.load(Ordering::SeqCst) {
                            return;
                        }
                    }

                    let (socket_read_half, socket_write_half) = socket.into_split();
                    let handle: JoinHandle<()> = tokio::spawn(async move {
                        Self::relay_traffic(
                            "socket_read_half",
                            socket_read_half,
                            "simulator_write_half",
                            simulator_write_half,
                        )
                        .await
                    });
                    join_handles.lock().await.push(handle);

                    let handle: JoinHandle<()> = tokio::spawn(async move {
                        Self::relay_traffic(
                            "simulator_read_half",
                            simulator_read_half,
                            "socket_write_half",
                            socket_write_half,
                        )
                        .await
                    });
                    join_handles.lock().await.push(handle);
                })
            };
            join_handles.lock().await.push(handle);

            Self {
                stream: test_side_duplex,
                port,
                join_handles,
                is_stopping,
            }
        }

        async fn relay_traffic<W: AsyncWriteExt + Unpin, R: AsyncReadExt + Unpin>(
            read_half_name: &'static str,
            mut read_half: R,
            write_half_name: &'static str,
            mut write_half: W,
        ) {
            let mut buffer = [0; 1024];
            loop {
                let read = match read_half.read(&mut buffer).await {
                    Ok(read) => read,
                    Err(err) => {
                        warn!("{write_half_name} error: closing channel: {:?}", err);
                        break;
                    }
                };

                if read == 0 {
                    info!("{read_half_name} returned empty buffer: clean channel close");
                    break;
                }
                if write_half.write(&buffer[0..read]).await.is_err() {
                    warn!("{write_half_name} error: closing channel");
                    break;
                }
            }
        }
    }
}
