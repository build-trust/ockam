use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("cloud")
        .arg("enroll")
        .arg("/ip4/127.0.0.1/tcp/8080") // cloud_addr
        .arg("auth0") // authenticator
        .arg("--vault")
        .arg("vt")
        .arg("--identity")
        .arg("idt")
        .arg("--overwrite");
    cmd.assert().success();
    Ok(())
}
