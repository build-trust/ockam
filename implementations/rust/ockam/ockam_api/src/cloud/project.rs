use minicbor::{Decode, Encode};
use std::borrow::Cow;

#[derive(Decode, Debug)]
#[cbor(map)]
pub struct Project<'a> {
    #[b(0)]
    pub id: Cow<'a, str>,
    #[b(1)]
    pub name: Cow<'a, str>,
    #[b(2)]
    pub services: Vec<Cow<'a, str>>,
    #[b(3)]
    pub access_route: Vec<u8>,
}

#[derive(Encode)]
#[cbor(map)]
pub struct CreateProject<'a> {
    #[b(0)]
    pub name: Cow<'a, str>,
    #[b(1)]
    pub services: Vec<Cow<'a, str>>,
}

impl<'a> CreateProject<'a> {
    pub fn new<S: Into<Cow<'a, str>>>(name: S, services: &'a [String]) -> Self {
        Self {
            name: name.into(),
            services: services
                .iter()
                .map(String::as_str)
                .map(Cow::Borrowed)
                .collect(),
        }
    }
}
