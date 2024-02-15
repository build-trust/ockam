use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let args = [
        "--test-argument-parser",
        "enroll",
        "--force",
        "--skip-resource-creation",
    ];
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(args);
    cmd.assert().failure();

    let args = ["--test-argument-parser", "enroll", "--force"];
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(args);
    cmd.assert().success();

    let args = [
        "--test-argument-parser",
        "enroll",
        "--skip-resource-creation",
    ];
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(args);
    cmd.assert().success();

    Ok(())
}
