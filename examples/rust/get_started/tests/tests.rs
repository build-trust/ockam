use example_test_helper::{CmdBuilder, Error};
use serial_test::serial;

#[test]
fn run_01_node() -> Result<(), Error> {
    // Run 01-node to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --example 01-node").run()?;
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("Goodbye"));
    Ok(())
}

#[test]
fn run_02_worker() -> Result<(), Error> {
    // Run to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --example 02-worker").run()?;
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("Goodbye"));
    Ok(())
}

#[test]
fn run_03_routing() -> Result<(), Error> {
    // Run to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --example 03-routing").run()?;
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("Goodbye"));
    Ok(())
}

#[test]
fn run_03_routing_many_hops() -> Result<(), Error> {
    // Run to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --example 03-routing-many-hops").run()?;
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("Goodbye"));
    Ok(())
}

#[test]
#[serial] // Serialise tests that may clash on TCP ports
fn run_04_routing_over_transport() -> Result<(), Error> {
    // Launch responder, wait for it to start up
    let resp = CmdBuilder::new("cargo run --example 04-routing-over-transport-responder").spawn()?;
    resp.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Run initiator to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --example 04-routing-over-transport-initiator").run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}

#[test]
#[serial] // Serialise tests that may clash on TCP ports
fn run_04_routing_over_transport_two_hops() -> Result<(), Error> {
    // Launch responder, wait for it to start up
    let resp = CmdBuilder::new("cargo run --example 04-routing-over-transport-two-hops-responder").spawn()?;
    resp.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Launch middle, wait for it to start up
    let mid = CmdBuilder::new("cargo run --example 04-routing-over-transport-two-hops-middle").spawn()?;
    mid.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Run initiator to completion
    let (exitcode, stdout) =
        CmdBuilder::new("cargo run --example 04-routing-over-transport-two-hops-initiator").run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}
