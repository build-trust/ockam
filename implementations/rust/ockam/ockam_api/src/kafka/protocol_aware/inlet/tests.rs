use crate::kafka::key_exchange::KafkaKeyExchangeController;
use crate::kafka::protocol_aware::inlet::InletInterceptorImpl;
use crate::kafka::protocol_aware::{
    utils, KafkaEncryptedContent, KafkaMessageRequestInterceptor, KafkaMessageResponseInterceptor,
};
use crate::kafka::KafkaInletController;
use bytes::BytesMut;
use indexmap::IndexMap;
use kafka_protocol::messages::fetch_response::{FetchableTopicResponse, PartitionData};
use kafka_protocol::messages::produce_request::{PartitionProduceData, TopicProduceData};
use kafka_protocol::messages::ApiKey::ProduceKey;
use kafka_protocol::messages::{
    ApiKey, FetchResponse, ProduceRequest, RequestHeader, ResponseHeader, TopicName,
};
use kafka_protocol::protocol::{Builder, Decodable, StrBytes};
use kafka_protocol::records::{
    Compression, Record, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions, TimestampType,
};
use minicbor::{Decoder, Encoder};
use ockam_core::{async_trait, Address};
use ockam_node::Context;
use serde_json::json;
use std::sync::Arc;

const ENCRYPTED_PREFIX: &[u8] = b"encrypted:";
const PREFIX_LEN: usize = ENCRYPTED_PREFIX.len();

struct MockKafkaKeyExchangeController;

#[async_trait]
impl KafkaKeyExchangeController for MockKafkaKeyExchangeController {
    async fn encrypt_content(
        &self,
        _context: &mut Context,
        _topic_name: &str,
        content: Vec<u8>,
    ) -> ockam_core::Result<KafkaEncryptedContent> {
        let mut new_content = ENCRYPTED_PREFIX.to_vec();
        new_content.extend_from_slice(&content);
        Ok(KafkaEncryptedContent {
            consumer_decryptor_address: Address::from_string("mock"),
            content: new_content,
            rekey_counter: u16::MAX,
        })
    }

    async fn decrypt_content(
        &self,
        _context: &mut Context,
        _consumer_decryptor_address: &Address,
        _rekey_counter: u16,
        encrypted_content: Vec<u8>,
    ) -> ockam_core::Result<Vec<u8>> {
        Ok(encrypted_content[PREFIX_LEN..].to_vec())
    }

    async fn publish_consumer(
        &self,
        _context: &mut Context,
        _topic_name: &str,
    ) -> ockam_core::Result<()> {
        Ok(())
    }
}

const TEST_KAFKA_API_VERSION: i16 = 13;

pub fn create_kafka_produce_request(content: &[u8]) -> BytesMut {
    let header = RequestHeader::builder()
        .request_api_key(ApiKey::ProduceKey as i16)
        .request_api_version(TEST_KAFKA_API_VERSION)
        .correlation_id(1)
        .client_id(Some(StrBytes::from_static_str("my-client-id")))
        .unknown_tagged_fields(Default::default())
        .build()
        .unwrap();

    let mut encoded = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut encoded,
        [Record {
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 0,
            producer_epoch: 0,
            timestamp_type: TimestampType::Creation,
            offset: 0,
            sequence: 0,
            timestamp: 0,
            key: None,
            value: Some(BytesMut::from(content).freeze()),
            headers: Default::default(),
        }]
        .iter(),
        &RecordEncodeOptions {
            version: 2,
            compression: Compression::None,
        },
    )
    .unwrap();

    let mut topic_data = IndexMap::new();
    topic_data.insert(
        TopicName::from(StrBytes::from_static_str("topic-name")),
        TopicProduceData::builder()
            .partition_data(vec![PartitionProduceData::builder()
                .index(1)
                .records(Some(encoded.freeze()))
                .unknown_tagged_fields(Default::default())
                .build()
                .unwrap()])
            .unknown_tagged_fields(Default::default())
            .build()
            .unwrap(),
    );
    let request = ProduceRequest::builder()
        .transactional_id(None)
        .acks(0)
        .timeout_ms(0)
        .topic_data(topic_data)
        .unknown_tagged_fields(Default::default())
        .build()
        .unwrap();

    utils::encode_request(
        &header,
        &request,
        TEST_KAFKA_API_VERSION,
        ApiKey::ProduceKey,
    )
    .unwrap()
}

pub fn create_kafka_fetch_response(content: &[u8]) -> BytesMut {
    let header = ResponseHeader::builder()
        .correlation_id(1)
        .unknown_tagged_fields(Default::default())
        .build()
        .unwrap();

    let mut encoded = BytesMut::new();
    RecordBatchEncoder::encode(
        &mut encoded,
        [Record {
            transactional: false,
            control: false,
            partition_leader_epoch: 0,
            producer_id: 0,
            producer_epoch: 0,
            timestamp_type: TimestampType::Creation,
            offset: 0,
            sequence: 0,
            timestamp: 0,
            key: None,
            value: Some(BytesMut::from(content).freeze()),
            headers: Default::default(),
        }]
        .iter(),
        &RecordEncodeOptions {
            version: 2,
            compression: Compression::None,
        },
    )
    .unwrap();

    let response = FetchResponse::builder()
        .throttle_time_ms(Default::default())
        .error_code(Default::default())
        .session_id(Default::default())
        .responses(vec![FetchableTopicResponse::builder()
            .topic(TopicName::from(StrBytes::from_static_str("topic-name")))
            .topic_id(Default::default())
            .partitions(vec![PartitionData::builder()
                .partition_index(1)
                .error_code(Default::default())
                .high_watermark(Default::default())
                .last_stable_offset(Default::default())
                .log_start_offset(Default::default())
                .diverging_epoch(Default::default())
                .current_leader(Default::default())
                .snapshot_id(Default::default())
                .aborted_transactions(Default::default())
                .preferred_read_replica(Default::default())
                .records(Some(encoded.freeze()))
                .unknown_tagged_fields(Default::default())
                .build()
                .unwrap()])
            .unknown_tagged_fields(Default::default())
            .build()
            .unwrap()])
        .unknown_tagged_fields(Default::default())
        .build()
        .unwrap();

    utils::encode_response(&header, &response, TEST_KAFKA_API_VERSION, ApiKey::FetchKey).unwrap()
}

pub fn parse_produce_request(content: &[u8]) -> ProduceRequest {
    let mut buffer = BytesMut::from(content);
    let _header = RequestHeader::decode(
        &mut buffer,
        ProduceKey.request_header_version(TEST_KAFKA_API_VERSION),
    )
    .unwrap();
    utils::decode_body(&mut buffer, TEST_KAFKA_API_VERSION).unwrap()
}

pub fn parse_fetch_response(content: &[u8]) -> FetchResponse {
    let mut buffer = BytesMut::from(content);
    let _header = ResponseHeader::decode(
        &mut buffer,
        ApiKey::FetchKey.response_header_version(TEST_KAFKA_API_VERSION),
    )
    .unwrap();
    utils::decode_body(&mut buffer, TEST_KAFKA_API_VERSION).unwrap()
}

pub fn decode_field_value(value: String) -> serde_json::Value {
    let value = hex::decode(value).unwrap();
    let encrypted_content: KafkaEncryptedContent = Decoder::new(value.as_ref()).decode().unwrap();
    assert_eq!(
        encrypted_content.consumer_decryptor_address,
        Address::from_string("mock")
    );

    let encrypted_tag =
        String::from_utf8(encrypted_content.content[0..PREFIX_LEN].to_vec()).unwrap();
    assert_eq!(encrypted_tag.as_bytes(), ENCRYPTED_PREFIX);

    let cleartext_content = encrypted_content.content[PREFIX_LEN..].to_vec();
    serde_json::from_slice::<serde_json::Value>(&cleartext_content).unwrap()
}

pub fn encode_field_value(value: serde_json::Value) -> String {
    let cleartext_content = serde_json::to_vec(&value).unwrap();
    let mut encrypted_content = ENCRYPTED_PREFIX.to_vec();
    encrypted_content.extend_from_slice(&cleartext_content);

    let mut write_buffer = Vec::new();
    let mut encoder = Encoder::new(&mut write_buffer);
    encoder
        .encode(KafkaEncryptedContent {
            consumer_decryptor_address: Address::from_string("mock"),
            content: encrypted_content,
            rekey_counter: u16::MAX,
        })
        .unwrap();

    hex::encode(&write_buffer)
}

#[ockam::test]
pub async fn json_encrypt_specific_fields(context: &mut Context) -> ockam::Result<()> {
    let interceptor = InletInterceptorImpl::new(
        Arc::new(MockKafkaKeyExchangeController {}),
        Default::default(),
        KafkaInletController::stub(),
        true,
        vec![
            "field1".to_string(),
            "field2".to_string(),
            "field3".to_string(),
        ],
    );

    let encrypted_response = interceptor
        .intercept_request(
            context,
            create_kafka_produce_request(
                json!(
                    {
                        "field1": "value1",
                        "field2": {
                            "nested_field1": "nested_value1",
                            "nested_field2": "nested_value2"
                        },
                        "field3": [
                            "array_value1",
                            "array_value2"
                        ],
                        "cleartext_field": "cleartext_value"
                    }
                )
                .to_string()
                .as_bytes(),
            ),
        )
        .await
        .unwrap();

    let request = parse_produce_request(&encrypted_response);
    let topic_data = request.topic_data.first().unwrap();
    assert_eq!("topic-name", topic_data.0 .0.as_str());

    let mut batch_content = topic_data
        .1
        .partition_data
        .first()
        .cloned()
        .unwrap()
        .records
        .unwrap();

    let records = RecordBatchDecoder::decode(&mut batch_content).unwrap();
    let record = records.first().unwrap();
    let record_content = record.value.clone().unwrap();

    // The record content is a JSON object
    let json: serde_json::value::Value = serde_json::from_slice(&record_content).unwrap();
    let map = json.as_object().unwrap();

    let field1_value = decode_field_value(map.get("field1").unwrap().as_str().unwrap().to_string());
    assert_eq!(field1_value, json!("value1"));

    let field2_value = decode_field_value(map.get("field2").unwrap().as_str().unwrap().to_string());
    assert_eq!(
        field2_value,
        json!({"nested_field1": "nested_value1", "nested_field2": "nested_value2"})
    );

    let field3_value = decode_field_value(map.get("field3").unwrap().as_str().unwrap().to_string());
    assert_eq!(field3_value, json!(["array_value1", "array_value2"]));

    let cleartext_value = map.get("cleartext_field").unwrap().as_str().unwrap();
    assert_eq!(cleartext_value, "cleartext_value");

    Ok(())
}

#[ockam::test]
pub async fn json_decrypt_specific_fields(context: &mut Context) -> ockam::Result<()> {
    let interceptor = InletInterceptorImpl::new(
        Arc::new(MockKafkaKeyExchangeController {}),
        Default::default(),
        KafkaInletController::stub(),
        true,
        vec![
            "field1".to_string(),
            "field2".to_string(),
            "field3".to_string(),
        ],
    );

    interceptor.add_request(1, ApiKey::FetchKey, TEST_KAFKA_API_VERSION);

    let field1_value = encode_field_value(json!("value1"));
    let field2_value = encode_field_value(json!({
        "nested_field1": "nested_value1",
        "nested_field2": "nested_value2"
    }));
    let field3_value = encode_field_value(json!(["array_value1", "array_value2"]));

    let cleartext_response = interceptor
        .intercept_response(
            context,
            create_kafka_fetch_response(
                json!(
                    {
                        "field1": field1_value,
                        "field2": field2_value,
                        "field3": field3_value,
                        "cleartext_field": "cleartext_value"
                    }
                )
                .to_string()
                .as_bytes(),
            ),
        )
        .await
        .unwrap();

    let response = parse_fetch_response(&cleartext_response);
    let partition_data = response
        .responses
        .first()
        .unwrap()
        .partitions
        .first()
        .unwrap();
    let mut records = partition_data.records.clone().unwrap();
    let records = RecordBatchDecoder::decode(&mut records).unwrap();

    let record = records.first().unwrap();
    let value =
        serde_json::from_slice::<serde_json::Value>(record.value.as_ref().unwrap()).unwrap();

    assert_eq!(
        json!({
            "field1": "value1",
            "field2": {
                "nested_field1": "nested_value1",
                "nested_field2": "nested_value2"
            },
            "field3": [
                "array_value1",
                "array_value2"
            ],
            "cleartext_field": "cleartext_value"
        }),
        value
    );

    Ok(())
}
