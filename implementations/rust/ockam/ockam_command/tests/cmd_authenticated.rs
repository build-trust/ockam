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
        .arg("Ifa804b7fca12a19eed206ae180b5b576860ae651");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("authenticated")
        .arg("list")
        .arg("/ip4/127.0.0.1/tcp/8080");
    cmd.assert().success();

    Ok(())
}
