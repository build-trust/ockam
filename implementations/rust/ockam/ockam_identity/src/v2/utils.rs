use super::super::v2::models::TimestampInSeconds;
use ockam_core::Result;

/// Create a new timestamp using the system time
#[cfg(feature = "std")]
pub fn now() -> Result<TimestampInSeconds> {
    if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(TimestampInSeconds::new(now.as_secs()))
    } else {
        Err(super::IdentityError::UnknownTimestamp.into())
    }
}

/// Create a new timestamp using the system time
#[cfg(not(feature = "std"))]
pub fn now() -> Result<TimestampInSeconds> {
    Err(super::IdentityError::UnknownTimestamp.into())
}
