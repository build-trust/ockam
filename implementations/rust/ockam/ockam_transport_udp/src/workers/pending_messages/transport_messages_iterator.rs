use crate::messages::{
    RoutingNumber, UdpRoutingMessage, UdpTransportMessage, CURRENT_VERSION, MAX_PAYLOAD_SIZE,
};
use crate::MAX_MESSAGE_SIZE;
use ockam_core::Result;
use ockam_transport_core::TransportError;
use tracing::trace;

pub(crate) struct TransportMessagesIterator {
    current_routing_number: RoutingNumber,
    offset: u16,
    total: u16,
    data: Vec<u8>,
}

impl TransportMessagesIterator {
    pub(crate) fn new(
        current_routing_number: RoutingNumber,
        routing_message: &UdpRoutingMessage,
    ) -> Result<Self> {
        let routing_message = ockam_core::cbor_encode_preallocate(routing_message)?;

        if routing_message.len() > MAX_MESSAGE_SIZE {
            return Err(TransportError::MessageLengthExceeded)?;
        }

        let total = routing_message.len() / MAX_PAYLOAD_SIZE + 1;

        let total: u16 = total
            .try_into()
            .map_err(|_| TransportError::MessageLengthExceeded)?;

        Ok(Self {
            current_routing_number,
            offset: 0,
            total,
            data: routing_message,
        })
    }
}

impl Iterator for TransportMessagesIterator {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == self.total {
            return None;
        }

        let data_offset_begin = (self.offset as usize) * MAX_PAYLOAD_SIZE;
        let data_offset_end = if self.offset + 1 == self.total {
            self.data.len()
        } else {
            data_offset_begin + MAX_PAYLOAD_SIZE
        };

        let part = UdpTransportMessage::new(
            CURRENT_VERSION,
            self.current_routing_number,
            self.offset,
            self.total,
            &self.data[data_offset_begin..data_offset_end],
        );

        trace!(
            "Sending Routing Message {}. Offset {}",
            self.current_routing_number,
            part.offset
        );

        match ockam_core::cbor_encode_preallocate(part) {
            Ok(res) => {
                self.offset += 1;
                Some(Ok(res))
            }
            Err(err) => Some(Err(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::messages::{RoutingNumber, UdpRoutingMessage, MAX_PAYLOAD_SIZE};
    use crate::workers::pending_messages::TransportMessagesIterator;
    use crate::MAX_MESSAGE_SIZE;
    use ockam_core::{route, Result};

    #[allow(non_snake_case)]
    #[test]
    fn small_message__create_iterator__should_have_one_part() -> Result<()> {
        let message = UdpRoutingMessage::new(
            route!["onward"],
            route!["return"],
            "Hello, Ockam!".as_bytes().into(),
            None,
        );

        let routing_number = RoutingNumber::default();

        let iterator = TransportMessagesIterator::new(routing_number, &message)?;

        assert_eq!(iterator.current_routing_number, routing_number);
        assert_eq!(iterator.total, 1);
        assert_eq!(iterator.offset, 0);

        Ok(())
    }

    #[allow(non_snake_case)]
    #[test]
    fn too_large_message__create_iterator__should_error() -> Result<()> {
        let message = UdpRoutingMessage::new(
            route!["onward"],
            route!["return"],
            vec![0; MAX_MESSAGE_SIZE].into(),
            None,
        );

        let routing_number = RoutingNumber::default();

        assert!(TransportMessagesIterator::new(routing_number, &message).is_err());

        Ok(())
    }

    #[allow(non_snake_case)]
    #[test]
    fn large_message__create_iterator__should_split_correctly() -> Result<()> {
        let message = UdpRoutingMessage::new(
            route!["onward"],
            route!["return"],
            vec![0; MAX_PAYLOAD_SIZE * 2].into(),
            None,
        );

        let routing_number = RoutingNumber::default();

        let mut iterator = TransportMessagesIterator::new(routing_number, &message)?;

        assert_eq!(iterator.current_routing_number, routing_number);
        assert_eq!(iterator.total, 3);
        assert_eq!(iterator.offset, 0);

        assert!(iterator.next().unwrap().is_ok());
        assert_eq!(iterator.current_routing_number, routing_number);
        assert_eq!(iterator.total, 3);
        assert_eq!(iterator.offset, 1);

        assert!(iterator.next().unwrap().is_ok());
        assert_eq!(iterator.current_routing_number, routing_number);
        assert_eq!(iterator.total, 3);
        assert_eq!(iterator.offset, 2);

        assert!(iterator.next().unwrap().is_ok());
        assert_eq!(iterator.current_routing_number, routing_number);
        assert_eq!(iterator.total, 3);
        assert_eq!(iterator.offset, 3);

        assert!(iterator.next().is_none());

        Ok(())
    }
}
