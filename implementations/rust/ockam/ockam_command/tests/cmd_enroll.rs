use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn valid_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let prefix_args = [
        "--test-argument-parser",
        "enroll",
        "--addr",
        "/dnsaddr/localhost/tcp/4000",
        "-a",
        "node-name",
    ];

    // email
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args);
    cmd.assert().success();

    // auth0
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("--auth0");
    cmd.assert().success();

    // token
    let mut cmd = Command::cargo_bin("ockam")?;
    cmd.args(&prefix_args).arg("--token").arg("token-value");
    cmd.assert().success();

    Ok(())
}
