//! Stream protocol request payloads

use crate::protocols::ProtocolPayload;
use crate::Message;
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::Uint;
use serde::{Deserialize, Serialize};

/// Request a new mailbox to be created
///
/// The expected response to this request is
/// [`InitResponse`](super::responses::InitResponse).
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
pub struct CreateStreamRequest {
    /// The stream name.
    pub stream_name: Option<String>,
}

impl CreateStreamRequest {
    /// Create a [`ProtocolPayload`] for a [`CreateStreamRequest`].
    //noinspection ALL
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new<S: Into<Option<String>>>(s: S) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_create",
            Self {
                stream_name: s.into(),
            },
        )
    }
}

/// Push a message into the mailbox
///
/// The expected response to this request is
/// [`PushConfirm`](super::responses::PushConfirm).
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
pub struct PushRequest {
    /// The request ID
    pub request_id: Uint,
    /// The encoded message data.
    pub data: Vec<u8>,
}

impl PushRequest {
    /// Create a [`ProtocolPayload`] for a [`PushRequest`].
    //noinspection ALL
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new<T: Into<Vec<u8>>>(request_id: u64, data: T) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_push",
            Self {
                request_id: request_id.into(),
                data: data.into(),
            },
        )
    }
}

/// Pull messages from the mailbox.
///
/// The expected response to this request is [`super::responses::PullResponse`].
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
pub struct PullRequest {
    /// The request id
    pub request_id: Uint,
    /// The stream index
    pub index: Uint,
    /// The number of messages to pull
    ///
    /// Zero is used as a sentinel to indicate all messages.
    pub limit: Uint,
}

impl PullRequest {
    /// Create a [`ProtocolPayload`] for a [`PullRequest`].
    //noinspection ALL
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new(request_id: u64, index: u64, limit: u64) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_pull",
            Self {
                request_id: request_id.into(),
                index: index.into(),
                limit: limit.into(),
            },
        )
    }
}

/// Index request protocols to get and save indices.
///
/// The expected response to this request is
/// [`IndexResponse`](super::responses::IndexResponse).
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
pub enum IndexRequest {
    /// A request for an index
    Get {
        /// The client id
        client_id: String,
        /// The stream name
        stream_name: String,
    },
    /// A request to save an index
    Save {
        /// The client id
        client_id: String,
        /// The stream name
        stream_name: String,
        /// The index to save
        index: Uint,
    },
}

impl IndexRequest {
    /// Create a new request to fetch the index.
    //noinspection ALL
    #[allow(dead_code)]
    pub fn get<S: Into<String>>(stream_name: S, client_id: S) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_index",
            Self::Get {
                client_id: client_id.into(),
                stream_name: stream_name.into(),
            },
        )
    }

    /// Create a new request to save the provided index.
    //noinspection ALL
    #[allow(dead_code)]
    pub fn save<S: Into<String>>(stream_name: S, client_id: S, index: u64) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_index",
            Self::Save {
                client_id: client_id.into(),
                stream_name: stream_name.into(),
                index: index.into(),
            },
        )
    }
}
