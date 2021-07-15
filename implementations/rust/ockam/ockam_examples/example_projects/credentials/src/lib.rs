use std::io::stdin;

pub const OFFICE_TCP_ADDRESS: &str = "127.0.0.1:4222";
pub const OFFICE_LISTENER_ADDRESS: &str = "office_listener";
pub const OFFICE_ISSUER_ADDRESS: &str = "office_issuer";
pub const DOOR_TCP_ADDRESS: &str = "127.0.0.1:5333";
pub const DOOR_LISTENER_ADDRESS: &str = "door_listener";
pub const DOOR_WORKER_ADDRESS: &str = "door_verifier";

pub fn read_line() -> String {
    let mut line = String::new();
    stdin().read_line(&mut line).unwrap();
    line.replace(&['\n', '\r'][..], "")
}
