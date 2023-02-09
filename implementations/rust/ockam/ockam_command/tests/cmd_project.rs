use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let prefix_args = ["--test-argument-parser", "project"];

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(prefix_args)
        .arg("create")
        .arg("space-name")
        .arg("project-name")
        .arg("--")
        .arg("service-a")
        .arg("service-b");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(prefix_args).arg("list");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(prefix_args).arg("show").arg("project-id");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(prefix_args)
        .arg("delete")
        .arg("space-name")
        .arg("project-id");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(prefix_args)
        .arg("authenticate")
        .arg("--token")
        .arg("02043d7bc316467b25b8df7118f4d1ba4b1911284236a3f94d8017ac7faff625");
    cmd.assert().success();

    Ok(())
}
