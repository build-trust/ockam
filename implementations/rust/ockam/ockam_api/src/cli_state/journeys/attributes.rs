use crate::journeys::{
    APPLICATION_EVENT_OCKAM_GIT_HASH, APPLICATION_EVENT_OCKAM_HOME, APPLICATION_EVENT_OCKAM_VERSION,
};
use crate::Version;
use gethostname::gethostname;
use opentelemetry::trace::TraceId;
use opentelemetry::Key;
use opentelemetry_sdk::trace::{IdGenerator, RandomIdGenerator};
use std::collections::HashMap;
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

/// Return a host id built from the MAC address and the IP address of the machine
/// If they can be retrieved from the command line. Otherwise return a random trace id
pub(crate) fn make_host_trace_id() -> TraceId {
    let machine = match (get_mac_address(), get_ip_address()) {
        (Some(mac_address), Some(ip_address)) => format!(
            "{}{}",
            mac_address.replace(':', ""),
            ip_address.replace('.', "")
        ),
        _ => gethostname().to_string_lossy().to_string(),
    };

    // take exactly 16 bytes from the machine name
    let mut machine = machine.as_bytes().to_vec();
    // make sure that there are at least 16 bytes
    if machine.len() < 16 {
        machine.extend(std::iter::repeat(1).take(16 - machine.len()));
    };
    if let Ok(truncated) = machine[0..16].try_into() {
        TraceId::from_bytes(truncated)
    } else {
        let random_id_generator = RandomIdGenerator::default();
        random_id_generator.new_trace_id()
    }
}

/// Return the MAC address for the current machine
pub(crate) fn get_mac_address() -> Option<String> {
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
pub(crate) fn get_ip_address() -> Option<String> {
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
