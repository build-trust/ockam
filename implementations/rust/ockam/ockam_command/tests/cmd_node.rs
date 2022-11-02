use anyhow::Result;
use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<()> {
    // show node success
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("node")
        .arg("show")
        .arg("node-name");
    cmd.assert().success();

    // delete node success
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.arg("--test-argument-parser")
        .arg("node")
        .arg("delete")
        .arg("node-name");
    cmd.assert().success();

    Ok(())
}
