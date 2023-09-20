use crate::utils::now;
use crate::TimestampInSeconds;
use ockam_core::Result;

pub(crate) enum Ttl {
    CreatedNowWithTtl(TimestampInSeconds),
    FullTimestamps {
        created_at: TimestampInSeconds,
        expires_at: TimestampInSeconds,
    },
}

impl Ttl {
    pub(crate) fn build(self) -> Result<(TimestampInSeconds, TimestampInSeconds)> {
        Ok(match self {
            Ttl::CreatedNowWithTtl(ttl) => {
                let created_at = now()?;
                let expires_at = created_at + ttl;

                (created_at, expires_at)
            }
            Ttl::FullTimestamps {
                created_at,
                expires_at,
            } => (created_at, expires_at),
        })
    }
}
