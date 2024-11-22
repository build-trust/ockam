use crate::ApiError;
use minicbor::{decode, encode, CborLen, Decode, Decoder, Encode, Encoder};
use serde::ser::Error;
use serde::{Deserialize, Serialize, Serializer};
use std::str::FromStr;
use time::format_description::well_known::Iso8601;
use time::macros::{format_description, offset};
use time::OffsetDateTime;

/// Transform a string that represents an Iso8601 date into a `time::OffsetDateTime`
pub fn parse_date(date: &str) -> ockam_core::Result<OffsetDateTime> {
    // Add the Z timezone to the date, as the `time` crate requires it
    let date = if date.ends_with('Z') {
        date.to_string()
    } else {
        format!("{}Z", date)
    };
    OffsetDateTime::parse(&date, &Iso8601::DEFAULT).map_err(|e| ApiError::core(e.to_string()))
}

/// Check if a string that represents an Iso8601 date is expired, using the `time` crate
pub fn is_expired(date: &str) -> ockam_core::Result<bool> {
    let date = parse_date(date)?;
    let now = OffsetDateTime::now_utc();
    Ok(date < now)
}

#[derive(Clone, Debug, Eq)]
pub struct UtcDateTime(OffsetDateTime);

impl UtcDateTime {
    pub fn new(date: OffsetDateTime) -> crate::Result<Self> {
        if date.offset() != offset!(UTC) {
            return Err(ApiError::General("The date must be in UTC".to_string()));
        }
        Ok(Self(date))
    }

    pub fn now() -> Self {
        Self(OffsetDateTime::now_utc())
    }

    pub fn is_in_the_past(&self) -> bool {
        self.0 < OffsetDateTime::now_utc()
    }

    pub fn is_in_the_future(&self) -> bool {
        !self.is_in_the_past()
    }

    /// If the date is "2024-10-01T00:00:00Z", this will return "10, Dec 2024"
    pub fn format_human(&self) -> Result<String, std::fmt::Error> {
        self.0
            .format(format_description!("[day], [month repr:short] [year]"))
            .map_err(|e| std::fmt::Error::custom(e.to_string()))
    }

    /// Return the number of days in human format. For example, "3 days"
    pub fn diff_human(&self, other: &Self) -> String {
        let diff = self.0 - other.0;
        format!("{} days", diff.whole_days())
    }

    pub fn into_inner(self) -> OffsetDateTime {
        self.0
    }
}

impl TryFrom<OffsetDateTime> for UtcDateTime {
    type Error = ApiError;

    fn try_from(date: OffsetDateTime) -> Result<Self, Self::Error> {
        Self::new(date)
    }
}

impl FromStr for UtcDateTime {
    type Err = ApiError;

    fn from_str(date: &str) -> Result<Self, Self::Err> {
        // Add the Z timezone to the date, as the `time` crate requires it
        let date = if date.ends_with('Z') {
            date.to_string()
        } else {
            format!("{}Z", date)
        };
        Self::new(
            OffsetDateTime::parse(&date, &Iso8601::DEFAULT)
                .map_err(|e| ApiError::core(e.to_string()))?,
        )
    }
}

impl PartialEq for UtcDateTime {
    fn eq(&self, other: &Self) -> bool {
        // Compare the dates using as unix timestamps, using a tolerance of 1 second
        let start_date = self.0.unix_timestamp();
        let other_start_date = other.0.unix_timestamp();
        (start_date - other_start_date).abs() <= 1
    }
}

impl<C> Encode<C> for UtcDateTime {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        self.0.unix_timestamp().encode(e, ctx)
    }
}

impl<C> CborLen<C> for UtcDateTime {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        self.0.unix_timestamp().cbor_len(ctx)
    }
}

impl<'b, C> Decode<'b, C> for UtcDateTime {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let timestamp = d.decode_with(ctx)?;
        let inner =
            OffsetDateTime::from_unix_timestamp(timestamp).map_err(decode::Error::message)?;
        Ok(Self(inner))
    }
}

impl Serialize for UtcDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.unix_timestamp().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for UtcDateTime {
    fn deserialize<D>(deserializer: D) -> Result<UtcDateTime, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let timestamp = i64::deserialize(deserializer)?;
        let inner =
            OffsetDateTime::from_unix_timestamp(timestamp).map_err(serde::de::Error::custom)?;
        Ok(UtcDateTime(inner))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};

    impl Arbitrary for UtcDateTime {
        fn arbitrary(g: &mut Gen) -> Self {
            // Use i32 instead of i64 to avoid using values larger than time::Date::MAX
            UtcDateTime(OffsetDateTime::from_unix_timestamp(i32::arbitrary(g) as i64).unwrap())
        }
    }

    quickcheck! {
        fn utc_date_time(unix_timestamp: i32) -> TestResult {
            let inner = OffsetDateTime::from_unix_timestamp(unix_timestamp as i64).unwrap();
            match UtcDateTime::new(inner) {
                Ok(_) => TestResult::passed(),
                Err(e) => TestResult::error(format!("{e:?}")),
            }
        }
    }

    #[test]
    fn test_utc_date_time() {
        let date = UtcDateTime::from_str("2024-10-01T00:00:00Z").unwrap();
        assert_eq!(date.format_human().unwrap(), "01, Oct 2024");
        let without_timezone = UtcDateTime::from_str("2024-10-01T00:00:00").unwrap();
        assert_eq!(without_timezone, date);
        let from_inner = OffsetDateTime::from_unix_timestamp(date.0.unix_timestamp()).unwrap();
        assert_eq!(UtcDateTime::try_from(from_inner).unwrap(), date);
        // Fail if the date is not in UTC
        assert!(UtcDateTime::from_str("2024-10-01T00:00:00+01:00").is_err());
    }

    #[test]
    fn utc_date_time_cbor_encode_decode() {
        let date = UtcDateTime::from_str("2024-10-01T00:00:00Z").unwrap();
        let mut bytes = Vec::new();
        date.encode(&mut Encoder::new(&mut bytes), &mut ()).unwrap();
        let decoded = UtcDateTime::decode(&mut Decoder::new(&bytes), &mut ()).unwrap();
        assert_eq!(decoded, date);
    }

    #[test]
    fn utc_date_time_serde() {
        let date = UtcDateTime::from_str("2024-10-01T00:00:00Z").unwrap();
        let serialized = serde_json::to_string(&date).unwrap();
        let deserialized: UtcDateTime = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, date);
    }
}
