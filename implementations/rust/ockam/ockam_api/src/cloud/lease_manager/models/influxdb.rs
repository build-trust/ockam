use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

// ======= TOKEN STRUCT =======
#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cbor(map)]
pub struct Token {
    #[cbor(b(1))]
    pub id: String,

    #[cbor(b(2))]
    pub issued_for: String,

    #[cbor(b(3))]
    pub created_at: String,

    #[cbor(b(4))]
    pub expires: String,

    #[cbor(b(5))]
    pub token: String,

    #[cbor(b(6))]
    pub status: String,
}
