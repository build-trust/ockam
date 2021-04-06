/// The maximum messages allowed without an allocator
pub const MAX_MSGS: usize = 128;
/// The number of bytes in a commitment
pub const COMMITMENT_BYTES: usize = 48;
/// The number of bytes in a challenge or nonce
pub const FIELD_BYTES: usize = 32;

/// Allocate message
pub(crate) const ALLOC_MSG: &'static str = "allocate more space";
