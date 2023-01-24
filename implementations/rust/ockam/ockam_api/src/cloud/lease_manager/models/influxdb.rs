use minicbor::{Decode, Encode};
use ockam_core::CowStr;
use serde::{Deserialize, Serialize};


// ======= TOKEN STRUCT =======
#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cbor(map)]
pub struct Token<'a> {
    #[serde(borrow)]
    #[cbor(b(1))]
    pub id: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(2))]
    pub issued_for: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(3))]
    pub created_at: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(4))]
    pub expires: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(5))]
    pub token: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(6))]
    pub status: CowStr<'a>,
}
