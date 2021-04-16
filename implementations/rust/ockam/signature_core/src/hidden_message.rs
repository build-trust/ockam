use crate::message::Message;
use crate::nonce::Nonce;

/// Two types of hidden messages
#[derive(Copy, Clone, Debug)]
pub enum HiddenMessage {
    /// Indicates the message is hidden and no other work is involved
    ///     so a blinding factor will be generated specific to this proof
    ProofSpecificBlinding(Message),
    /// Indicates the message is hidden but it is involved with other proofs
    ///     like boundchecks, set memberships or inequalities, so the blinding factor
    ///     is provided from an external source.
    ExternalBlinding(Message, Nonce),
}
