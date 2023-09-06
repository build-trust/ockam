use crate::TimestampInSeconds;

impl core::ops::Deref for TimestampInSeconds {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u64> for TimestampInSeconds {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl core::ops::Add<TimestampInSeconds> for TimestampInSeconds {
    type Output = TimestampInSeconds;

    fn add(self, rhs: TimestampInSeconds) -> Self::Output {
        TimestampInSeconds(self.0 + rhs.0)
    }
}
