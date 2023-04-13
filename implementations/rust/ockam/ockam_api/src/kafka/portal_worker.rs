use bytes::{Bytes, BytesMut};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{FlowControlId, FlowControlPolicy};
use ockam_core::{
    errcode::{Kind, Origin},
    Address, AllowAll, AsyncTryClone, Encodable, Error, LocalInfo, LocalMessage, Route, Routed,
    TransportMessage, Worker,
};
use ockam_node::Context;
use ockam_transport_tcp::{PortalMessage, MAX_PAYLOAD_SIZE};

use crate::kafka::inlet_controller::KafkaInletController;
use crate::kafka::length_delimited::{length_encode, KafkaMessageDecoder};
use crate::kafka::protocol_aware::{Interceptor, TopicUuidMap};
use crate::kafka::secure_channel_map::KafkaSecureChannelController;

///by default kafka supports up to 1MB messages, 16MB is the maximum suggested
pub(crate) const MAX_KAFKA_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;

enum Receiving {
    Requests,

    // When we are receiving responses we have the responsibility of validating
    // next step in onward route.
    // Since we know it beforehand we simply ignore the provided onward route
    // and use the one we know.
    Responses(Route),
}

/// Acts like a relay for messages between tcp inlet and outlet for both directions.
/// It's meant to be created by the portal listener.
///
/// This implementation manage both streams inlet and outlet in two different workers, one dedicated
/// to the requests (inlet=>outlet) the other for the responses (outlet=>inlet).
/// Since every kafka message is length-delimited every message is read and written
/// through a framed encoder/decoder.
///
/// ```text
/// ┌────────┐  decoder    ┌─────────┐  encoder    ┌────────┐
/// │        ├────────────►│ Kafka   ├────────────►│        │
/// │        │             │ Request │             │        │
/// │  TCP   │             └─────────┘             │  TCP   │
/// │ Inlet  │             ┌─────────┐             │ Outlet │
/// │        │  encoder    │ Kafka   │   decoder   │        │
/// │        │◄────────────┤ Response│◄────────────┤        │
/// └────────┘             └─────────┘             └────────┘
///```
pub(crate) struct KafkaPortalWorker {
    //the instance of worker managing the opposite: request or response
    //the first one to receive the disconnect message will stop both workers
    other_worker_address: Address,
    receiving: Receiving,
    shared_protocol_state: Interceptor,
    inlet_map: KafkaInletController,
    disconnect_received: Arc<AtomicBool>,
    decoder: KafkaMessageDecoder,
    max_message_size: u32,
}

#[ockam::worker]
impl Worker for KafkaPortalWorker {
    type Message = PortalMessage;
    type Context = Context;

    //Every tcp payload message is received gets written into a buffer
    // when the whole kafka message is received the message is intercepted
    // and then forwarded to the original destination.
    //As it may take several tcp payload messages to complete a single kafka
    // message or a single message may contain several kafka messages within
    // there is no guaranteed relation between message incoming and messages
    // outgoing.
    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        routed_message: Routed<Self::Message>,
    ) -> ockam::Result<()> {
        let onward_route = routed_message.onward_route();
        let return_route = routed_message.return_route();
        let local_info = routed_message.local_message().local_info().to_vec();
        let portal_message = routed_message.as_body();

        match portal_message {
            PortalMessage::Payload(message) => {
                let result = self
                    .intercept_and_transform_messages(context, message)
                    .await;

                match result {
                    Ok(maybe_kafka_message) => {
                        if let Some(encoded_message) = maybe_kafka_message {
                            self.split_and_send(
                                context,
                                onward_route,
                                return_route,
                                encoded_message,
                                local_info.as_slice(),
                            )
                            .await?;
                        }
                    }
                    Err(cause) => {
                        trace!("error: {cause:?}");
                        return match cause {
                            InterceptError::Io(cause) => {
                                Err(Error::new(Origin::Transport, Kind::Io, cause))
                            }
                            InterceptError::Ockam(error) => Err(error),
                        };
                    }
                }
            }
            PortalMessage::Disconnect => {
                self.forward(context, routed_message).await?;

                //the first one to receive disconnect and to swap the atomic will
                //stop both workers
                let disconnect_received = self.disconnect_received.swap(true, Ordering::SeqCst);
                if !disconnect_received {
                    trace!(
                        "{:?} received disconnect event from {:?}",
                        context.address(),
                        return_route
                    );
                    context
                        .stop_worker(self.other_worker_address.clone())
                        .await?;
                    context.stop_worker(context.address()).await?;
                }
            }
            PortalMessage::Ping | PortalMessage::Pong => {
                self.forward(context, routed_message).await?
            }
        }

        Ok(())
    }
}

//internal error to return both io and ockam errors
#[derive(Debug)]
pub(crate) enum InterceptError {
    Io(ockam_core::compat::io::Error),
    Ockam(ockam_core::Error),
}

impl KafkaPortalWorker {
    async fn forward(
        &self,
        context: &mut Context,
        routed_message: Routed<PortalMessage>,
    ) -> ockam_core::Result<()> {
        trace!(
            "before: onwards={:?}; return={:?};",
            routed_message.local_message().transport().onward_route,
            routed_message.local_message().transport().return_route
        );
        let mut local_message = routed_message.into_local_message();
        let transport = local_message.transport_mut();

        //to correctly proxy messages to the inlet or outlet side
        //we invert the return route when a message pass through
        transport
            .return_route
            .modify()
            .prepend(self.other_worker_address.clone());

        if let Receiving::Responses(fixed_onward_route) = &self.receiving {
            transport.onward_route = fixed_onward_route.clone();
        } else {
            transport.onward_route.step()?;
        }

        trace!(
            "after: onwards={:?}; return={:?};",
            local_message.transport().onward_route,
            local_message.transport().return_route
        );
        context.forward(local_message).await
    }

    async fn split_and_send(
        &self,
        context: &mut Context,
        onward_route: Route,
        return_route: Route,
        buffer: Bytes,
        local_info: &[LocalInfo],
    ) -> ockam_core::Result<()> {
        let onward_route = if let Receiving::Responses(fixed_onward_route) = &self.receiving {
            fixed_onward_route.clone()
        } else {
            onward_route.clone().modify().pop_front().into()
        };

        for chunk in buffer.chunks(MAX_PAYLOAD_SIZE) {
            //to correctly proxy messages to the inlet or outlet side
            //we invert the return route when a message pass through
            let message = LocalMessage::new(
                TransportMessage::v1(
                    onward_route.clone(),
                    return_route
                        .clone()
                        .modify()
                        .prepend(self.other_worker_address.clone()),
                    PortalMessage::Payload(chunk.to_vec()).encode()?,
                ),
                local_info.to_vec(),
            );

            context.forward(message).await?;
        }
        Ok(())
    }

    ///Takes in buffer and returns a buffer made of one or more complete kafka message
    async fn intercept_and_transform_messages(
        &mut self,
        context: &mut Context,
        encoded_message: &Vec<u8>,
    ) -> Result<Option<Bytes>, InterceptError> {
        let mut encoded_buffer: Option<BytesMut> = None;

        for complete_kafka_message in self
            .decoder
            .decode_messages(
                BytesMut::from(encoded_message.as_slice()),
                self.max_message_size,
            )
            .map_err(InterceptError::Ockam)?
        {
            let transformed_message = match self.receiving {
                Receiving::Requests => {
                    self.shared_protocol_state
                        .intercept_request(context, complete_kafka_message)
                        .await
                }
                Receiving::Responses(_) => {
                    self.shared_protocol_state
                        .intercept_response(context, complete_kafka_message, &self.inlet_map)
                        .await
                }
            }?;

            //avoid copying the first message
            if let Some(encoded_buffer) = encoded_buffer.as_mut() {
                encoded_buffer.extend_from_slice(
                    length_encode(transformed_message)
                        .map_err(InterceptError::Ockam)?
                        .as_ref(),
                );
            } else {
                encoded_buffer =
                    Some(length_encode(transformed_message).map_err(InterceptError::Ockam)?);
            }
        }

        Ok(encoded_buffer.map(|buffer| buffer.freeze()))
    }
}

impl KafkaPortalWorker {
    ///returns address used for inlet communications, aka the one facing the client side,
    /// used for requests.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn start_kafka_portal(
        context: &mut Context,
        secure_channel_controller: Arc<dyn KafkaSecureChannelController>,
        uuid_to_name: TopicUuidMap,
        inlet_map: KafkaInletController,
        max_kafka_message_size: Option<u32>,
        flow_control_id: Option<FlowControlId>,
        inlet_responder_route: Route,
    ) -> ockam_core::Result<Address> {
        let shared_protocol_state = Interceptor::new(secure_channel_controller, uuid_to_name);

        let requests_worker_address = Address::random_tagged("KafkaPortalWorker.requests");
        let responses_worker_address = Address::random_tagged("KafkaPortalWorker.responses");
        let disconnect_received = Arc::new(AtomicBool::new(false));

        let request_worker = Self {
            inlet_map: inlet_map.clone(),
            shared_protocol_state: shared_protocol_state.async_try_clone().await?,
            other_worker_address: responses_worker_address.clone(),
            receiving: Receiving::Requests,
            disconnect_received: disconnect_received.clone(),
            decoder: KafkaMessageDecoder::new(),
            max_message_size: max_kafka_message_size.unwrap_or(MAX_KAFKA_MESSAGE_SIZE),
        };
        let response_worker = Self {
            inlet_map: inlet_map.clone(),
            shared_protocol_state,
            other_worker_address: requests_worker_address.clone(),
            receiving: Receiving::Responses(inlet_responder_route),
            disconnect_received: disconnect_received.clone(),
            decoder: KafkaMessageDecoder::new(),
            max_message_size: max_kafka_message_size.unwrap_or(MAX_KAFKA_MESSAGE_SIZE),
        };

        context
            .start_worker(
                requests_worker_address.clone(),
                request_worker,
                AllowAll,
                AllowAll,
            )
            .await?;

        if let Some(flow_control_id) = flow_control_id {
            context.flow_controls().add_consumer(
                responses_worker_address.clone(),
                &flow_control_id,
                FlowControlPolicy::ProducerAllowMultiple,
            );
        }
        context
            .start_worker(
                responses_worker_address,
                response_worker,
                AllowAll,
                AllowAll,
            )
            .await?;

        Ok(requests_worker_address)
    }
}

#[cfg(test)]
mod test {
    use bytes::{Buf, BufMut, BytesMut};
    use kafka_protocol::messages::metadata_request::MetadataRequestBuilder;
    use kafka_protocol::messages::metadata_response::MetadataResponseBroker;
    use kafka_protocol::messages::{
        ApiKey, BrokerId, MetadataRequest, MetadataResponse, RequestHeader, ResponseHeader,
    };
    use kafka_protocol::protocol::Builder;
    use kafka_protocol::protocol::Decodable;
    use kafka_protocol::protocol::Encodable as KafkaEncodable;
    use kafka_protocol::protocol::StrBytes;
    use ockam::identity::secure_channels;
    use ockam_core::compat::sync::{Arc, Mutex};
    use ockam_core::{route, Address, AllowAll, Routed, Worker};
    use ockam_multiaddr::MultiAddr;
    use ockam_node::Context;
    use ockam_transport_tcp::{PortalMessage, MAX_PAYLOAD_SIZE};
    use std::collections::BTreeMap;
    use std::time::Duration;

    use crate::kafka::inlet_controller::KafkaInletController;
    use crate::kafka::portal_worker::KafkaPortalWorker;
    use crate::kafka::secure_channel_map::KafkaSecureChannelControllerImpl;
    use crate::port_range::PortRange;
    use ockam::MessageReceiveOptions;

    const TEST_MAX_KAFKA_MESSAGE_SIZE: u32 = 128 * 1024;
    const TEST_KAFKA_API_VERSION: i16 = 13;

    //a simple worker that keep receiving buffer
    #[derive(Clone)]
    struct TcpPayloadReceiver {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    #[ockam_core::worker]
    impl Worker for TcpPayloadReceiver {
        type Message = PortalMessage;
        type Context = Context;

        async fn handle_message(
            &mut self,
            _context: &mut Self::Context,
            message: Routed<Self::Message>,
        ) -> ockam_core::Result<()> {
            if let PortalMessage::Payload(payload) = message.as_body() {
                self.buffer.lock().unwrap().extend_from_slice(payload);
            }
            Ok(())
        }
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5_000)]
    async fn kafka_portal_worker__ping_pong_pass_through__should_pass(
        context: &mut Context,
    ) -> ockam::Result<()> {
        let portal_inlet_address = setup_only_worker(context).await;

        context
            .send(
                route![portal_inlet_address, context.address()],
                PortalMessage::Ping,
            )
            .await?;

        let message: Routed<PortalMessage> = context.receive::<PortalMessage>().await?;
        if let PortalMessage::Ping = message.as_body() {
        } else {
            panic!("invalid message type")
        }

        context
            .send(message.return_route(), PortalMessage::Pong)
            .await?;

        let message: Routed<PortalMessage> = context.receive::<PortalMessage>().await?;
        if let PortalMessage::Pong = message.as_body() {
        } else {
            panic!("invalid message type")
        }

        context.stop().await
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5_000)]
    async fn kafka_portal_worker__pieces_of_kafka_message__message_assembled(
        context: &mut Context,
    ) -> ockam::Result<()> {
        let portal_inlet_address = setup_only_worker(context).await;

        let mut request_buffer = BytesMut::new();
        encode(
            &mut request_buffer,
            create_request_header(ApiKey::MetadataKey),
            MetadataRequest::default(),
        );

        let first_piece_of_payload = &request_buffer[0..request_buffer.len() - 1];
        let second_piece_of_payload = &request_buffer[request_buffer.len() - 1..];

        //send 2 distinct pieces and see if the kafka message is re-assembled back
        context
            .send(
                route![portal_inlet_address.clone(), context.address()],
                PortalMessage::Payload(first_piece_of_payload.to_vec()),
            )
            .await?;
        context
            .send(
                route![portal_inlet_address, context.address()],
                PortalMessage::Payload(second_piece_of_payload.to_vec()),
            )
            .await?;

        let message = context.receive::<PortalMessage>().await?;

        if let PortalMessage::Payload(payload) = message.as_body() {
            assert_eq!(payload, request_buffer.as_ref());
        } else {
            panic!("invalid message")
        }

        context.stop().await
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5_000)]
    async fn kafka_portal_worker__double_kafka_message__message_assembled(
        context: &mut Context,
    ) -> ockam::Result<()> {
        let portal_inlet_address = setup_only_worker(context).await;

        let mut request_buffer = BytesMut::new();
        encode(
            &mut request_buffer,
            create_request_header(ApiKey::MetadataKey),
            MetadataRequest::default(),
        );
        encode(
            &mut request_buffer,
            create_request_header(ApiKey::MetadataKey),
            MetadataRequest::default(),
        );

        let double_payload = request_buffer.as_ref();
        context
            .send(
                route![portal_inlet_address.clone(), context.address()],
                PortalMessage::Payload(double_payload.to_vec()),
            )
            .await?;
        let message = context.receive::<PortalMessage>().await?;

        if let PortalMessage::Payload(payload) = message.as_body() {
            assert_eq!(payload, double_payload);
        } else {
            panic!("invalid message")
        }

        context.stop().await
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5_000)]
    async fn kafka_portal_worker__bigger_than_limit_kafka_message__error(
        context: &mut Context,
    ) -> ockam::Result<()> {
        let portal_inlet_address = setup_only_worker(context).await;

        //with the message container it goes well over the max allowed message kafka size
        let mut zero_buffer: Vec<u8> = Vec::new();
        for _n in 0..TEST_MAX_KAFKA_MESSAGE_SIZE + 1 {
            zero_buffer.push(0);
        }

        //you don't want to create a produce request since it would trigger
        //a lot of side effects and we just want to validate the transport
        let mut insanely_huge_tag = BTreeMap::new();
        insanely_huge_tag.insert(0, zero_buffer);

        let mut request_buffer = BytesMut::new();
        encode(
            &mut request_buffer,
            create_request_header(ApiKey::MetadataKey),
            MetadataRequestBuilder::default()
                .topics(Default::default())
                .include_cluster_authorized_operations(Default::default())
                .include_topic_authorized_operations(Default::default())
                .allow_auto_topic_creation(Default::default())
                .unknown_tagged_fields(insanely_huge_tag)
                .build()
                .unwrap(),
        );

        let huge_payload = request_buffer.as_ref();
        for chunk in huge_payload.chunks(MAX_PAYLOAD_SIZE) {
            let _error = context
                .send(
                    route![portal_inlet_address.clone(), context.address()],
                    PortalMessage::Payload(chunk.to_vec()),
                )
                .await;
        }

        let message = context
            .receive_extended::<PortalMessage>(
                MessageReceiveOptions::new().with_timeout(Duration::from_millis(200)),
            )
            .await;

        assert!(message.is_err(), "expected timeout!");
        context.stop().await
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5_000)]
    async fn kafka_portal_worker__almost_over_limit_than_limit_kafka_message__two_kafka_message_pass(
        context: &mut Context,
    ) -> ockam::Result<()> {
        let portal_inlet_address = setup_only_worker(context).await;

        //let's build the message to 90% of max. size
        let mut zero_buffer: Vec<u8> = Vec::new();
        for _n in 0..(TEST_MAX_KAFKA_MESSAGE_SIZE as f64 * 0.9) as usize {
            zero_buffer.push(0);
        }

        //you don't want to create a produce request since it would trigger
        //a lot of side effects and we just want to validate the transport
        let mut insanely_huge_tag = BTreeMap::new();
        insanely_huge_tag.insert(0, zero_buffer);

        let mut huge_outgoing_request = BytesMut::new();
        encode(
            &mut huge_outgoing_request,
            create_request_header(ApiKey::MetadataKey),
            MetadataRequestBuilder::default()
                .topics(Default::default())
                .include_cluster_authorized_operations(Default::default())
                .include_topic_authorized_operations(Default::default())
                .allow_auto_topic_creation(Default::default())
                .unknown_tagged_fields(insanely_huge_tag.clone())
                .build()
                .unwrap(),
        );

        let receiver = TcpPayloadReceiver {
            buffer: Default::default(),
        };

        context
            .start_worker(
                Address::from_string("tcp_payload_receiver"),
                receiver.clone(),
                AllowAll,
                AllowAll,
            )
            .await?;

        // let's duplicate the message
        huge_outgoing_request.extend(huge_outgoing_request.clone());

        for chunk in huge_outgoing_request.as_ref().chunks(MAX_PAYLOAD_SIZE) {
            context
                .send(
                    route![portal_inlet_address.clone(), "tcp_payload_receiver"],
                    PortalMessage::Payload(chunk.to_vec()),
                )
                .await?;
        }

        //make sure every packet was received
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
        context.stop().await
    }

    async fn setup_only_worker(context: &mut Context) -> Address {
        let inlet_map = KafkaInletController::new(
            MultiAddr::default(),
            route![],
            route![],
            [255, 255, 255, 255].into(),
            PortRange::new(0, 0).unwrap(),
        );

        let secure_channels = secure_channels();
        let secure_channel_controller =
            KafkaSecureChannelControllerImpl::new(secure_channels, MultiAddr::default())
                .into_trait();

        KafkaPortalWorker::start_kafka_portal(
            context,
            secure_channel_controller,
            Default::default(),
            inlet_map,
            Some(TEST_MAX_KAFKA_MESSAGE_SIZE),
            None,
            route![context.address()],
        )
        .await
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
        RequestHeader::builder()
            .request_api_key(api_key as i16)
            .request_api_version(TEST_KAFKA_API_VERSION)
            .correlation_id(1)
            .client_id(Some(StrBytes::from_str("my-client-id")))
            .unknown_tagged_fields(Default::default())
            .build()
            .unwrap()
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5000)]
    async fn kafka_portal_worker__metadata_exchange__response_changed(
        context: &mut Context,
    ) -> ockam::Result<()> {
        let handler = crate::test::start_manager_for_tests(context).await?;

        let secure_channel_controller = KafkaSecureChannelControllerImpl::new(
            handler.secure_channels.clone(),
            MultiAddr::default(),
        )
        .into_trait();

        let inlet_map = KafkaInletController::new(
            MultiAddr::default(),
            route![],
            route![],
            [127, 0, 0, 1].into(),
            PortRange::new(0, 0).unwrap(),
        );
        let portal_inlet_address = KafkaPortalWorker::start_kafka_portal(
            context,
            secure_channel_controller,
            Default::default(),
            inlet_map.clone(),
            None,
            None,
            route![context.address()],
        )
        .await?;

        let mut request_buffer = BytesMut::new();
        //let's create a real kafka request and pass it through the portal
        encode(
            &mut request_buffer,
            create_request_header(ApiKey::MetadataKey),
            MetadataRequest::default(),
        );

        context
            .send(
                route![portal_inlet_address, context.address()],
                PortalMessage::Payload(request_buffer.to_vec()),
            )
            .await?;

        let message: Routed<PortalMessage> = context
            .receive_extended::<PortalMessage>(MessageReceiveOptions::new().without_timeout())
            .await?;

        if let PortalMessage::Payload(payload) = message.as_body() {
            assert_eq!(&request_buffer.to_vec(), payload);
        } else {
            panic!("invalid message type")
        }
        trace!("return_route: {:?}", &message.return_route());

        let mut response_buffer = BytesMut::new();
        {
            let response_header = ResponseHeader::builder()
                .correlation_id(1)
                .unknown_tagged_fields(Default::default())
                .build()
                .unwrap();

            let metadata_response = MetadataResponse::builder()
                .throttle_time_ms(Default::default())
                .cluster_id(Default::default())
                .cluster_authorized_operations(-2147483648)
                .unknown_tagged_fields(Default::default())
                .controller_id(BrokerId::from(1))
                .topics(Default::default())
                .brokers(indexmap::IndexMap::from_iter(vec![(
                    BrokerId(1),
                    MetadataResponseBroker::builder()
                        .host(StrBytes::from_str("bad.remote.host.example.com"))
                        .port(1234)
                        .rack(Default::default())
                        .unknown_tagged_fields(Default::default())
                        .build()
                        .unwrap(),
                )]))
                .build()
                .unwrap();

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
                message.return_route(),
                PortalMessage::Payload(response_buffer.to_vec()),
            )
            .await?;

        let message: Routed<PortalMessage> = context
            .receive_extended::<PortalMessage>(MessageReceiveOptions::new().without_timeout())
            .await?;

        if let PortalMessage::Payload(payload) = message.body() {
            assert_ne!(&response_buffer.to_vec(), &payload);
            let mut buffer_received = BytesMut::from(payload.as_slice());
            let _size = buffer_received.get_u32();
            let header =
                ResponseHeader::decode(&mut buffer_received, TEST_KAFKA_API_VERSION).unwrap();
            assert_eq!(1, header.correlation_id);
            let response =
                MetadataResponse::decode(&mut buffer_received, TEST_KAFKA_API_VERSION).unwrap();
            assert_eq!(1, response.brokers.len());
            let broker = response.brokers.get(&BrokerId::from(1)).unwrap();
            assert_eq!("127.0.0.1", &broker.host.to_string());
            assert_eq!(0, broker.port);

            let address = inlet_map.retrieve_inlet(1).await.expect("inlet not found");
            assert_eq!("127.0.0.1".to_string(), address.ip().to_string());
            assert_eq!(0, address.port());
        } else {
            panic!("invalid message type")
        }

        context.stop().await
    }
}
