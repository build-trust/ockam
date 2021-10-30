//! Stream protocol response payloads and parser

use crate::{
    protocols::{ProtocolParser, ProtocolPayload},
    Message, OckamError, Result,
};
use ockam_core::compat::{collections::BTreeSet, string::String, vec::Vec};
use ockam_core::{Decodable, Uint};
use serde::{Deserialize, Serialize};

/// Response to a `CreateStreamRequest`
#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
pub struct Init {
    pub stream_name: String,
}

impl Init {
    //noinspection RsExternalLinter
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new<S: Into<String>>(s: S) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_create",
            Self {
                stream_name: s.into(),
            },
        )
    }
}

/// Confirm push operation on the mailbox
#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
pub struct PushConfirm {
    pub request_id: Uint,
    pub status: Status,
    pub index: Uint,
}

impl PushConfirm {
    //noinspection RsExternalLinter
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new<S: Into<Status>>(request_id: u64, status: S, index: u64) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_push",
            Self {
                request_id: request_id.into(),
                index: index.into(),
                status: status.into(),
            },
        )
    }
}

/// A simple status code
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Status {
    Ok,
    Error,
}

impl From<bool> for Status {
    fn from(b: bool) -> Self {
        if b {
            Self::Ok
        } else {
            Self::Error
        }
    }
}

impl From<Option<()>> for Status {
    fn from(b: Option<()>) -> Self {
        b.map(|_| Self::Ok).unwrap_or(Self::Error)
    }
}

/// Response to a `PullRequest`
#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
pub struct PullResponse {
    pub request_id: Uint,
    pub messages: Vec<StreamMessage>,
}

impl PullResponse {
    //noinspection RsExternalLinter
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new<T: Into<Vec<StreamMessage>>>(request_id: u64, messages: T) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_pull",
            Self {
                request_id: request_id.into(),
                messages: messages.into(),
            },
        )
    }
}

/// A stream message with a reference index
#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
pub struct StreamMessage {
    /// Index of the message in the stream
    pub index: Uint,
    /// Encoded data of the message
    pub data: Vec<u8>,
}

/// The index return payload
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Index {
    pub client_id: String,
    pub stream_name: String,
    pub index: Option<Uint>,
}

/// A convenience enum to wrap all possible response types
///
/// In your worker you will want to match this enum, given to you via
/// the `ProtocolParser` abstraction.
#[allow(clippy::enum_variant_names)]
#[derive(Serialize, Deserialize, Message)]
pub enum Response {
    Init(Init),
    PushConfirm(PushConfirm),
    PullResponse(PullResponse),
    Index(Index),
}

impl ProtocolParser for Response {
    fn check_id(id: &str) -> bool {
        vec![
            "stream_create",
            "stream_push",
            "stream_pull",
            "stream_index",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>()
        .contains(id)
    }

    fn parse(ProtocolPayload { protocol, data }: ProtocolPayload) -> Result<Self> {
        Ok(match protocol.as_str() {
            "stream_create" => Response::Init(Init::decode(&data)?),
            "stream_push" => Response::PushConfirm(PushConfirm::decode(&data)?),
            "stream_pull" => Response::PullResponse(PullResponse::decode(&data)?),
            "stream_index" => Response::Index(Index::decode(&data)?),
            _ => return Err(OckamError::NoSuchProtocol.into()),
        })
    }
}
