use example_test_helper::{CmdBuilder, Error};
use serial_test::serial;

#[test]
#[serial] // Serialise tests that may clash on TCP ports
fn run_01_access_control_for_transport() -> Result<(), Error> {
    // Launch responder, wait for it to start up
    let resp =
        CmdBuilder::new("cargo run --example 01-access-control-for-transport-responder").spawn()?;
    resp.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Run initiator to completion
    let (exitcode, stdout) =
        CmdBuilder::new("cargo run --example 01-access-control-for-transport-initiator").run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}

#[test]
fn run_02_abac_in_place() -> Result<(), Error> {
    // Run to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --example 02-abac-in-place").run()?;
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("Goodbye"));
    Ok(())
}

#[test]
fn run_03_abac_workers() -> Result<(), Error> {
    // Run to completion
    let (exitcode, stdout) = CmdBuilder::new("cargo run --example 03-abac-workers").run()?;
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("Goodbye"));
    Ok(())
}

#[test]
#[serial] // Serialise tests that may clash on TCP ports
fn run_04_abac_for_transport() -> Result<(), Error> {
    // Launch responder, wait for it to start up
    let resp = CmdBuilder::new("cargo run --example 04-abac-for-transport-responder").spawn()?;
    resp.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Run initiator to completion
    let (exitcode, stdout) =
        CmdBuilder::new("cargo run --example 04-abac-for-transport-initiator").run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}
