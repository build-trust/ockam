use ockam_core::bare::{read_slice, write_slice};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Encodable, Encoded, Message, NeutralMessage};
use serde::{Deserialize, Serialize};

/// A command message type for a Portal
#[derive(Debug, PartialEq, Eq)]
pub enum PortalMessage<'de> {
    /// First message that Inlet sends to the Outlet
    Ping,
    /// First message that Outlet sends to the Inlet
    Pong,
    /// Message to indicate that connection from Outlet to the target,
    /// or from the target to the Inlet was dropped
    Disconnect,
    /// Message with binary payload and packet counter
    // TODO: Add route_index. May not be as important as for eBPF portals, as regular portals
    //  require reliable channel anyways. And if PortalMessage is sent over a channel that
    //  guarantees ordering, we don't need route_index
    Payload(&'de [u8], Option<u16>),
}

impl<'de> PortalMessage<'de> {
    /// Decode a slice into a PortalMessage without making a copy
    pub fn decode(slice: &'de [u8]) -> ockam_core::Result<PortalMessage<'de>> {
        Self::internal_decode(slice).ok_or_else(|| {
            ockam_core::Error::new(Origin::Transport, Kind::Protocol, "Invalid message")
        })
    }

    fn internal_decode(slice: &'de [u8]) -> Option<PortalMessage<'de>> {
        #[allow(clippy::get_first)]
        let enum_variant = slice.get(0)?;
        let mut index = 1;
        match enum_variant {
            0 => Some(PortalMessage::Ping),
            1 => Some(PortalMessage::Pong),
            2 => Some(PortalMessage::Disconnect),
            3 => {
                if let Some(payload) = read_slice(slice, &mut index) {
                    let counter = if slice.len() - index >= 3 {
                        let has_counter = slice[index];
                        index += 1;
                        if has_counter == 1 {
                            Some(u16::from_le_bytes(
                                slice[index..index + 2].try_into().unwrap(),
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    Some(PortalMessage::Payload(payload, counter))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Shortcut to encode a PortalMessage into a NeutralMessage
    pub fn to_neutral_message(self) -> ockam_core::Result<NeutralMessage> {
        Ok(NeutralMessage::from(self.encode()?))
    }
}

impl Encodable for PortalMessage<'_> {
    fn encode(self) -> ockam_core::Result<Encoded> {
        self.internal_encode()
            .map_err(|e| ockam_core::Error::new(Origin::Transport, Kind::Protocol, e.to_string()))
    }
}

impl PortalMessage<'_> {
    fn internal_encode(self) -> std::io::Result<Encoded> {
        match self {
            PortalMessage::Ping => Ok(vec![0]),
            PortalMessage::Pong => Ok(vec![1]),
            PortalMessage::Disconnect => Ok(vec![2]),
            PortalMessage::Payload(payload, counter) => {
                // to avoid an extra allocation, it's worth doing some math
                let capacity = 1 + payload.len() + if counter.is_some() { 3 } else { 1 } + {
                    ockam_core::bare::size_of_variable_length(payload.len() as u64)
                };
                let mut vec = Vec::with_capacity(capacity);
                vec.push(3);
                write_slice(&mut vec, payload);
                // TODO: re-enable once orchestrator accepts packet counter
                // if let Some(counter) = counter {
                //     vec.push(1); // has counter
                //     vec.extend_from_slice(&counter.to_le_bytes())
                // } else {
                //     vec.push(0);
                // }
                Ok(vec)
            }
        }
    }
}

/// An internal message type for a Portal
#[derive(Serialize, Deserialize, Message, PartialEq, Eq)]
pub enum PortalInternalMessage {
    /// Connection was dropped
    Disconnect,
}

/// Maximum allowed size for a payload
pub const MAX_PAYLOAD_SIZE: usize = 48 * 1024;

#[cfg(test)]
mod test {
    use crate::PortalMessage;
    use ockam_core::Message;
    use ockam_core::{Decodable, Encodable};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Message, Debug)]
    pub enum PortalMessageV1 {
        Ping,
        Pong,
        Disconnect,
        Payload(Vec<u8>),
    }

    #[test]
    fn older_message_can_be_decoded() {
        let payload = "hello".as_bytes().to_vec();

        let encoded = PortalMessageV1::encode(PortalMessageV1::Ping).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Ping));

        let encoded = PortalMessageV1::encode(PortalMessageV1::Pong).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Pong));

        let encoded = PortalMessageV1::encode(PortalMessageV1::Disconnect).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Disconnect));

        let encoded = PortalMessageV1::encode(PortalMessageV1::Payload(payload.clone())).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        if let PortalMessage::Payload(decoded_payload, _) = decoded {
            assert_eq!(decoded_payload, payload);
        } else {
            panic!("Decoded message is not a Payload");
        }
    }

    #[test]
    fn newer_message_can_be_decoded() {
        let payload = "hello".as_bytes().to_vec();

        let encoded = PortalMessage::encode(PortalMessage::Ping).unwrap();
        let decoded = PortalMessageV1::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessageV1::Ping));

        let encoded = PortalMessage::encode(PortalMessage::Pong).unwrap();
        let decoded = PortalMessageV1::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessageV1::Pong));

        let encoded = PortalMessage::encode(PortalMessage::Disconnect).unwrap();
        let decoded = PortalMessageV1::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessageV1::Disconnect));

        let encoded = PortalMessage::encode(PortalMessage::Payload(&payload, Some(123))).unwrap();
        let decoded = PortalMessageV1::decode(&encoded).unwrap();
        if let PortalMessageV1::Payload(decoded_payload) = decoded {
            assert_eq!(decoded_payload, payload);
        } else {
            panic!("Decoded message is not a Payload");
        }
    }

    #[ignore]
    #[test]
    fn newer_message_can_be_encoded() {
        let payload = "hello".as_bytes().to_vec();

        let encoded = PortalMessage::encode(PortalMessage::Ping).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Ping));

        let encoded = PortalMessage::encode(PortalMessage::Pong).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Pong));

        let encoded = PortalMessage::encode(PortalMessage::Disconnect).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Disconnect));

        let encoded = PortalMessage::encode(PortalMessage::Payload(&payload, None)).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        if let PortalMessage::Payload(decoded_payload, packet_counter) = decoded {
            assert_eq!(decoded_payload, payload);
            assert_eq!(packet_counter, None);
        } else {
            panic!("Decoded message is not a Payload");
        }

        let encoded = PortalMessage::encode(PortalMessage::Payload(&payload, Some(123))).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        if let PortalMessage::Payload(decoded_payload, packet_counter) = decoded {
            assert_eq!(decoded_payload, payload);
            assert_eq!(packet_counter, Some(123));
        } else {
            panic!("Decoded message is not a Payload");
        }
    }
}
