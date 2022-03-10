use ockam::Message;
use serde::{Deserialize, Serialize};

#[derive(Message, Deserialize, Serialize)]
pub struct Tmp {
    a: String,
}

#[derive(Message, Deserialize, Serialize)]
pub struct Tmp1 {
    a: Vec<u8>,
    b: Vec<Tmp>,
}

fn assert_impl<T: Message>() {}
fn main() {
    assert_impl::<String>();
    assert_impl::<Tmp>();
    assert_impl::<Tmp1>();
}
