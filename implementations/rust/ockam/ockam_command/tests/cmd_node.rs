use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    // show node success
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("node")
        .arg("show")
        .arg("node-name");
    cmd.assert().success();

    // show node failure
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("node")
        .arg("show")
        .arg("--api-node")
        .arg("node-name");
    cmd.assert().failure();

    // delete node success
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("node")
        .arg("delete")
        .arg("node-name");
    cmd.assert().success();

    // delete node failure
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("node")
        .arg("delete")
        .arg("--api-node")
        .arg("node-name");
    cmd.assert().failure();
    Ok(())
}
