use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let prefix_args = ["--test-argument-parser", "project"];

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args)
        .arg("create")
        .arg("space-id")
        .arg("project-name")
        .arg("service-a service-b")
        .arg("--")
        .arg("/ip4/127.0.0.1/tcp/8080");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("list").arg("space-id");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args)
        .arg("show")
        .arg("space-id")
        .arg("project-id")
        .arg("/ip4/127.0.0.1/tcp/8080");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args)
        .arg("delete")
        .arg("space-id")
        .arg("project-id")
        .arg("/ip4/127.0.0.1/tcp/8080");
    cmd.assert().success();

    Ok(())
}
