use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

// ======= TOKEN STRUCT =======
#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cbor(map)]
pub struct Token {
    #[cbor(n(1))]
    pub id: String,

    #[cbor(n(2))]
    pub issued_for: String,

    #[cbor(n(3))]
    pub created_at: String,

    #[cbor(n(4))]
    pub expires: String,

    #[cbor(n(5))]
    pub token: String,

    #[cbor(n(6))]
    pub status: String,
}
