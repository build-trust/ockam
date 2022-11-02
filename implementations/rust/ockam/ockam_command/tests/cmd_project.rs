use anyhow::Result;
use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<()> {
    let prefix_args = ["--test-argument-parser", "project"];

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args)
        .arg("create")
        .arg("space-name")
        .arg("project-name")
        .arg("--")
        .arg("service-a")
        .arg("service-b");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("list");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("show").arg("project-id");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args)
        .arg("delete")
        .arg("space-name")
        .arg("project-id");
    cmd.assert().success();

    Ok(())
}
