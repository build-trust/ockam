use crate::bindings::ockam_error_t;

pub fn ockam_error_is_none(error: &ockam_error_t) -> bool {
    error.code == 0
}

pub fn ockam_error_has_error(error: &ockam_error_t) -> bool {
    error.code != 0
}
