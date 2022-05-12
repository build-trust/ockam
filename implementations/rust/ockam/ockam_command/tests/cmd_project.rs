use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let prefix_args = ["--dry-run", "cloud", "project"];

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("create").arg("p1"); // project_name
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("list");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("show").arg("p1");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("delete").arg("p1");
    cmd.assert().success();

    Ok(())
}
