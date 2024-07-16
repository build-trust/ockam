use crate::ApiError;
use time::format_description::well_known::Iso8601;
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
