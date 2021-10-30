//! Stream protocol request payloads

use crate::protocols::ProtocolPayload;
use crate::Message;
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::Uint;
use serde::{Deserialize, Serialize};

/// Request a new mailbox to be created
#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
pub struct CreateStreamRequest {
    pub stream_name: Option<String>,
}

impl CreateStreamRequest {
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
#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
pub struct PushRequest {
    pub request_id: Uint, // uint
    pub data: Vec<u8>,
}

impl PushRequest {
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

/// Pull messages from the mailbox
#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
pub struct PullRequest {
    pub request_id: Uint,
    pub index: Uint,
    pub limit: Uint,
}

impl PullRequest {
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

/// Index request protocols to get and save indices
#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
pub enum Index {
    Get {
        client_id: String,
        stream_name: String,
    },
    Save {
        client_id: String,
        stream_name: String,
        index: Uint,
    },
}

impl Index {
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
