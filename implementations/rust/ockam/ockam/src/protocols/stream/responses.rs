//! Stream protocol response payloads and parser

use crate::{
    protocols::{ParserFragment, ProtocolPayload},
    Any, Context, Message, ProtocolId, Result, Routed, Worker,
};
use ockam_core::compat::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_bare::Uint;

/// Response to a `CreateStreamRequest`
#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
                request_id: Uint(request_id),
                index: Uint(index),
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
                request_id: Uint(request_id),
                messages: messages.into(),
            },
        )
    }
}

/// A stream message with a reference index
#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
#[derive(Serialize, Deserialize)]
pub enum Response {
    Init(Init),
    PushConfirm(PushConfirm),
    PullResponse(PullResponse),
    Index(Index),
}

/// A stream protocol parser with user-provided receive hook
pub struct ResponseParser<W, F>
where
    W: Worker,
    F: Fn(&mut W, &mut Context, Routed<Response>) -> bool,
{
    f: F,
    _w: core::marker::PhantomData<W>,
}

impl<W, F> ResponseParser<W, F>
where
    W: Worker,
    F: Fn(&mut W, &mut Context, Routed<Response>) -> bool,
{
    //noinspection RsExternalLinter
    /// Create a new stream protocol parser with a response closure
    ///
    /// The provided function will be called for every incoming
    /// response to the stream protocol.  You can use it, and the
    /// mutable access to your worker state to map response messages
    /// to the worker state.
    #[allow(dead_code)]
    pub fn new(f: F) -> Self {
        Self {
            f,
            _w: core::marker::PhantomData,
        }
    }
}

impl<W, F> ParserFragment<W> for ResponseParser<W, F>
where
    W: Worker,
    F: Fn(&mut W, &mut Context, Routed<Response>) -> bool,
{
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
        ctx: &mut Context,
        routed: &Routed<Any>,
        ProtocolPayload { protocol, data }: ProtocolPayload,
    ) -> Result<bool> {
        // Parse payload into a response
        let resp = match protocol.as_str() {
            "stream_create" => Response::Init(Init::decode(&data)?),
            "stream_push" => Response::PushConfirm(PushConfirm::decode(&data)?),
            "stream_pull" => Response::PullResponse(PullResponse::decode(&data)?),
            "stream_index" => Response::Index(Index::decode(&data)?),
            _ => unreachable!(),
        };

        let (addr, local_msg) = routed.dissolve();

        // Call the user code
        let handled = (&self.f)(state, ctx, Routed::new(resp, addr, local_msg));
        Ok(handled)
    }
}
