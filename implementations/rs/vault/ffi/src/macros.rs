macro_rules! check_buffer {
    ($buffer:expr, $error:expr) => {
        if $buffer.is_null() {}
    };
    ($buffer:expr, $length:expr, $error:expr) => {
        if $buffer.is_null() || $length == 0 {
            *$error = Error::InvalidParam.into();
            return;
        }
    };
}
