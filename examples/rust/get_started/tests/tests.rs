use example_test_helper::{CmdBuilder, Error};

// Avoid TCP port clashes when tests run in parallel
const PORT_RUN_04_ROUTING_OVER_TRANSPORT: u32 = 4000;
const PORT_MIDDLE_RUN_04_ROUTING_OVER_TRANSPORT_TWO_HOPS: u32 = 4001;
const PORT_RESPONDER_RUN_04_ROUTING_OVER_TRANSPORT_TWO_HOPS: u32 = 4002;

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
fn run_04_routing_over_transport() -> Result<(), Error> {
    // Launch responder, wait for it to start up
    let resp = CmdBuilder::new(&format!(
        "cargo run --example 04-routing-over-transport-responder {PORT_RUN_04_ROUTING_OVER_TRANSPORT}"
    ))
    .spawn()?;
    resp.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Run initiator to completion
    let (exitcode, stdout) = CmdBuilder::new(&format!(
        "cargo run --example 04-routing-over-transport-initiator {PORT_RUN_04_ROUTING_OVER_TRANSPORT}",
    ))
    .run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}

#[test]
fn run_04_routing_over_transport_two_hops() -> Result<(), Error> {
    // Launch responder, wait for it to start up
    let resp = CmdBuilder::new(&format!(
        "cargo run --example 04-routing-over-transport-two-hops-responder {PORT_RESPONDER_RUN_04_ROUTING_OVER_TRANSPORT_TWO_HOPS}"
    ))
    .spawn()?;
    resp.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Launch middle, wait for it to start up
    let mid = CmdBuilder::new(&format!(
        "cargo run --example 04-routing-over-transport-two-hops-middle {PORT_MIDDLE_RUN_04_ROUTING_OVER_TRANSPORT_TWO_HOPS}"
    ))
    .spawn()?;
    mid.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Run initiator to completion
    let (exitcode, stdout) =
        CmdBuilder::new(&format!(
            "cargo run --example 04-routing-over-transport-two-hops-initiator {PORT_MIDDLE_RUN_04_ROUTING_OVER_TRANSPORT_TWO_HOPS} {PORT_RESPONDER_RUN_04_ROUTING_OVER_TRANSPORT_TWO_HOPS}"
        ))
        .run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}
