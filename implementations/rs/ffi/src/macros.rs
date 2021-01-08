macro_rules! check_buffer {
    ($buffer:expr) => {
        if $buffer.is_null() {
            return Error::InvalidParam.into();
        }
    };
    ($buffer:expr, $length:expr) => {
        if $buffer.is_null() || $length == 0 {
            return Error::InvalidParam.into();
        }
    };
}
