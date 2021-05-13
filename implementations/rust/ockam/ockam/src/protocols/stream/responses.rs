//! Stream protocol response payloads and parser

use crate::{
    protocols::{ParserFragment, ProtocolPayload, UserParser},
    Message, ProtocolId, Result, Worker,
};
use serde::{Deserialize, Serialize};

/// Response to a `CreateStreamRequest`
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Init {
    pub stream_name: String,
}

impl Init {
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
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PushConfirm {
    pub request_id: usize,
    pub status: Status,
    pub index: usize, // uint
}

impl PushConfirm {
    pub fn new<S: Into<Status>>(request_id: usize, status: S, index: usize) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_push",
            Self {
                request_id,
                index,
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
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PullResponse {
    pub request_id: usize,
    pub messages: Vec<StreamMessage>,
}

impl PullResponse {
    pub fn new<T: Into<Vec<StreamMessage>>>(request_id: usize, messages: T) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_pull",
            Self {
                request_id,
                messages: messages.into(),
            },
        )
    }
}

/// A stream message with a reference index
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct StreamMessage {
    /// Index of the message in the stream
    pub index: usize,
    /// Encoded data of the message
    pub data: Vec<u8>,
}

/// The index return payload
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Index {
    pub stream_name: String,
    pub client_id: String,
    pub index: usize,
}

/// A convenience enum to wrap all possible response types
///
/// In your worker you will want to match this enum, given to you via
/// the `ProtocolParser` abstraction.
pub enum Response {
    Init(Init),
    PushConfirm(PushConfirm),
    PullResponse(PullResponse),
    Index(Index),
}

/// A stream protocol parser with user-provided receive hook
pub struct ResponseParser<W: Worker> {
    f: UserParser<W, Response>,
}

impl<W: Worker> ResponseParser<W> {
    /// Create a new stream protocol parser with a response closure
    ///
    /// The provided function will be called for every incoming
    /// response to the stream protocol.  You can use it, and the
    /// mutable access to your worker state to map response messages
    /// to the worker state.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&mut W, Response) + Send + Sync + 'static,
    {
        Self { f: Box::new(f) }
    }
}

impl<W: Worker> ParserFragment<W> for ResponseParser<W> {
    fn ids(&self) -> Vec<ProtocolId> {
        vec![
            "stream_create",
            "stream_push",
            "stream_pull",
            "stream_index",
        ]
        .into_iter()
        .map(Into::into)
        .collect()
    }

    fn parse(
        &self,
        state: &mut W,
        ProtocolPayload { protocol, data }: ProtocolPayload,
    ) -> Result<()> {
        // Parse payload into a response
        let resp = match protocol.as_str() {
            "stream_create" => Response::Init(Init::decode(&data)?),
            "stream_push" => Response::PushConfirm(PushConfirm::decode(&data)?),
            "stream_pull" => Response::PullResponse(PullResponse::decode(&data)?),
            "stream_index" => Response::Index(Index::decode(&data)?),
            _ => unreachable!(),
        };

        // Call the user code
        (*self.f)(state, resp);

        Ok(())
    }
}
