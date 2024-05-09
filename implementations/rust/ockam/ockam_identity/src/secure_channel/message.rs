use crate::models::{ChangeHistory, CredentialAndPurposeKey};
use minicbor::encode::{Error, Write};
use minicbor::{Decode, Decoder, Encode, Encoder};
use ockam_core::compat::vec::Vec;
use ockam_core::Route;
use uuid::{Bytes, Uuid};

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
    pub(crate) fn to_v2(self) -> SecureChannelMessageV2<'a> {
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
#[derive(Debug, Encode, Decode, Clone)]
#[rustfmt::skip]
pub enum SecureChannelMessageV2<'a> {
    /// Encrypted payload message.
    #[n(0)] Payload(#[b(0)] PlaintextPayloadMessage<'a>),
    /// Present credentials one more time.
    #[n(1)] RefreshCredentials(#[n(0)] RefreshCredentialsMessage),
    /// Close the channel.
    #[n(2)] Close,
    /// Encrypted payload message part
    #[n(3)] PayloadPart {
        /// Current message part
        #[b(0)] part: PlaintextPayloadMessage<'a>,
        /// Message UUID, used to identify which parts belong to which message
        #[n(1)] payload_uuid: UuidCbor,
        /// Number for this part
        #[n(2)] current_part_number: u32,
        /// Total number of expected parts
        #[n(3)] total_number_of_parts: u32,
    },
}

/// Wrapper around the Uuid data type to provide CBOR instances for that type
#[derive(Debug, Clone)]
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
        self.0.as_bytes().encode(e, ctx)
    }

    fn is_nil(&self) -> bool {
        false
    }
}

impl<'b, C> Decode<'b, C> for UuidCbor {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let bs = Bytes::decode(d, ctx)?;
        Ok(UuidCbor(Uuid::from_bytes(bs)))
    }

    fn nil() -> Option<Self> {
        None
    }
}

/// Secure Channel Message format.
#[derive(Debug, Encode, Decode, Clone)]
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
#[derive(Debug, Encode, Decode, Clone)]
#[rustfmt::skip]
pub struct RefreshCredentialsMessage {
    /// Exported identity
    #[n(0)] pub change_history: ChangeHistory,
    /// Credentials associated to the identity along with corresponding Credentials Purpose Keys
    /// to verify those Credentials
    #[n(1)] pub credentials: Vec<CredentialAndPurposeKey>,
}
