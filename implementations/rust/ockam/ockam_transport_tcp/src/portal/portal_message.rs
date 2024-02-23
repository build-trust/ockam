use ockam_core::Message;
use serde::de::{EnumAccess, VariantAccess};
use serde::{Deserialize, Deserializer, Serialize};

/// A command message type for a Portal
#[derive(Serialize, Message, Debug)]
pub enum PortalMessage {
    /// First message that Inlet sends to the Outlet
    Ping,
    /// First message that Outlet sends to the Inlet
    Pong,
    /// Message to indicate that connection from Outlet to the target,
    /// or from the target to the Inlet was dropped
    Disconnect,
    /// Message with binary payload and packet counter
    Payload(Vec<u8>, #[serde(skip)] Option<u16>),
}

// Manually implement deserialization for PortalMessage
// to support deserializing older message types
impl<'de> Deserialize<'de> for PortalMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PayloadVisitor;

        impl<'de> serde::de::Visitor<'de> for PayloadVisitor {
            type Value = (Vec<u8>, Option<u16>);

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid Payload")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let payload = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

                // If the message is using V1 it won't contain the u8 to mark for counter presence
                // hence we return None and ignore reading errors.
                // However, if the field is present we can be confident that the rest of the message
                // must be present.
                let counter = if let Some(counter_set) = seq.next_element::<u8>().ok().flatten() {
                    if counter_set != 0 {
                        seq.next_element()?
                    } else {
                        None
                    }
                } else {
                    None
                };
                Ok((payload, counter))
            }
        }

        struct PortalMessageVisitor;

        impl<'de> serde::de::Visitor<'de> for PortalMessageVisitor {
            type Value = PortalMessage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid PortalMessage")
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                let (variant_index, variant): (u8, _) = data.variant()?;
                match variant_index {
                    0 => Ok(PortalMessage::Ping),
                    1 => Ok(PortalMessage::Pong),
                    2 => Ok(PortalMessage::Disconnect),
                    3 => {
                        // we expect 3 elements: the first is the payload, the second is the u8
                        // to mark for counter presence, and the third is the counter itself
                        let (payload, counter): (Vec<u8>, Option<u16>) =
                            variant.tuple_variant(3, PayloadVisitor)?;
                        Ok(PortalMessage::Payload(payload, counter))
                    }
                    _ => Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(variant_index as u64),
                        &self,
                    )),
                }
            }
        }

        const VARIANTS: &[&str] = &["Ping", "Pong", "Disconnect", "Payload"];
        deserializer.deserialize_enum("PortalMessage", VARIANTS, PortalMessageVisitor)
    }
}

/// An internal message type for a Portal
#[derive(Serialize, Deserialize, Message)]
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

        let encoded = PortalMessageV1::encode(&PortalMessageV1::Ping).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Ping));

        let encoded = PortalMessageV1::encode(&PortalMessageV1::Pong).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Pong));

        let encoded = PortalMessageV1::encode(&PortalMessageV1::Disconnect).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Disconnect));

        let encoded = PortalMessageV1::encode(&PortalMessageV1::Payload(payload.clone())).unwrap();
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

        let encoded = PortalMessage::encode(&PortalMessage::Ping).unwrap();
        let decoded = PortalMessageV1::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessageV1::Ping));

        let encoded = PortalMessage::encode(&PortalMessage::Pong).unwrap();
        let decoded = PortalMessageV1::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessageV1::Pong));

        let encoded = PortalMessage::encode(&PortalMessage::Disconnect).unwrap();
        let decoded = PortalMessageV1::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessageV1::Disconnect));

        let encoded =
            PortalMessage::encode(&PortalMessage::Payload(payload.clone(), Some(123))).unwrap();
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

        let encoded = PortalMessage::encode(&PortalMessage::Ping).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Ping));

        let encoded = PortalMessage::encode(&PortalMessage::Pong).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Pong));

        let encoded = PortalMessage::encode(&PortalMessage::Disconnect).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, PortalMessage::Disconnect));

        let encoded =
            PortalMessage::encode(&PortalMessage::Payload(payload.clone(), None)).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        if let PortalMessage::Payload(decoded_payload, packet_counter) = decoded {
            assert_eq!(decoded_payload, payload);
            assert_eq!(packet_counter, None);
        } else {
            panic!("Decoded message is not a Payload");
        }

        let encoded =
            PortalMessage::encode(&PortalMessage::Payload(payload.clone(), Some(123))).unwrap();
        let decoded = PortalMessage::decode(&encoded).unwrap();
        if let PortalMessage::Payload(decoded_payload, packet_counter) = decoded {
            assert_eq!(decoded_payload, payload);
            assert_eq!(packet_counter, Some(123));
        } else {
            panic!("Decoded message is not a Payload");
        }
    }
}
