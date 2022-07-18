use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let prefix_args = [
        "--test-argument-parser",
        "invitation",
        "/dnsaddr/localhost/tcp/4000",
        "-a",
        "node-name",
    ];

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args)
        .arg("create")
        .arg("space-id")
        .arg("invitee@test.com");
    cmd.assert().success();

    // With custom cloud address
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args)
        .arg("create")
        .arg("space-id")
        .arg("invitee@test.com");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("list");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("accept").arg("invitation-id");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("reject").arg("invitation-id");
    cmd.assert().success();

    Ok(())
}
