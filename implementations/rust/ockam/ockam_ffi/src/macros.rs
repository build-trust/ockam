/// Safety macro which ensures a buffer is not null and not empty.
#[macro_export]
macro_rules! check_buffer {
    ($buffer:expr) => {
        if $buffer.is_null() {
            return Err(FfiError::InvalidParam.into());
        }
    };
    ($buffer:expr, $length:expr) => {
        if $buffer.is_null() || $length == 0 {
            return Err(FfiError::InvalidParam.into());
        }
    };
}
