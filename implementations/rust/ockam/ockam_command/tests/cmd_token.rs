use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("token")
        .arg("/ip4/127.0.0.1/tcp/8080")
        .arg("--vault")
        .arg("vt")
        .arg("--identity")
        .arg("idt")
        .arg("--overwrite")
        .arg("--")
        .arg("k1=v1")
        .arg("k2=v2");
    cmd.assert().success();

    Ok(())
}
