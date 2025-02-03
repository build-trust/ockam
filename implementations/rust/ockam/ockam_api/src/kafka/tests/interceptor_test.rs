use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::Duration;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use kafka_protocol::messages::metadata_response::MetadataResponseBroker;
use kafka_protocol::messages::{
    ApiKey, BrokerId, MetadataRequest, MetadataResponse, RequestHeader, ResponseHeader,
};
use kafka_protocol::protocol::Decodable;
use kafka_protocol::protocol::Encodable as KafkaEncodable;
use kafka_protocol::protocol::StrBytes;

use ockam::identity::Identifier;
use ockam::MessageReceiveOptions;
use ockam_abac::{
    Action, Env, Policies, Resource, ResourcePolicySqlxDatabase, ResourceType,
    ResourceTypePolicySqlxDatabase,
};
use ockam_core::compat::sync::{Arc, Mutex};
use ockam_core::{route, Address, AllowAll, NeutralMessage, Routed, Worker};
use ockam_multiaddr::MultiAddr;
use ockam_node::database::SqlxDatabase;
use ockam_node::Context;
use ockam_transport_tcp::{read_portal_payload_length, PortalInterceptorWorker, PortalMessage};

use crate::kafka::inlet_controller::KafkaInletController;
use crate::kafka::key_exchange::controller::KafkaKeyExchangeControllerImpl;
use crate::kafka::protocol_aware::inlet::InletInterceptorImpl;
use crate::kafka::protocol_aware::KafkaMessageInterceptorWrapper;
use crate::kafka::protocol_aware::MAX_KAFKA_MESSAGE_SIZE;
use crate::kafka::{ConsumerPublishing, ConsumerResolution};
use crate::port_range::PortRange;
use crate::test_utils::{NodeManagerHandle, TestNode};

const TEST_MAX_KAFKA_MESSAGE_SIZE: u32 = 128 * 1024;
const TEST_KAFKA_API_VERSION: i16 = 13;

// a simple worker that keep receiving buffer
#[derive(Clone)]
struct TcpPayloadReceiver {
    buffer: Arc<Mutex<Vec<u8>>>,
}

#[ockam_core::worker]
impl Worker for TcpPayloadReceiver {
    type Message = NeutralMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        _context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        let message = PortalMessage::decode(message.payload())?;
        if let PortalMessage::Payload(payload, _) = message {
            self.buffer.lock().unwrap().extend_from_slice(payload);
        }
        Ok(())
    }
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 5_000)]
async fn kafka_portal_worker__pieces_of_kafka_message__message_assembled(
    context: &mut Context,
) -> ockam::Result<()> {
    TestNode::clean().await?;
    let handle = crate::test_utils::start_manager_for_tests(context, None, None).await?;
    let portal_inlet_address = setup_only_worker(context, &handle).await;

    let mut request_buffer = BytesMut::new();
    encode(
        &mut request_buffer,
        create_request_header(ApiKey::Metadata),
        MetadataRequest::default(),
    );

    let first_piece_of_payload = &request_buffer[0..request_buffer.len() - 1];
    let second_piece_of_payload = &request_buffer[request_buffer.len() - 1..];

    // send 2 distinct pieces and see if the kafka message is re-assembled back
    context
        .send(
            route![
                portal_inlet_address.clone(),
                context.primary_address().clone()
            ],
            PortalMessage::Payload(first_piece_of_payload, None).to_neutral_message()?,
        )
        .await?;
    context
        .send(
            route![portal_inlet_address, context.primary_address().clone()],
            PortalMessage::Payload(second_piece_of_payload, None).to_neutral_message()?,
        )
        .await?;

    let payload = context.receive::<NeutralMessage>().await?.into_payload();
    let message = PortalMessage::decode(&payload)?;
    if let PortalMessage::Payload(payload, _) = message {
        assert_eq!(payload, request_buffer.as_ref());
    } else {
        panic!("invalid message")
    }
    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 5_000)]
async fn kafka_portal_worker__double_kafka_message__message_assembled(
    context: &mut Context,
) -> ockam::Result<()> {
    TestNode::clean().await?;
    let handle = crate::test_utils::start_manager_for_tests(context, None, None).await?;
    let portal_inlet_address = setup_only_worker(context, &handle).await;

    let mut request_buffer = BytesMut::new();
    encode(
        &mut request_buffer,
        create_request_header(ApiKey::Metadata),
        MetadataRequest::default(),
    );
    encode(
        &mut request_buffer,
        create_request_header(ApiKey::Metadata),
        MetadataRequest::default(),
    );

    let double_payload = request_buffer.as_ref();
    context
        .send(
            route![
                portal_inlet_address.clone(),
                context.primary_address().clone()
            ],
            PortalMessage::Payload(double_payload, None).to_neutral_message()?,
        )
        .await?;
    let payload = context.receive::<NeutralMessage>().await?.into_payload();
    let message = PortalMessage::decode(&payload)?;
    if let PortalMessage::Payload(payload, _) = message {
        assert_eq!(payload, double_payload);
    } else {
        panic!("invalid message")
    }
    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 5_000)]
async fn kafka_portal_worker__bigger_than_limit_kafka_message__error(
    context: &mut Context,
) -> ockam::Result<()> {
    TestNode::clean().await?;
    let handle = crate::test_utils::start_manager_for_tests(context, None, None).await?;
    let portal_inlet_address = setup_only_worker(context, &handle).await;

    // with the message container it goes well over the max allowed message kafka size
    let mut zero_buffer: Vec<u8> = Vec::new();
    for _n in 0..TEST_MAX_KAFKA_MESSAGE_SIZE + 1 {
        zero_buffer.push(0);
    }

    // you don't want to create a produce request since it would trigger
    // a lot of side effects and we just want to validate the transport
    let mut insanely_huge_tag: BTreeMap<i32, Bytes> = BTreeMap::new();
    insanely_huge_tag.insert(0, zero_buffer.into());

    let mut request_buffer = BytesMut::new();
    encode(
        &mut request_buffer,
        create_request_header(ApiKey::Metadata),
        MetadataRequest::default().with_unknown_tagged_fields(insanely_huge_tag),
    );

    let huge_payload = request_buffer.as_ref();
    for chunk in huge_payload.chunks(read_portal_payload_length()) {
        let _error = context
            .send(
                route![
                    portal_inlet_address.clone(),
                    context.primary_address().clone()
                ],
                PortalMessage::Payload(chunk, None).to_neutral_message()?,
            )
            .await;
    }

    let message = context
        .receive_extended::<NeutralMessage>(
            MessageReceiveOptions::new().with_timeout(Duration::from_millis(200)),
        )
        .await;

    assert!(message.is_err(), "expected timeout!");
    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 5_000)]
async fn kafka_portal_worker__almost_over_limit_than_limit_kafka_message__two_kafka_message_pass(
    context: &mut Context,
) -> ockam::Result<()> {
    TestNode::clean().await?;
    let handle = crate::test_utils::start_manager_for_tests(context, None, None).await?;
    let portal_inlet_address = setup_only_worker(context, &handle).await;

    // let's build the message to 90% of max. size
    let mut zero_buffer: Vec<u8> = Vec::new();
    for _n in 0..(TEST_MAX_KAFKA_MESSAGE_SIZE as f64 * 0.9) as usize {
        zero_buffer.push(0);
    }

    // you don't want to create a produce request since it would trigger
    // a lot of side effects, and we just want to validate the transport
    let mut insanely_huge_tag: BTreeMap<i32, Bytes> = BTreeMap::new();
    insanely_huge_tag.insert(0, zero_buffer.into());

    let mut huge_outgoing_request = BytesMut::new();
    encode(
        &mut huge_outgoing_request,
        create_request_header(ApiKey::Metadata),
        MetadataRequest::default().with_unknown_tagged_fields(insanely_huge_tag.clone()),
    );

    let receiver = TcpPayloadReceiver {
        buffer: Default::default(),
    };

    context.start_worker(
        Address::from_string("tcp_payload_receiver"),
        receiver.clone(),
    )?;

    // let's duplicate the message
    huge_outgoing_request.extend(huge_outgoing_request.clone());

    for chunk in huge_outgoing_request
        .as_ref()
        .chunks(read_portal_payload_length())
    {
        context
            .send(
                route![portal_inlet_address.clone(), "tcp_payload_receiver"],
                PortalMessage::Payload(chunk, None).to_neutral_message()?,
            )
            .await?;
    }

    // make sure every packet was received
    loop {
        if receiver.buffer.lock().unwrap().len() >= huge_outgoing_request.len() {
            break;
        }
        ockam_node::compat::tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let incoming_rebuilt_buffer = receiver.buffer.lock().unwrap().to_vec();

    assert_eq!(incoming_rebuilt_buffer.len(), huge_outgoing_request.len());
    assert_eq!(
        incoming_rebuilt_buffer.as_slice(),
        huge_outgoing_request.as_ref()
    );

    Ok(())
}

async fn setup_only_worker(context: &mut Context, handle: &NodeManagerHandle) -> Address {
    let inlet_map = KafkaInletController::new(
        (*handle.node_manager).clone(),
        MultiAddr::default(),
        route![],
        route![],
        "255.255.255.255".to_string(),
        PortRange::new(0, 0).unwrap(),
        None,
    );

    // Random Identifier, doesn't affect the test
    let authority_identifier =
        Identifier::from_str("I0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .unwrap();
    let secure_channels = handle.secure_channels.clone();

    let database = SqlxDatabase::in_memory("kafka").await.unwrap();
    let policies = Policies::new(
        Arc::new(ResourcePolicySqlxDatabase::new(
            database.clone(),
            "kafka_test",
        )),
        Arc::new(ResourceTypePolicySqlxDatabase::new(
            database.clone(),
            "kafka_test",
        )),
    );

    let consumer_policy_access_control = policies.make_policy_access_control(
        secure_channels.identities().identities_attributes(),
        Resource::new("arbitrary-resource-name", ResourceType::KafkaConsumer),
        Action::HandleMessage,
        Env::new(),
        Some(authority_identifier.clone()),
    );

    let producer_policy_access_control = policies.make_policy_access_control(
        secure_channels.identities().identities_attributes(),
        Resource::new("arbitrary-resource-name", ResourceType::KafkaProducer),
        Action::HandleMessage,
        Env::new(),
        Some(authority_identifier.clone()),
    );

    let secure_channel_controller = KafkaKeyExchangeControllerImpl::new(
        (*handle.node_manager).clone(),
        secure_channels,
        ConsumerResolution::ViaRelay(MultiAddr::default()),
        ConsumerPublishing::None,
        consumer_policy_access_control,
        producer_policy_access_control,
    );

    PortalInterceptorWorker::create_inlet_interceptor(
        context,
        None,
        route![context.primary_address().clone()],
        Arc::new(AllowAll),
        Arc::new(AllowAll),
        Arc::new(KafkaMessageInterceptorWrapper::new(
            Arc::new(InletInterceptorImpl::new(
                Arc::new(secure_channel_controller),
                Default::default(),
                inlet_map,
                true,
                vec![],
            )),
            TEST_MAX_KAFKA_MESSAGE_SIZE,
        )),
        read_portal_payload_length(),
    )
    .unwrap()
}

fn encode<H, R>(mut request_buffer: &mut BytesMut, header: H, request: R)
where
    H: KafkaEncodable,
    R: KafkaEncodable,
{
    let size = header.compute_size(TEST_KAFKA_API_VERSION).unwrap()
        + request.compute_size(TEST_KAFKA_API_VERSION).unwrap();
    request_buffer.put_u32(size as u32);

    header
        .encode(&mut request_buffer, TEST_KAFKA_API_VERSION)
        .unwrap();
    request
        .encode(&mut request_buffer, TEST_KAFKA_API_VERSION)
        .unwrap();
}

fn create_request_header(api_key: ApiKey) -> RequestHeader {
    RequestHeader::default()
        .with_request_api_key(api_key as i16)
        .with_request_api_version(TEST_KAFKA_API_VERSION)
        .with_correlation_id(1)
        .with_client_id(Some(StrBytes::from_static_str("my-client-id")))
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 5000)]
async fn kafka_portal_worker__metadata_exchange__response_changed(
    context: &mut Context,
) -> ockam::Result<()> {
    TestNode::clean().await?;
    let handle = crate::test_utils::start_manager_for_tests(context, None, None).await?;
    let project_authority = handle
        .node_manager
        .node_manager
        .project_authority()
        .unwrap();

    let consumer_policy_access_control = handle
        .node_manager
        .policy_access_control(
            Some(project_authority.clone()),
            Resource::new("arbitrary-resource-name", ResourceType::KafkaConsumer),
            Action::HandleMessage,
            None,
        )
        .await?;

    let producer_policy_access_control = handle
        .node_manager
        .policy_access_control(
            Some(project_authority.clone()),
            Resource::new("arbitrary-resource-name", ResourceType::KafkaProducer),
            Action::HandleMessage,
            None,
        )
        .await?;

    let secure_channel_controller = KafkaKeyExchangeControllerImpl::new(
        (*handle.node_manager).clone(),
        handle.secure_channels.clone(),
        ConsumerResolution::ViaRelay(MultiAddr::default()),
        ConsumerPublishing::None,
        consumer_policy_access_control,
        producer_policy_access_control,
    );

    let inlet_map = KafkaInletController::new(
        (*handle.node_manager).clone(),
        MultiAddr::default(),
        route![],
        route![],
        "127.0.0.1".to_string(),
        PortRange::new(0, 0).unwrap(),
        None,
    );

    let portal_inlet_address = PortalInterceptorWorker::create_inlet_interceptor(
        context,
        None,
        route![context.primary_address().clone()],
        Arc::new(AllowAll),
        Arc::new(AllowAll),
        Arc::new(KafkaMessageInterceptorWrapper::new(
            Arc::new(InletInterceptorImpl::new(
                Arc::new(secure_channel_controller),
                Default::default(),
                inlet_map.clone(),
                true,
                vec![],
            )),
            MAX_KAFKA_MESSAGE_SIZE,
        )),
        read_portal_payload_length(),
    )?;

    let mut request_buffer = BytesMut::new();
    // let's create a real kafka request and pass it through the portal
    encode(
        &mut request_buffer,
        create_request_header(ApiKey::Metadata),
        MetadataRequest::default(),
    );

    context
        .send(
            route![portal_inlet_address, context.primary_address().clone()],
            PortalMessage::Payload(&request_buffer, None).to_neutral_message()?,
        )
        .await?;

    let message = context
        .receive_extended::<NeutralMessage>(MessageReceiveOptions::new().without_timeout())
        .await?
        .into_local_message();
    let return_route = message.return_route;
    let message = PortalMessage::decode(&message.payload)?;

    if let PortalMessage::Payload(payload, _) = message {
        assert_eq!(&request_buffer, payload);
    } else {
        panic!("invalid message type")
    }
    trace!("return_route: {:?}", &return_route);

    let mut response_buffer = BytesMut::new();
    {
        let response_header = ResponseHeader::default().with_correlation_id(1);
        let metadata_response = MetadataResponse::default()
            .with_controller_id(BrokerId::from(1))
            .with_brokers(vec![MetadataResponseBroker::default()
                .with_node_id(BrokerId::from(1))
                .with_host(StrBytes::from_static_str("bad.remote.host.example.com"))
                .with_port(1234)]);

        let size = response_header
            .compute_size(TEST_KAFKA_API_VERSION)
            .unwrap()
            + metadata_response
                .compute_size(TEST_KAFKA_API_VERSION)
                .unwrap();

        response_buffer.put_u32(size as u32);
        response_header
            .encode(&mut response_buffer, TEST_KAFKA_API_VERSION)
            .unwrap();
        metadata_response
            .encode(&mut response_buffer, TEST_KAFKA_API_VERSION)
            .unwrap();
        assert_eq!(size + 4, response_buffer.len());
    }

    context
        .send(
            return_route,
            PortalMessage::Payload(&response_buffer, None).to_neutral_message()?,
        )
        .await?;

    let message = context
        .receive_extended::<NeutralMessage>(MessageReceiveOptions::new().without_timeout())
        .await?;
    let message = PortalMessage::decode(message.payload())?;

    if let PortalMessage::Payload(payload, _) = message {
        assert_ne!(&response_buffer.to_vec(), &payload);
        let mut buffer_received = BytesMut::from(payload);
        let _size = buffer_received.get_u32();
        let header = ResponseHeader::decode(&mut buffer_received, TEST_KAFKA_API_VERSION).unwrap();
        assert_eq!(1, header.correlation_id);
        let response =
            MetadataResponse::decode(&mut buffer_received, TEST_KAFKA_API_VERSION).unwrap();
        assert_eq!(1, response.brokers.len());
        let broker = response.brokers.first().unwrap();
        assert_eq!(1, broker.node_id.0);
        assert_eq!("127.0.0.1", &broker.host.to_string());
        assert_eq!(0, broker.port);

        let address = inlet_map.retrieve_inlet(1).await.expect("inlet not found");
        assert_eq!("127.0.0.1".to_string(), address.hostname());
        assert_eq!(0, address.port());
    } else {
        panic!("invalid message type")
    }
    Ok(())
}
