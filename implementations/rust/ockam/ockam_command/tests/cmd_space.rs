use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let prefix_args = ["--test-argument-parser", "space"];
    let common_args = ["/dnsaddr/localhost/tcp/4000", "-n", "node-name"];

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args)
        .arg("create")
        .arg("space-name")
        .args(common_args)
        .arg("--")
        .arg("extra-user");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("list").args(common_args);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args)
        .arg("show")
        .arg("space-id")
        .args(common_args);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args)
        .arg("delete")
        .arg("space-id")
        .args(common_args);
    cmd.assert().success();

    Ok(())
}
