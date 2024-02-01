use crate::utils::now;
use crate::TimestampInSeconds;
use ockam_core::Result;

pub(crate) enum Ttl {
    CreatedNowWithTtl(TimestampInSeconds),
    FullTimestamps {
        from: TimestampInSeconds,
        until: TimestampInSeconds,
    },
}

impl Ttl {
    pub(crate) fn build(self) -> Result<(TimestampInSeconds, TimestampInSeconds)> {
        Ok(match self {
            Ttl::CreatedNowWithTtl(ttl) => {
                let from = now()?;
                let until = from + ttl;

                (from, until)
            }
            Ttl::FullTimestamps { from, until } => (from, until),
        })
    }
}
