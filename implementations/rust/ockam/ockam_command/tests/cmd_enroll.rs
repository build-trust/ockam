use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--dry-run")
        .arg("cloud")
        .arg("enroll")
        .arg("127.0.0.1") // cloud_addr
        .arg("auth0") // authenticator
        .arg("--vault")
        .arg("vt")
        .arg("--identity")
        .arg("idt")
        .arg("--overwrite");
    cmd.assert().success();
    Ok(())
}
