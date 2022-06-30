use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    // email
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("enroll")
        .arg("/ip4/127.0.0.1/tcp/8080")
        .arg("--vault")
        .arg("vt")
        .arg("--identity")
        .arg("idt")
        .arg("--overwrite");
    cmd.assert().success();

    // auth0
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("enroll")
        .arg("/ip4/127.0.0.1/tcp/8080")
        .arg("--vault")
        .arg("vt")
        .arg("--identity")
        .arg("idt")
        .arg("--overwrite")
        .arg("--auth0");
    cmd.assert().success();

    // token
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("enroll")
        .arg("/ip4/127.0.0.1/tcp/8080")
        .arg("--vault")
        .arg("vt")
        .arg("--identity")
        .arg("idt")
        .arg("--overwrite")
        .arg("--token")
        .arg("token-value");
    cmd.assert().success();

    Ok(())
}
