mod peer_pending_routing_message_storage;
mod pending_message;
mod pending_routing_message_storage;
mod transport_messages_iterator;

pub(crate) use peer_pending_routing_message_storage::*;
pub(crate) use pending_message::*;
pub(crate) use pending_routing_message_storage::*;
pub(crate) use transport_messages_iterator::*;

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use crate::messages::{
        RoutingNumber, UdpRoutingMessage, UdpTransportMessage, MAX_PAYLOAD_SIZE,
    };
    use crate::workers::pending_messages::{PendingMessage, TransportMessagesIterator};
    use crate::MAX_MESSAGE_SIZE;
    use ockam_core::compat::rand::RngCore;
    use ockam_core::{route, Result};
    use rand::prelude::SliceRandom;
    use rand::thread_rng;

    #[test]
    fn small_message__reassemble__should_succeed() -> Result<()> {
        let message = UdpRoutingMessage::new(
            route!["onward"],
            route!["return"],
            "Hello, Ockam!".as_bytes().into(),
            None,
        );

        let routing_number = RoutingNumber::default();

        let mut iterator = TransportMessagesIterator::new(routing_number, &message)?;

        let next = iterator.next().transpose()?.unwrap();
        let packet: UdpTransportMessage = minicbor::decode(&next)?;
        let mut pending_message = PendingMessage::new(vec![]);
        assert!(pending_message.try_assemble().is_none());
        pending_message.add_transport_message(packet)?;
        let message_received = pending_message.try_assemble().unwrap();
        let message_received: UdpRoutingMessage = minicbor::decode(&message_received)?;

        assert_eq!(message_received, message);

        Ok(())
    }

    #[test]
    fn medium_message__reassemble__should_succeed() -> Result<()> {
        let mut payload = vec![0; MAX_PAYLOAD_SIZE];
        thread_rng().fill_bytes(&mut payload);

        let message =
            UdpRoutingMessage::new(route!["onward"], route!["return"], payload.into(), None);

        let routing_number = RoutingNumber::default();

        let mut pending_message = PendingMessage::new(vec![]);

        let mut iterator = TransportMessagesIterator::new(routing_number, &message)?;

        let next = iterator.next().transpose()?.unwrap();
        let packet: UdpTransportMessage = minicbor::decode(&next)?;
        pending_message.add_transport_message(packet)?;
        assert!(pending_message.try_assemble().is_none());

        let next = iterator.next().transpose()?.unwrap();
        let packet: UdpTransportMessage = minicbor::decode(&next)?;
        pending_message.add_transport_message(packet)?;

        let message_received = pending_message.try_assemble().unwrap();
        let message_received: UdpRoutingMessage = minicbor::decode(&message_received)?;

        assert_eq!(message_received, message);

        Ok(())
    }

    #[test]
    fn large_message__reassemble__should_succeed() -> Result<()> {
        let mut payload = vec![0; MAX_MESSAGE_SIZE / 2];
        thread_rng().fill_bytes(&mut payload);

        let message =
            UdpRoutingMessage::new(route!["onward"], route!["return"], payload.into(), None);

        let routing_number = RoutingNumber::default();

        let mut pending_message = PendingMessage::new(vec![]);

        let mut iterator = TransportMessagesIterator::new(routing_number, &message)?;

        while let Some(next) = iterator.next().transpose()? {
            let packet: UdpTransportMessage = minicbor::decode(&next)?;
            assert!(pending_message.try_assemble().is_none());
            pending_message.add_transport_message(packet)?;
        }

        let message_received = pending_message.try_assemble().unwrap();
        let message_received: UdpRoutingMessage = minicbor::decode(&message_received)?;

        assert_eq!(message_received, message);

        Ok(())
    }

    impl<'a> UdpTransportMessage<'a> {
        pub fn into_owned(self) -> UdpTransportMessage<'static> {
            UdpTransportMessage {
                version: self.version,
                routing_number: self.routing_number,
                offset: self.offset,
                total: self.total,
                payload: self.payload.into_owned().into(),
            }
        }
    }

    #[test]
    fn large_message__reassemble__should_succeed2() -> Result<()> {
        let mut payload = vec![0; MAX_MESSAGE_SIZE / 2];
        thread_rng().fill_bytes(&mut payload);

        let message =
            UdpRoutingMessage::new(route!["onward"], route!["return"], payload.into(), None);

        let routing_number = RoutingNumber::default();

        let mut pending_message = PendingMessage::new(vec![]);

        let mut iterator = TransportMessagesIterator::new(routing_number, &message)?;

        let mut packets = vec![];

        while let Some(next) = iterator.next().transpose()? {
            let packet: UdpTransportMessage = minicbor::decode(&next)?;
            packets.push(packet.into_owned());
        }

        packets.shuffle(&mut thread_rng());

        for packet in packets {
            assert!(pending_message.try_assemble().is_none());
            pending_message.add_transport_message(packet)?;
        }

        let message_received = pending_message.try_assemble().unwrap();
        let message_received: UdpRoutingMessage = minicbor::decode(&message_received)?;

        assert_eq!(message_received, message);

        Ok(())
    }
}
