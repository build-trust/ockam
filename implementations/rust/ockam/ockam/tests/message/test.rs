use ockam::*;
use serde::{Deserialize, Serialize};

#[derive(Message, Deserialize, Serialize)]
pub struct Tmp {
    a: String,
}

impl Encodable for Tmp {
    fn encode(self) -> Result<Encoded> {
        serialize(self)
    }
}
impl Decodable for Tmp {
    fn decode(e: &[u8]) -> Result<Tmp> {
        deserialize(e)
    }
}

#[derive(Message, Deserialize, Serialize)]
pub struct Tmp1 {
    a: Vec<u8>,
    b: Vec<Tmp>,
}

impl Encodable for Tmp1 {
    fn encode(self) -> Result<Encoded> {
        serialize(self)
    }
}
impl Decodable for Tmp1 {
    fn decode(e: &[u8]) -> Result<Tmp1> {
        deserialize(e)
    }
}

fn assert_impl<T: Message>() {}
fn main() {
    assert_impl::<String>();
    assert_impl::<Tmp>();
    assert_impl::<Tmp1>();
}
