/// The maximum messages allowed without an allocator
pub const MAX_MSGS: usize = 128;
/// The number of bytes in a commitment
pub const COMMITMENT_BYTES: usize = 48;
/// The number of bytes in a challenge or nonce
pub const FIELD_BYTES: usize = 32;

/// Allocate message
pub const ALLOC_MSG: &str = "allocate more space";
