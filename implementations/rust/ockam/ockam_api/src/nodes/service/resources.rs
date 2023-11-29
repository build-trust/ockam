use ockam_abac::Resource;

pub const INLET: Resource = Resource::assert_inline("tcp-inlet");
pub const OUTLET: Resource = Resource::assert_inline("tcp-outlet");
