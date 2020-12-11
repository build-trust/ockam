macro_rules! check_buffer {
    ($buffer:expr) => {
        if $buffer.is_null() {
            return VaultFailErrorKind::InvalidParam(1).into();
        }
    };
    ($buffer:expr, $length:expr) => {
        if $buffer.is_null() {
            return VaultFailErrorKind::InvalidParam(1).into();
        }
        if $length == 0 {
            return VaultFailErrorKind::InvalidParam(2).into();
        }
    };
}
