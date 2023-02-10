use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use ockam_core::compat::sync::Arc;
use ockam_core::{
    errcode::{Kind, Origin},
    Address, AllowAll, Encodable, Error, LocalInfo, LocalMessage, Route, Routed, TransportMessage,
    Worker,
};
use ockam_node::Context;
use ockam_transport_tcp::{PortalMessage, MAX_PAYLOAD_SIZE};

use crate::kafka::decoder::KafkaDecoder;
use crate::kafka::encoder::KafkaEncoder;
use crate::kafka::inlet_map::KafkaInletMap;
use crate::kafka::protocol_aware::ProtocolState;

///by default kafka supports up to 1MB messages, 16MB is the maximum suggested
pub(crate) const MAX_KAFKA_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

enum Receiving {
    Requests,
    Responses,
}

///Acts like a relay for messages between tcp inlet and outlet for both directions.
/// It's meant to be created by the portal listener.
///
/// This instance manage both streams inlet and outlet in two different workers, one dedicated
/// to the requests (inlet=>outlet) the other for the responses (outlet=>inlet).
/// since every kafka message is length-delimited every message is read and written
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
    reader: KafkaDecoder,
    writer: KafkaEncoder,
    receiving: Receiving,
    shared_protocol_state: ProtocolState,
    inlet_map: KafkaInletMap,
    disconnect_received: Arc<AtomicBool>,
}

#[ockam::worker]
impl Worker for KafkaPortalWorker {
    type Message = PortalMessage;
    type Context = Context;

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
        //to correctly proxy messages to the inlet or outlet side
        //we invert the return route when a message pass through
        let mut local_message = routed_message.into_local_message();
        let transport = local_message.transport_mut();
        transport
            .return_route
            .modify()
            .prepend(self.other_worker_address.clone());

        transport.onward_route.step()?;

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
        buffer: Vec<u8>,
        local_info: &[LocalInfo],
    ) -> ockam_core::Result<()> {
        for chunk in buffer.chunks(MAX_PAYLOAD_SIZE) {
            //to correctly proxy messages to the inlet or outlet side
            //we invert the return route when a message pass through
            let message = LocalMessage::new(
                TransportMessage::v1(
                    onward_route.clone().modify().pop_front(),
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

    async fn intercept_and_transform_messages(
        &mut self,
        context: &mut Context,
        encoded_message: &Vec<u8>,
    ) -> Result<Option<Vec<u8>>, InterceptError> {
        self.reader
            .write_length_encoded(encoded_message)
            .await
            .map_err(InterceptError::Io)?;

        loop {
            let maybe_result = self.reader.read_kafka_message().await;
            if let Some(result) = maybe_result {
                let complete_kafka_message = result.map_err(InterceptError::Io)?;
                let transformed_message = match self.receiving {
                    Receiving::Requests => self
                        .shared_protocol_state
                        .intercept_request(complete_kafka_message),
                    Receiving::Responses => {
                        self.shared_protocol_state
                            .intercept_response(context, complete_kafka_message, &self.inlet_map)
                            .await
                    }
                }?;

                trace!("transformed_message: {:?}", transformed_message.len());
                self.writer
                    .write_kafka_message(transformed_message.to_vec())
                    .await
                    .map_err(InterceptError::Io)?;
            } else {
                break;
            }
        }

        let length_encoded_buffer = self
            .writer
            .read_length_encoded()
            .await
            .map_err(InterceptError::Io)?;
        trace!("length_encoded_buffer: {:?}", length_encoded_buffer.len());
        Ok(Some(length_encoded_buffer))
    }
}

impl KafkaPortalWorker {
    ///returns address used for inlet communications, aka the one facing the client side,
    /// used for requests.
    pub(crate) async fn start(
        context: &mut Context,
        inlet_map: KafkaInletMap,
    ) -> ockam_core::Result<Address> {
        let shared_protocol_state = ProtocolState::new();

        let inlet_address = Address::random_tagged("KafkaPortalWorker.inlet");
        let outlet_address = Address::random_tagged("KafkaPortalWorker.outlet");
        let disconnect_received = Arc::new(AtomicBool::new(false));

        let inlet_worker = Self {
            inlet_map: inlet_map.clone(),
            shared_protocol_state: shared_protocol_state.clone(),
            other_worker_address: outlet_address.clone(),
            reader: KafkaDecoder::new(),
            writer: KafkaEncoder::new(),
            receiving: Receiving::Requests,
            disconnect_received: disconnect_received.clone(),
        };
        let outlet_worker = Self {
            inlet_map: inlet_map.clone(),
            shared_protocol_state,
            other_worker_address: inlet_address.clone(),
            reader: KafkaDecoder::new(),
            writer: KafkaEncoder::new(),
            receiving: Receiving::Responses,
            disconnect_received: disconnect_received.clone(),
        };

        context
            .start_worker(inlet_address.clone(), inlet_worker, AllowAll, AllowAll)
            .await?;

        context
            .start_worker(outlet_address, outlet_worker, AllowAll, AllowAll)
            .await?;

        Ok(inlet_address)
    }
}

#[cfg(test)]
mod test {
    use bytes::{Buf, BufMut, BytesMut};
    use kafka_protocol::messages::metadata_response::MetadataResponseBroker;
    use kafka_protocol::messages::{
        ApiKey, BrokerId, MetadataRequest, MetadataResponse, RequestHeader, ResponseHeader,
    };
    use kafka_protocol::protocol::Decodable;
    use kafka_protocol::protocol::Encodable as KafkaEncodable;
    use kafka_protocol::protocol::StrBytes;

    use ockam::Context;
    use ockam_core::compat::sync::Arc;
    use ockam_core::{route, AsyncTryClone, Routed};
    use ockam_transport_tcp::{PortalMessage, TcpTransport};

    use crate::kafka::inlet_map::KafkaInletMap;
    use crate::kafka::portal_worker::KafkaPortalWorker;
    use crate::port_range::PortRange;
    use ockam::compat::asynchronous::RwLock;

    const TEST_KAFKA_API_VERSION: i16 = 13;

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5000)]
    async fn kafka_portal_worker__ping_pong_pass_through__should_pass(
        context: &mut Context,
    ) -> ockam::Result<()> {
        let tcp_transport = TcpTransport::create(context).await?;

        let inlet_map = KafkaInletMap::new(
            Arc::new(tcp_transport),
            route![],
            "0.0.0.0".into(),
            PortRange::new(20_000, 40_000).unwrap(),
        );
        let portal_inlet_address = KafkaPortalWorker::start(context, inlet_map).await?;

        context
            .send(
                route![portal_inlet_address, context.address()],
                PortalMessage::Ping,
            )
            .await?;

        let message: Routed<PortalMessage> = context.receive::<PortalMessage>().await?.take();
        if let PortalMessage::Ping = message.as_body() {
        } else {
            assert!(false, "invalid message type")
        }

        context
            .send(message.return_route(), PortalMessage::Pong)
            .await?;

        let message: Routed<PortalMessage> = context.receive::<PortalMessage>().await?.take();
        if let PortalMessage::Pong = message.as_body() {
        } else {
            assert!(false, "invalid message type")
        }

        context.stop().await
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5000)]
    async fn kafka_portal_worker__metadata_exchange__response_changed(
        context: &mut Context,
    ) -> ockam::Result<()> {
        let handle = crate::test::start_manager_for_tests(context).await?;

        let inlet_map = KafkaInletMap::new(
            handle.tcp.create_inlet_controller().await?,
            route![],
            "127.0.0.1".into(),
            PortRange::new(20_000, 40_000).unwrap(),
        );
        let portal_inlet_address = KafkaPortalWorker::start(context, inlet_map.clone()).await?;

        let mut request_buffer = BytesMut::new();
        {
            //let's create a real kafka request and pass it through the portal
            let request_header = RequestHeader {
                request_api_key: ApiKey::MetadataKey as i16,
                request_api_version: TEST_KAFKA_API_VERSION,
                correlation_id: 1,
                client_id: Some(StrBytes::from_str("my-id")),
                unknown_tagged_fields: Default::default(),
            };
            let metadata_request = MetadataRequest {
                topics: None,
                allow_auto_topic_creation: false,
                include_cluster_authorized_operations: false,
                include_topic_authorized_operations: false,
                unknown_tagged_fields: Default::default(),
            };

            let size = request_header.compute_size(TEST_KAFKA_API_VERSION).unwrap()
                + metadata_request
                    .compute_size(TEST_KAFKA_API_VERSION)
                    .unwrap();
            request_buffer.put_u32(size as u32);

            request_header
                .encode(&mut request_buffer, TEST_KAFKA_API_VERSION)
                .unwrap();
            metadata_request
                .encode(&mut request_buffer, TEST_KAFKA_API_VERSION)
                .unwrap();
            assert_eq!(size + 4, request_buffer.len());
        }

        context
            .send(
                route![portal_inlet_address, context.address()],
                PortalMessage::Payload(request_buffer.to_vec()),
            )
            .await?;

        let message: Routed<PortalMessage> = context.receive_block::<PortalMessage>().await?.take();

        if let PortalMessage::Payload(payload) = message.as_body() {
            assert_eq!(&request_buffer.to_vec(), payload);
        } else {
            assert!(false, "invalid message type")
        }
        trace!("return_route: {:?}", &message.return_route());

        let mut response_buffer = BytesMut::new();
        {
            let response_header = ResponseHeader {
                correlation_id: 1,
                unknown_tagged_fields: Default::default(),
            };

            let metadata_response = MetadataResponse {
                throttle_time_ms: 0,
                brokers: indexmap::IndexMap::from_iter(vec![(
                    BrokerId(1),
                    MetadataResponseBroker {
                        host: StrBytes::from_str("bad.remote.host.example.com"),
                        port: 1234,
                        rack: Some(Default::default()),
                        unknown_tagged_fields: Default::default(),
                    },
                )]),
                cluster_id: Some(StrBytes::from_str("7rbGj9JNQwm_qlW3pQ2YRw")),
                controller_id: BrokerId::from(1),
                topics: Default::default(),
                cluster_authorized_operations: -2147483648,
                unknown_tagged_fields: Default::default(),
            };
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

        let message: Routed<PortalMessage> = context.receive_block::<PortalMessage>().await?.take();

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
            assert_eq!(20_000, broker.port);

            let address = inlet_map.retrieve_inlet(1).await.expect("inlet not found");
            assert_eq!("127.0.0.1".to_string(), address.ip().to_string());
            assert_eq!(20_000, address.port());
        } else {
            assert!(false, "invalid message type")
        }

        context.stop().await
    }

    //TODO: request smaller than kafka message
    //TODO: request with 2 kafka messages
    //TODO: request bigger than max limit
    //TODO: request as big as max limit-1 + another chunk
}
