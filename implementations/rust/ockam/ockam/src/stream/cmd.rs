use crate::{
    protocols::{ProtocolParser, ProtocolPayload},
    Message, ProtocolId, Result,
};
use ockam_core::{compat::collections::BTreeSet, Decodable};
use serde::{Deserialize, Serialize};

/// A protocol exchanged between a stream consumer and stream producer
#[derive(Debug, Serialize, Deserialize, Message)]
pub enum StreamWorkerCmd {
    /// Trigger a fetch event
    ///
    /// These events are fired from worker to _itself_ to create a
    /// delayed reactive response
    Fetch,
    /// Pull messages from the consumer's buffer
    Pull {
        /// Number of messages to pull.
        ///
        /// Zero is used as a sentinel to indicate "all messages".
        num: usize,
    },
}

impl StreamWorkerCmd {
    /// Return a [`ProtocolPayload`] containing a
    /// [`Fetch`](StreamWorkerCmd::Fetch) event.
    pub fn fetch() -> ProtocolPayload {
        ProtocolPayload::new(ProtocolId::from("internal.stream.fetch"), Self::Fetch)
    }

    /// Pull messages from the consumer's buffer
    ///
    /// When sending `Pull { num: 0 }` all available messages are
    /// pulled.  It is recommended to configure your stream consumer
    /// into ["forwarding mode"](crate::stream::Stream::with_recipient).
    pub fn pull(num: usize) -> ProtocolPayload {
        ProtocolPayload::new(ProtocolId::from("internal.stream.pull"), Self::Pull { num })
    }
}

impl ProtocolParser for StreamWorkerCmd {
    fn check_id(id: &str) -> bool {
        vec!["internal.stream.fetch", "internal.stream.pull"]
            .into_iter()
            .collect::<BTreeSet<_>>()
            .contains(id)
    }

    fn parse(pp: ProtocolPayload) -> Result<Self> {
        StreamWorkerCmd::decode(&pp.data)
    }
}
