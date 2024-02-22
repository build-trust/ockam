use crate::journeys::{
    APPLICATION_EVENT_OCKAM_GIT_HASH, APPLICATION_EVENT_OCKAM_HOME, APPLICATION_EVENT_OCKAM_VERSION,
};
use crate::Version;
use chrono::{DateTime, Datelike, Utc};
use gethostname::gethostname;
use opentelemetry::trace::{SpanId, TraceId};
use opentelemetry::Key;
use opentelemetry_sdk::trace::{IdGenerator, RandomIdGenerator};
use std::collections::HashMap;
use std::fmt::Write;
use std::process::Command;

/// This function returns the default attributes to set on each user event: OCKAM_HOME, `ockam` version and git hash
pub fn default_attributes<'a>() -> HashMap<&'a Key, String> {
    let mut attributes = HashMap::new();
    let ockam_home = std::env::var("OCKAM_HOME").unwrap_or("OCKAM_HOME not set".to_string());
    attributes.insert(APPLICATION_EVENT_OCKAM_HOME, ockam_home);
    attributes.insert(
        APPLICATION_EVENT_OCKAM_VERSION,
        Version::crate_version().to_string(),
    );
    attributes.insert(
        APPLICATION_EVENT_OCKAM_GIT_HASH,
        Version::git_hash().to_string(),
    );
    attributes
}

/// Return the trace id for a host journey:
///
///  - The first character encode the format version
///  - The next 25 characters identify the host
///  - The last 6 characters are the 'now' date as YYMMDD
///
pub(crate) fn make_host_trace_id(now: DateTime<Utc>) -> TraceId {
    let mut machine = make_host();
    // make sure that there exactly 25 characters
    if machine.len() < 25 {
        machine.extend(std::iter::repeat("1").take(25 - machine.len()));
    };
    machine = machine[0..25].to_string();

    // date as a 6 characters string
    let now = now_as_string(now);

    // trace_id as a 32 characters hex string = 1 + 25 + 6
    // the digit 1 is present at the beginning as a version indicator, in case we need to evolve the format
    // We also append the date in order to roll the host traces every few days
    trace_id_from_hex(format!("1{machine}{now}").as_str())
}

/// Return the trace id for a project journey:
///
///  - The first character encode the format version
///  - The next 25 characters identify the project, based on the project id
///  - The last 6 characters are the 'now' date as YYMMDD
///    The date day is rounded to the nearest multiple of 5. For example 240220, then 240225, 240301, 240305, etc...
///    This allows to bucket all the spans in the same trace, even if the spans come from  different machines which
///    can start their own project journey trace independently.
///
pub(crate) fn make_project_trace_id(project_id: &str, now: DateTime<Utc>) -> TraceId {
    // take the whole project without '-' as the base for the trace id
    let project_id_trace_id = &project_id.replace('-', "")[0..25];

    // trace_id as a 32 characters hex string = 1 + 25 + 6
    // The digit 1 is present at the beginning as a version indicator, in case we need to evolve the format
    // We also append the date in order to roll the project traces every few days
    trace_id_from_hex(format!("1{}{}", project_id_trace_id, now_as_string(now)).as_str())
}

/// Create the top-level span_id for a journey, based on its trace_id
/// We use the last 16 characters so that the span id contains the date
/// that is incorporated in the trace id and ends-up being unique.
pub(crate) fn make_journey_span_id(trace_id: TraceId) -> SpanId {
    let trace_id = trace_id.to_string();
    let length = trace_id.len();
    match SpanId::from_hex(&trace_id[length - 16..length]) {
        Ok(span_id) => span_id,
        _ => {
            let random_id_generator = RandomIdGenerator::default();
            random_id_generator.new_span_id()
        }
    }
}

/// Return a string containing enough data to uniquely identify a host
/// That string is hexadecimal string which can be part of a trace id
pub(crate) fn make_host() -> String {
    let host = match (get_mac_address(), get_ip_address()) {
        (Some(mac_address), Some(ip_address)) => format!(
            "{}{}",
            mac_address.replace(':', ""),
            ip_address.replace('.', "")
        ),
        _ => gethostname().to_string_lossy().to_string(),
    };
    convert_to_hex(&host)
}

// Check if the string is already in hexadecimal format
// If it is already hexadecimal, return it as is, otherwise convert it to hex
fn convert_to_hex(s: &str) -> String {
    let is_hex = s.chars().all(|c| c.is_ascii_hexdigit());

    if is_hex {
        s.to_string()
    } else {
        s.bytes().fold(String::new(), |mut output, b| {
            let _ = write!(output, "{b:02x}");
            output
        })
    }
}

/// Parse an hex string to a TraceId and generate a random one in case of a parsing error
fn trace_id_from_hex(trace_id: &str) -> TraceId {
    match TraceId::from_hex(trace_id) {
        Ok(trace_id) => trace_id,
        Err(_) => {
            let random_id_generator = RandomIdGenerator::default();
            random_id_generator.new_trace_id()
        }
    }
}

/// Return a string formatted as YYMMDD
/// and rounded to the near multiple of 5 after DD=05
fn now_as_string(now: DateTime<Utc>) -> String {
    let year = now.year() - 2000;
    let month = now.month();
    // round the day to the closest multiple of 5
    // so the days end-up being 1, 5, 10, 15, 20, 25, 30
    let today = now.day();
    let day = if today < 5 { 1 } else { (today / 5) * 5 };

    format!("{:02}{:02}{:02}", year, month, day)
}

/// Return the MAC address for the current machine
fn get_mac_address() -> Option<String> {
    let output = Command::new("ifconfig").output().ok()?;
    let output_str = String::from_utf8_lossy(&output.stdout);

    let mut result = None;
    for line in output_str.lines() {
        // Check if the line contains "ether" which is typically followed by MAC address in ifconfig output
        if line.contains("ether") {
            // Extract MAC address substring
            let split = line.split(' ').collect::<Vec<_>>();
            let mac_address = split.get(1)?;
            result = Some(mac_address.to_string());
            break;
        }
    }
    result
}

/// Return the IP address for the current machine
fn get_ip_address() -> Option<String> {
    let output = Command::new("ifconfig").output().ok()?;
    let output_str = String::from_utf8_lossy(&output.stdout);

    let mut result = None;
    for line in output_str.lines() {
        // Check if the line contains "inet" which is typically followed by IP address in ifconfig output
        //  - skip the localhost interface
        //  - note the space after inet to catch the IPv4 address and not v6
        if line.contains("inet ") && !line.contains("127.0.0.1") {
            // Extract IP address substring
            let split = line.split(' ').collect::<Vec<_>>();
            let ip_address = split.get(1)?;
            result = Some(ip_address.to_string());
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_make_host_trace_id_ends_with_the_current_date() {
        let trace_id = make_host_trace_id(datetime("2024-02-22T12:00:00Z")).to_string();
        let length = trace_id.len();
        // the day is rounded to the closest multiple of 5
        assert_eq!(
            &trace_id[length - 6..length],
            "240220",
            "the current host is {} = mac {:?} / ip {:?} / hostname {:?}",
            make_host(),
            get_mac_address(),
            get_ip_address(),
            gethostname().to_string_lossy().to_string()
        );
    }

    #[test]
    fn test_make_host_journey_span_id_is_end_of_trace_id() {
        let trace_id = make_host_trace_id(datetime("2024-02-22T12:00:00Z"));
        let span_id = make_journey_span_id(trace_id);
        assert!(trace_id.to_string().ends_with(span_id.to_string().as_str()));
    }

    #[test]
    fn test_make_project_trace_id_contains_part_of_the_project_id_and_current_date() {
        let trace_id = make_project_trace_id(
            "8a12dc0e-d48b-4da1-925d-cda822505348",
            datetime("2024-02-22T12:00:00Z"),
        )
        .to_string();
        // the day is rounded to the closest multiple of 5
        assert_eq!(
            trace_id, "18a12dc0ed48b4da1925dcda82240220",
            "the trace id {trace_id} is incorrect"
        );
    }

    #[test]
    fn test_make_project_span_id_is_end_of_trace_id() {
        let trace_id = make_project_trace_id(
            "8a12dc0e-d48b-4da1-925d-cda822505348",
            datetime("2024-02-22T12:00:00Z"),
        );
        let span_id = make_journey_span_id(trace_id);
        assert!(trace_id.to_string().ends_with(span_id.to_string().as_str()));
    }

    #[test]
    fn test_now_as_string() {
        // days are rounded to the lower multiple of 5 after 5
        assert_eq!(now_as_string(datetime("2024-02-01T12:00:00Z")), "240201");
        assert_eq!(now_as_string(datetime("2024-02-04T12:00:00Z")), "240201");
        assert_eq!(now_as_string(datetime("2024-02-05T12:00:00Z")), "240205");
        assert_eq!(now_as_string(datetime("2024-02-07T12:00:00Z")), "240205");
        assert_eq!(now_as_string(datetime("2024-02-09T12:00:00Z")), "240205");
        assert_eq!(now_as_string(datetime("2024-02-10T12:00:00Z")), "240210");
        assert_eq!(now_as_string(datetime("2024-03-31T12:00:00Z")), "240330");
    }

    /// HELPERS
    fn datetime(s: &str) -> DateTime<Utc> {
        Utc.from_utc_datetime(&DateTime::parse_from_rfc3339(s).unwrap().naive_utc())
    }
}
