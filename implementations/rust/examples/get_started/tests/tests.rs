use example_test_helper::{CmdBuilder, Error};
use serial_test::serial;

#[test]
#[serial]
fn run_01_node() -> Result<(), Error> {
    // Run 01-node to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --locked --example 01-node").run()?;
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("Goodbye"));
    Ok(())
}

#[test]
#[serial]
fn run_02_worker() -> Result<(), Error> {
    // Run to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --locked --example 02-worker").run()?;
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("Goodbye"));
    Ok(())
}

#[test]
#[serial]
fn run_03_routing() -> Result<(), Error> {
    // Run to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --locked --example 03-routing").run()?;
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("Goodbye"));
    Ok(())
}

#[test]
#[serial]
fn run_03_routing_many_hops() -> Result<(), Error> {
    // Run to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --locked --example 03-routing-many-hops").run()?;
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("Goodbye"));
    Ok(())
}

#[test]
#[serial]
fn run_04_routing_over_transport() -> Result<(), Error> {
    // Launch responder, wait for it to start up
    let resp = CmdBuilder::new("cargo run --locked --example 04-routing-over-transport-responder").spawn()?;
    resp.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Run initiator to completion
    let (exitcode, stdout) =
        CmdBuilder::new("cargo run --locked --example 04-routing-over-transport-initiator").run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}

#[test]
#[serial]
fn run_04_routing_over_transport_two_hops() -> Result<(), Error> {
    // Launch responder, wait for it to start up
    let resp = CmdBuilder::new("cargo run --locked --example 04-routing-over-transport-two-hops-responder").spawn()?;
    resp.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Launch middle, wait for it to start up
    let mid = CmdBuilder::new("cargo run --locked --example 04-routing-over-transport-two-hops-middle").spawn()?;
    mid.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Run initiator to completion
    let (exitcode, stdout) =
        CmdBuilder::new("cargo run --locked --example 04-routing-over-transport-two-hops-initiator").run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}

#[test]
#[serial]
fn run_04_udp() -> Result<(), Error> {
    // Launch responder, wait for it to start up
    let resp = CmdBuilder::new("cargo run --locked --example 04-udp-transport-responder").spawn()?;
    resp.match_stdout(r"(?i)Waiting for incoming UDP datagram")?;

    // Run initiator to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --locked --example 04-udp-transport-initiator").run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}

#[test]
#[serial]
fn run_05_secure_channel_over_two_transport_hops() -> Result<(), Error> {
    // Launch responder, wait for it to start up
    let resp =
        CmdBuilder::new("cargo run --locked --example 05-secure-channel-over-two-transport-hops-responder").spawn()?;
    resp.match_stdout("Initializing ockam processor")?;

    // Launch middle, wait for it to start up
    let mid =
        CmdBuilder::new("cargo run --locked --example 05-secure-channel-over-two-transport-hops-middle").spawn()?;
    mid.match_stdout("Initializing ockam processor")?;

    // Run initiator to completion
    let (exitcode, stdout) =
        CmdBuilder::new("cargo run --locked --example 05-secure-channel-over-two-transport-hops-initiator").run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}

#[test]
#[serial]
fn run_06_credentials_exchange() -> Result<(), Error> {
    // Launch the issuer, wait for it to start up
    let resp = CmdBuilder::new("cargo run --locked --example 06-credentials-exchange-issuer").spawn()?;
    resp.match_stdout("issuer started")?;

    // Launch the server, wait for it to start up
    let resp = CmdBuilder::new("cargo run --locked --example 06-credentials-exchange-server").spawn()?;
    resp.match_stdout("server started")?;

    // Launch the client, wait for the message to be sent and received
    let (exitcode, stdout) = CmdBuilder::new("cargo run --locked --example 06-credentials-exchange-client").run()?;
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("received"));

    Ok(())
}

#[test]
#[serial]
fn run_hello() -> Result<(), Error> {
    let (exitcode, stdout) = CmdBuilder::new("cargo run --locked --example hello").run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("App Received: Hello Ockam!"));
    Ok(())
}

#[test]
#[serial]
fn vault_and_identity() -> Result<(), Error> {
    let (exitcode, stdout) = CmdBuilder::new("cargo run --locked --example vault-and-identities").run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("No more workers left. Goodbye!"));
    Ok(())
}
