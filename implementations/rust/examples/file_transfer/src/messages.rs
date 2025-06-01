use ockam::{deserialize, serialize, Decodable, Encodable, Encoded, Message};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message)]
pub struct FileDescription {
    pub name: String,
    pub size: usize,
}

impl Encodable for FileDescription {
    fn encode(self) -> ockam::Result<Encoded> {
        serialize(self)
    }
}

impl Decodable for FileDescription {
    fn decode(v: &[u8]) -> ockam::Result<Self> {
        deserialize(v)
    }
}

#[derive(Serialize, Deserialize, Message)]
pub enum FileData {
    Description(FileDescription),
    Data(Vec<u8>),
    Quit,
}

impl Encodable for FileData {
    fn encode(self) -> ockam::Result<Encoded> {
        serialize(self)
    }
}

impl Decodable for FileData {
    fn decode(v: &[u8]) -> ockam::Result<Self> {
        deserialize(v)
    }
}
