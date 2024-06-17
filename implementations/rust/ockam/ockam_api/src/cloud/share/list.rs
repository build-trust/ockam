use minicbor::{CborLen, Decode, Encode};
use serde::{Deserialize, Serialize};

use super::{InvitationWithAccess, ReceivedInvitation, SentInvitation};

#[derive(Clone, Debug, Encode, Decode, CborLen, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct ListInvitations {
    #[n(1)] pub kind: InvitationListKind,
}

#[derive(Clone, Debug, PartialEq, Decode, Encode, CborLen, Deserialize, Serialize)]
#[cbor(index_only)]
#[rustfmt::skip]
pub enum InvitationListKind {
    #[n(0)] All,
    #[n(1)] Sent,
    #[n(2)] Received,
    #[n(3)] Accepted,
}

#[derive(Clone, Debug, Encode, Decode, CborLen, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct InvitationList {
    #[n(1)] pub sent: Option<Vec<SentInvitation>>,
    #[n(2)] pub received: Option<Vec<ReceivedInvitation>>,
    #[n(3)] pub accepted: Option<Vec<InvitationWithAccess>>,
}
