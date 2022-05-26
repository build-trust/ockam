use minicbor::{Decode, Encode};
use std::borrow::Cow;

#[derive(Decode, Debug)]
#[cbor(map)]
pub struct Space<'a> {
    #[b(0)]
    pub id: Cow<'a, str>,
    #[b(1)]
    pub name: Cow<'a, str>,
}

#[derive(Encode)]
#[cbor(map)]
pub struct CreateSpace<'a> {
    #[b(0)]
    pub name: Cow<'a, str>,
}

impl<'a> CreateSpace<'a> {
    pub fn new<S: Into<Cow<'a, str>>>(name: S) -> Self {
        Self { name: name.into() }
    }
}
