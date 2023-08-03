use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let prefix_args = ["--test-argument-parser", "project"];

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(prefix_args)
        .arg("create")
        .arg("space-name")
        .arg("project-name");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(prefix_args).arg("list");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(prefix_args).arg("version");
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
    let enrollment_ticket = include_str!("./fixtures/user.enrollment.ticket").trim();
    cmd.args(prefix_args).arg("enroll").arg(enrollment_ticket);
    cmd.assert().success();

    Ok(())
}
