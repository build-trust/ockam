use ockam::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct FileDescription {
    pub name: String,
    pub size: usize,
}
impl Message for FileDescription {}

#[derive(Serialize, Deserialize)]
pub enum FileData {
    Description(FileDescription),
    Data(Vec<u8>),
    Quit,
}

impl Message for FileData {}
