use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use kafka_protocol::messages::produce_request::{PartitionProduceData, TopicProduceData};
use kafka_protocol::messages::{
    fetch_request::{FetchPartition, FetchTopic},
    fetch_response::FetchableTopicResponse,
    fetch_response::PartitionData,
    ApiKey, FetchRequest, FetchResponse, ProduceRequest, RequestHeader, ResponseHeader, TopicName,
};
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

use crate::kafka::key_exchange::controller::KafkaKeyExchangeControllerImpl;
use crate::kafka::protocol_aware::inlet::KafkaInletInterceptorFactory;
use crate::kafka::protocol_aware::utils::{encode_request, encode_response};
use crate::kafka::{ConsumerPublishing, ConsumerResolution, KafkaInletController};
use crate::test_utils::{NodeManagerHandle, TestNode};
use ockam::compat::tokio::io::DuplexStream;
use ockam::tcp::{TcpInletOptions, TcpOutletOptions};
use ockam::Context;
use ockam_abac::{Action, Resource, ResourceType};
use ockam_core::compat::sync::Arc;
use ockam_core::route;
use ockam_core::Address;
use ockam_core::AllowAll;
use ockam_multiaddr::proto::Service;
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::tokio;
use ockam_transport_core::HostnamePort;
use ockam_transport_tcp::{read_portal_payload_length, PortalInletInterceptor};

// TODO: upgrade to 13 by adding a metadata request to map uuid<=>topic_name
const TEST_KAFKA_API_VERSION: i16 = 12;

async fn create_kafka_service(
    context: &Context,
    handle: &NodeManagerHandle,
    listener_address: Address,
    outlet_address: Address,
) -> ockam::Result<u16> {
    let project_authority = handle
        .node_manager
        .node_manager
        .project_authority()
        .unwrap();

    let consumer_policy_access_control = handle
        .node_manager
        .policy_access_control(
            Some(project_authority.clone()),
            Resource::new(listener_address.address(), ResourceType::KafkaConsumer),
            Action::HandleMessage,
            None,
        )
        .await?;

    let producer_policy_access_control = handle
        .node_manager
        .policy_access_control(
            Some(project_authority.clone()),
            Resource::new(listener_address.address(), ResourceType::KafkaProducer),
            Action::HandleMessage,
            None,
        )
        .await?;

    let secure_channel_controller = KafkaKeyExchangeControllerImpl::new(
        (*handle.node_manager).clone(),
        handle.secure_channels.clone(),
        ConsumerResolution::ViaRelay(MultiAddr::try_from("/service/api")?),
        ConsumerPublishing::None,
        consumer_policy_access_control,
        producer_policy_access_control,
    );

    let mut interceptor_multiaddr = MultiAddr::default();
    interceptor_multiaddr.push_back(Service::new(listener_address.address()))?;

    let inlet_controller = KafkaInletController::new(
        (*handle.node_manager).clone(),
        interceptor_multiaddr,
        route![],
        route![],
        "127.0.0.1".parse().unwrap(),
        (0, 0).try_into().unwrap(),
        None,
    );

    let inlet = handle
        .tcp
        .create_inlet(
            "127.0.0.1:0",
            route![listener_address.clone(), outlet_address.clone()],
            TcpInletOptions::new(),
        )
        .await?;

    PortalInletInterceptor::create(
        context,
        listener_address,
        Arc::new(KafkaInletInterceptorFactory::new(
            secure_channel_controller,
            inlet_controller,
            true,
            vec![],
        )),
        Arc::new(AllowAll),
        Arc::new(AllowAll),
        read_portal_payload_length(),
    )?;

    Ok(inlet.socket_address().port())
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 60_000)]
async fn producer__flow_with_mock_kafka__content_encryption_and_decryption(
    context: &mut Context,
) -> ockam::Result<()> {
    TestNode::clean().await?;
    let handle = crate::test_utils::start_manager_for_tests(context, None, None).await?;

    let consumer_bootstrap_port = create_kafka_service(
        context,
        &handle,
        "kafka_consumer_listener".into(),
        "kafka_consumer_outlet".into(),
    )
    .await?;

    let producer_bootstrap_port = create_kafka_service(
        context,
        &handle,
        "kafka_producer_listener".into(),
        "kafka_producer_outlet".into(),
    )
    .await?;

    // for the consumer to become available to the producer, the consumer has to issue a Fetch
    // request first, so the sidecar can react by creating the relay for partition
    // 1 of 'my-topic'
    {
        let mut consumer_mock_kafka = TcpServerSimulator::start("127.0.0.1:0").await;
        handle.tcp.create_outlet(
            "kafka_consumer_outlet",
            HostnamePort::new("127.0.0.1", consumer_mock_kafka.port)?,
            TcpOutletOptions::new(),
        )?;

        simulate_first_kafka_consumer_empty_reply_and_ignore_result(
            consumer_bootstrap_port,
            &mut consumer_mock_kafka,
        )
        .await;
        drop(consumer_mock_kafka);
        // drop the outlet and re-create it when we need it later
        context.stop_address(&"kafka_consumer_outlet".into())?;
    }

    let mut producer_mock_kafka = TcpServerSimulator::start("127.0.0.1:0").await;
    handle.tcp.create_outlet(
        "kafka_producer_outlet",
        HostnamePort::new("127.0.0.1", producer_mock_kafka.port)?,
        TcpOutletOptions::new(),
    )?;
    let request =
        simulate_kafka_producer_and_read_request(producer_bootstrap_port, &mut producer_mock_kafka)
            .await;

    let encrypted_body = request
        .topic_data
        .first()
        .unwrap()
        .partition_data
        .first()
        .unwrap()
        .records
        .as_ref()
        .unwrap();

    let mut encrypted_body = BytesMut::from(encrypted_body.as_ref());
    let records = RecordBatchDecoder::decode(
        &mut encrypted_body,
        None::<fn(&mut Bytes, Compression) -> Result<BytesMut, _>>,
    )
    .unwrap();

    // verify the message has been encrypted
    assert_ne!(
        records.first().unwrap().value.as_ref().unwrap(),
        "hello world!".as_bytes()
    );

    let mut consumer_mock_kafka = TcpServerSimulator::start("127.0.0.1:0").await;
    handle.tcp.create_outlet(
        "kafka_consumer_outlet",
        HostnamePort::new("127.0.0.1", consumer_mock_kafka.port)?,
        TcpOutletOptions::new(),
    )?;

    // give the secure channel between producer and consumer to finish initialization
    tokio::time::sleep(Duration::from_secs(2)).await;

    let plain_fetch_response = simulate_kafka_consumer_and_read_response(
        consumer_bootstrap_port,
        &mut consumer_mock_kafka,
        &request,
    )
    .await;

    let plain_content = plain_fetch_response
        .responses
        .first()
        .as_ref()
        .unwrap()
        .partitions
        .first()
        .as_ref()
        .unwrap()
        .records
        .as_ref()
        .unwrap();

    let mut plain_content = BytesMut::from(plain_content.as_ref());
    let records = RecordBatchDecoder::decode(
        &mut plain_content,
        None::<fn(&mut Bytes, Compression) -> Result<BytesMut, _>>,
    )
    .unwrap();

    assert_eq!(
        records.first().as_ref().unwrap().value.as_ref().unwrap(),
        "hello world!".as_bytes()
    );

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
    let header = RequestHeader::default()
        .with_request_api_key(ApiKey::ProduceKey as i16)
        .with_request_api_version(TEST_KAFKA_API_VERSION)
        .with_correlation_id(1)
        .with_client_id(Some(StrBytes::from_static_str("my-client-id")));

    let mut encoded = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut encoded,
        [Record {
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
        None::<fn(&mut BytesMut, &mut BytesMut, Compression) -> Result<(), _>>,
    )
    .unwrap();

    let topic_data = vec![TopicProduceData::default()
        .with_name(TopicName::from(StrBytes::from_static_str("my-topic-name")))
        .with_partition_data(vec![PartitionProduceData::default()
            .with_index(1)
            .with_records(Some(encoded.freeze()))])];
    let request = ProduceRequest::default().with_topic_data(topic_data);

    send_kafka_request(stream, header, request, ApiKey::ProduceKey).await;
}

// this is needed in order to make the consumer create the relays to the secure
// channel
async fn simulate_first_kafka_consumer_empty_reply_and_ignore_result(
    consumer_bootstrap_port: u16,
    mock_kafka_connection: &mut TcpServerSimulator,
) {
    let mut kafka_client_connection =
        TcpStream::connect(format!("127.0.0.1:{consumer_bootstrap_port}"))
            .await
            .unwrap();
    send_kafka_fetch_request(&mut kafka_client_connection).await;
    // we don't want the answer, but we need to be sure the
    // message passed through and the relay had been created
    mock_kafka_connection
        .stream
        .read_exact(&mut [0; 4])
        .await
        .unwrap();
}

// we use the encrypted producer request to generate the encrypted fetch response
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
    let _fetch_request: FetchRequest = read_kafka_request::<
        &mut DuplexStream,
        RequestHeader,
        FetchRequest,
    >(mock_kafka_connection.stream(), ApiKey::FetchKey)
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
    let producer_content = producer_request
        .topic_data
        .first()
        .unwrap()
        .partition_data
        .first()
        .unwrap()
        .records
        .clone();

    let topic_name = TopicName::from(StrBytes::from_static_str("my-topic-name"));
    send_kafka_response(
        stream,
        ResponseHeader::default().with_correlation_id(1),
        FetchResponse::default().with_responses(vec![FetchableTopicResponse::default()
            .with_topic(topic_name)
            .with_partitions(vec![PartitionData::default()
                .with_partition_index(1)
                .with_records(producer_content)])]),
        ApiKey::FetchKey,
    )
    .await;
}

async fn send_kafka_fetch_request(stream: &mut TcpStream) {
    send_kafka_request(
        stream,
        RequestHeader::default()
            .with_request_api_key(ApiKey::FetchKey as i16)
            .with_request_api_version(TEST_KAFKA_API_VERSION)
            .with_correlation_id(1)
            .with_client_id(Some(StrBytes::from_static_str("my-client-id"))),
        FetchRequest::default().with_topics(vec![FetchTopic::default()
            .with_topic(TopicName::from(StrBytes::from_static_str("my-topic-name")))
            .with_topic_id(Uuid::from_slice(b"my-topic-name___").unwrap())
            .with_partitions(vec![FetchPartition::default().with_partition(1)])]),
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
        // we want to close the channel _before_ joining current handles to interrupt them
        drop(self.stream);
        let mut guard = self.join_handles.lock().await;
        for handle in guard.iter_mut() {
            // we don't care about failures
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
        let (simulator_read_half, simulator_write_half) = tokio::io::split(simulator_side_duplex);

        let handle: JoinHandle<()> = {
            let is_stopping = is_stopping.clone();
            let join_handles = join_handles.clone();
            tokio::spawn(async move {
                let socket;
                loop {
                    // tokio would block on the listener forever, we need to poll a little in
                    // order to interrupt it
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
                let handle: JoinHandle<()> = {
                    let is_stopping = is_stopping.clone();
                    tokio::spawn(async move {
                        Self::relay_traffic(
                            is_stopping,
                            "socket_read_half",
                            socket_read_half,
                            "simulator_write_half",
                            simulator_write_half,
                        )
                        .await
                    })
                };
                join_handles.lock().await.push(handle);

                let handle: JoinHandle<()> = {
                    let is_stopping = is_stopping.clone();
                    tokio::spawn(async move {
                        Self::relay_traffic(
                            is_stopping,
                            "simulator_read_half",
                            simulator_read_half,
                            "socket_write_half",
                            socket_write_half,
                        )
                        .await
                    })
                };
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
        is_stopping: Arc<AtomicBool>,
        read_half_name: &'static str,
        mut read_half: R,
        write_half_name: &'static str,
        mut write_half: W,
    ) {
        let mut buffer = [0; 1024];
        while !is_stopping.load(Ordering::Relaxed) {
            let timeout_future =
                tokio::time::timeout(Duration::from_secs(1), read_half.read(&mut buffer));

            let result = match timeout_future.await {
                Err(_) => {
                    continue;
                }
                Ok(result) => result,
            };
            let read = match result {
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
