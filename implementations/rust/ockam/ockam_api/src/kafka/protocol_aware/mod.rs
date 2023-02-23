use kafka_protocol::messages::ApiKey;
use minicbor::{Decode, Encode};

use ockam_core::compat::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use ockam_core::AsyncTryClone;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

use crate::kafka::secure_channel_map::{KafkaSecureChannelController, UniqueSecureChannelId};

mod request;
mod response;
mod utils;

#[derive(Clone, Debug)]
struct RequestInfo {
    pub request_api_key: ApiKey,
    pub request_api_version: i16,
}

type CorrelationId = i32;

/// map shared across all kafka workers, since the client might request it
/// only from one connection
pub(super) type TopicUuidMap = Arc<Mutex<HashMap<String, String>>>;

#[derive(AsyncTryClone)]
pub(crate) struct Interceptor {
    request_map: Arc<Mutex<HashMap<CorrelationId, RequestInfo>>>,
    uuid_to_name: TopicUuidMap,
    secure_channel_controller: Arc<dyn KafkaSecureChannelController>,
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
///Wraps the content within every record batch
struct MessageWrapper {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1652220>,
    #[b(1)] secure_channel_identifier: UniqueSecureChannelId,
    #[b(2)] content: Vec<u8>
}

impl Interceptor {
    pub(crate) fn new(
        secure_channel_controller: Arc<dyn KafkaSecureChannelController>,
        uuid_to_name: TopicUuidMap,
    ) -> Interceptor {
        Self {
            request_map: Arc::new(Mutex::new(Default::default())),
            uuid_to_name,
            secure_channel_controller,
        }
    }
}
