use minicbor::{Decode, Encode};
use serde::Serialize;

use super::RoleInShare;

#[derive(Clone, Debug, Decode, Encode, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct AcceptInvitation {
    #[n(1)] pub id: String,
}

#[derive(Clone, Debug, Decode, Encode, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct AcceptedInvitation {
    #[n(1)] pub id: String,
    #[n(2)] pub scope: RoleInShare,
    #[n(3)] pub target_id: String,
}
