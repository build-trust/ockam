use minicbor::{Decode, Encode};

/// Result of comparison of current `IdentityChangeHistory` to the `IdentityChangeHistory`
/// of the same Identity, that was known to us earlier
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum IdentityHistoryComparison {
    /// No difference
    #[n(1)] Equal,
    /// Some changes don't match between current identity and known identity
    #[n(2)] Conflict,
    /// Current identity is more recent than known identity
    #[n(3)] Newer,
    /// Known identity is more recent
    #[n(4)] Older,
}
