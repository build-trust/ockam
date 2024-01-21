use assert_cmd::prelude::*;
use std::process::Command;
use proptest::prelude::*;
use assert_cmd::Command;
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    fn test_authenticated_get(ip in "127.0.0.1", port in 1025u16..65535u16, id in "[A-Fa-f0-9]{64}") {
        let endpoint = format!("/ip4/{}/tcp/{}", ip, port);
        let mut cmd = Command::cargo_bin("ockam").unwrap();
        cmd.arg("--test-argument-parser")
            .arg("authenticated")
            .arg("get")
            .arg(endpoint)
            .arg("--id")
            .arg(id);
        cmd.assert().success();
    }
    #[test]
    fn test_authenticated_list(ip in "127.0.0.1", port in 1025u16..65535u16) {
        let endpoint = format!("/ip4/{}/tcp/{}", ip, port);
        let mut cmd = Command::cargo_bin("ockam").unwrap();
        cmd.arg("--test-argument-parser")
            .arg("authenticated")
            .arg("list")
            .arg(endpoint);
        cmd.assert().success();
    }
}