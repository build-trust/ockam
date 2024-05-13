use crate::models::{ChangeHistory, CredentialAndPurposeKey};
use core::str::FromStr;
use minicbor::encode::{Error, Write};
use minicbor::{Decode, Decoder, Encode, Encoder};
use ockam_core::compat::vec::Vec;
use ockam_core::Route;
use uuid::Uuid;

/// Secure Channel Message format.
#[derive(Debug, Encode, Decode, Clone)]
#[rustfmt::skip]
pub enum SecureChannelMessageV1<'a> {
    /// Encrypted payload message.
    #[n(0)] Payload(#[b(0)] PlaintextPayloadMessage<'a>),
    /// Present credentials one more time.
    #[n(1)] RefreshCredentials(#[n(0)] RefreshCredentialsMessage),
    /// Close the channel.
    #[n(2)] Close,
}

impl<'a> SecureChannelMessageV1<'a> {
    pub(crate) fn into_v2(self) -> SecureChannelMessageV2<'a> {
        match self {
            SecureChannelMessageV1::Payload(p) => SecureChannelMessageV2::Payload(p),
            SecureChannelMessageV1::RefreshCredentials(c) => {
                SecureChannelMessageV2::RefreshCredentials(c)
            }
            SecureChannelMessageV1::Close => SecureChannelMessageV2::Close,
        }
    }
}

/// Secure Channel Message format.
/// This new version supports multipart payloads
#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[rustfmt::skip]
pub enum SecureChannelMessageV2<'a> {
    /// Encrypted payload message.
    #[n(0)] Payload(#[b(0)] PlaintextPayloadMessage<'a>),
    /// Present credentials one more time.
    #[n(1)] RefreshCredentials(#[n(0)] RefreshCredentialsMessage),
    /// Close the channel.
    #[n(2)] Close,
    /// Encrypted payload message part
    #[n(3)] PayloadPart(#[b(0)] PlaintextPayloadPartMessage<'a>),
}

/// Secure Channel Message format.
#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[rustfmt::skip]
pub struct PlaintextPayloadPartMessage<'a> {
    /// Onward route of the message.
    #[n(0)] pub onward_route: Route,
    /// Return route of the message.
    #[n(1)] pub return_route: Route,
    /// Untyped binary payload.
    #[cbor(with = "minicbor::bytes")]
    #[b(2)] pub payload: &'a [u8],
    /// Number for this part
    #[n(3)] pub current_part_number: u32,
    /// Total number of expected parts
    #[n(4)] pub total_number_of_parts: u32,
    /// Message UUID, used to identify which parts belong to which message
    #[n(5)] pub payload_uuid: UuidCbor,
}

/// Wrapper around the Uuid data type to provide CBOR instances for that type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UuidCbor(Uuid);

impl UuidCbor {
    /// Wrap a Uuid as UuidCbor for serialization
    pub fn new(uuid: Uuid) -> UuidCbor {
        UuidCbor(uuid)
    }
}

impl From<UuidCbor> for Uuid {
    fn from(value: UuidCbor) -> Self {
        value.0
    }
}

impl<C> Encode<C> for UuidCbor {
    fn encode<W: Write>(&self, e: &mut Encoder<W>, ctx: &mut C) -> Result<(), Error<W::Error>> {
        self.0.to_string().encode(e, ctx)
    }
}

impl<'b, C> Decode<'b, C> for UuidCbor {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let bs = String::decode(d, ctx)?;
        Ok(UuidCbor(
            Uuid::from_str(&bs).map_err(minicbor::decode::Error::message)?,
        ))
    }
}

/// Secure Channel Message format.
#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[rustfmt::skip]
pub struct PlaintextPayloadMessage<'a> {
    /// Onward route of the message.
    #[n(0)] pub onward_route: Route,
    /// Return route of the message.
    #[n(1)] pub return_route: Route,
    /// Untyped binary payload.
    #[cbor(with = "minicbor::bytes")]
    #[b(2)] pub payload: &'a [u8],
}

/// Secure Channel Message format.
#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq)]
#[rustfmt::skip]
pub struct RefreshCredentialsMessage {
    /// Exported identity
    #[n(0)] pub change_history: ChangeHistory,
    /// Credentials associated to the identity along with corresponding Credentials Purpose Keys
    /// to verify those Credentials
    #[n(1)] pub credentials: Vec<CredentialAndPurposeKey>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::route;

    #[test]
    fn a_payload_can_be_encoded_then_decoded() {
        let expected = SecureChannelMessageV2::Payload(PlaintextPayloadMessage {
            onward_route: route!["1#onward_route"],
            return_route: route!["1#return_route"],
            payload: &[1, 2, 3],
        });
        let encoded = minicbor::to_vec(expected.clone()).unwrap();

        // double-check the encoding
        // and use this value on the Elixir side to check the decoding in messages_test.exs
        assert_eq!(hex::encode(encoded.clone()), "8200818381a20101028c186f186e1877186118721864185f1872186f18751874186581a20101028c18721865187418751872186e185f1872186f18751874186543010203");

        // now decode
        let actual: SecureChannelMessageV2 = minicbor::decode(&encoded).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn a_payload_can_be_decoded_from_elixir() {
        let hex_msg = "8200818381a20101028c186f186e1877186118721864185f1872186f18751874186581a20101028c18721865187418751872186e185f1872186f18751874186543010203";
        let decoded = hex::decode(hex_msg).unwrap();
        let actual: SecureChannelMessageV2 = minicbor::decode(&decoded).unwrap();

        let expected = SecureChannelMessageV2::Payload(PlaintextPayloadMessage {
            onward_route: route!["1#onward_route"],
            return_route: route!["1#return_route"],
            payload: &[1, 2, 3],
        });
        assert_eq!(actual, expected);
    }

    #[test]
    fn a_payload_part_can_be_encoded_then_decoded() {
        let expected = SecureChannelMessageV2::PayloadPart(PlaintextPayloadPartMessage {
            onward_route: route!["1#onward_route"],
            return_route: route!["1#return_route"],
            payload: &[1, 2, 3],
            current_part_number: 1,
            total_number_of_parts: 3,
            payload_uuid: UuidCbor(Uuid::from_str("24922fc8-ea4c-4387-b069-e2b296e0de7d").unwrap()),
        });

        let encoded = minicbor::to_vec(expected.clone()).unwrap();

        // double-check the encoding
        // and use this value on the Elixir side to check the decoding in messages_test.exs
        assert_eq!(hex::encode(encoded.clone()), "8203818681a20101028c186f186e1877186118721864185f1872186f18751874186581a20101028c18721865187418751872186e185f1872186f187518741865430102030103782432343932326663382d656134632d343338372d623036392d653262323936653064653764");
        // now decode
        let actual: SecureChannelMessageV2 = minicbor::decode(&encoded).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn a_payload_part_can_be_decoded_from_elixir() {
        let hex_msg = "8203818681a20101028c186f186e1877186118721864185f1872186f18751874186581a20101028c18721865187418751872186e185f1872186f187518741865430102030103782432343932326663382d656134632d343338372d623036392d653262323936653064653764";
        let decoded = hex::decode(hex_msg).unwrap();
        let actual: SecureChannelMessageV2 = minicbor::decode(&decoded).unwrap();

        let expected = SecureChannelMessageV2::PayloadPart(PlaintextPayloadPartMessage {
            onward_route: route!["1#onward_route"],
            return_route: route!["1#return_route"],
            payload: &[1, 2, 3],
            current_part_number: 1,
            total_number_of_parts: 3,
            payload_uuid: UuidCbor(Uuid::from_str("24922fc8-ea4c-4387-b069-e2b296e0de7d").unwrap()),
        });

        assert_eq!(actual, expected);
    }
}
