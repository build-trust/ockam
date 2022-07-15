use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("authenticated")
        .arg("get")
        .arg("/ip4/127.0.0.1/tcp/8080")
        .arg("--id")
        .arg("identifier")
        .arg("key");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("authenticated")
        .arg("del")
        .arg("/ip4/127.0.0.1/tcp/8080")
        .arg("--id")
        .arg("identifier")
        .arg("key");
    cmd.assert().success();

    Ok(())
}
