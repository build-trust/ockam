//! Stream protocol request payloads

use crate::protocols::ProtocolPayload;
use serde::{Deserialize, Serialize};

/// Request a new mailbox to be created
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateStreamRequest {
    pub stream_name: Option<String>,
}

impl CreateStreamRequest {
    pub fn new<'s, S: Into<Option<&'s str>>>(s: S) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_create",
            Self {
                stream_name: s.into().map(|s| s.to_string()),
            },
        )
    }
}

/// Push a message into the mailbox
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PushRequest {
    pub request_id: usize, // uint
    pub data: Vec<u8>,
}

impl PushRequest {
    pub fn new<T: Into<Vec<u8>>>(request_id: usize, data: T) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_push",
            Self {
                request_id,
                data: data.into(),
            },
        )
    }
}

/// Pull messages from the mailbox
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PullRequest {
    pub request_id: usize,
    pub index: usize,
    pub limit: usize,
}

impl PullRequest {
    pub fn new(request_id: usize, index: usize, limit: usize) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_pull",
            Self {
                request_id,
                index,
                limit,
            },
        )
    }
}

/// Index request protocols to get and save indices
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Index {
    Get {
        stream_name: String,
        client_id: String,
    },
    Save {
        stream_name: String,
        client_id: String,
        index: usize,
    },
}

impl Index {
    pub fn get<S: Into<String>>(stream_name: S, client_id: S) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_index",
            Self::Get {
                stream_name: stream_name.into(),
                client_id: client_id.into(),
            },
        )
    }

    pub fn save<S: Into<String>>(stream_name: S, client_id: S, index: usize) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_index",
            Self::Save {
                stream_name: stream_name.into(),
                client_id: client_id.into(),
                index,
            },
        )
    }
}
