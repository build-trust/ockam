use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("enroll")
        .arg("/ip4/127.0.0.1/tcp/8080") // address
        .arg("auth0") // authenticator
        .arg("--vault")
        .arg("vt")
        .arg("--identity")
        .arg("idt")
        .arg("--overwrite");
    cmd.assert().success();
    Ok(())
}
