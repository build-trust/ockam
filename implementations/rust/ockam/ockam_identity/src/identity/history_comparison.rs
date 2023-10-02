/// Result of comparison of current `IdentityChangeHistory` to the `IdentityChangeHistory`
/// of the same Identity, that was known to us earlier
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityHistoryComparison {
    /// No difference
    Equal,
    /// Some changes don't match between current identity and known identity
    Conflict,
    /// Current identity is more recent than known identity
    Newer,
    /// Known identity is more recent
    Older,
}
