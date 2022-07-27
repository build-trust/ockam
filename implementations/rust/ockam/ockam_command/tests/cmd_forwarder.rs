use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("forwarder")
        .arg("create")
        .arg("/ip4/127.0.0.1/tcp/8080")
        .arg("forwarder_alias")
        .arg("-n")
        .arg("node-name");
    cmd.assert().success();

    Ok(())
}
