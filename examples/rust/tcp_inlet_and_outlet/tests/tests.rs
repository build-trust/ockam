use example_test_helper::{CmdBuilder, Error};

// Avoid tests clashing on TCP ports when run in parallel
const RUN_01_PORT: u32 = 4000;
const RUN_02_INLET_PORT: u32 = 4001;
const RUN_02_ROUTING_PORT: u32 = 4002;
const RUN_03_INLET_PORT: u32 = 4003;
const RUN_03_ROUTING_PORT: u32 = 4004;
const RUN_04_INLET_PORT: u32 = 4005;

#[test]
fn run_01_inlet_outlet_one_process() -> Result<(), Error> {
    // Spawn example, wait for it to start up
    let runner = CmdBuilder::new(&format!(
        "cargo run --example 01-inlet-outlet 127.0.0.1:{RUN_01_PORT} ockam.io:80"
    ))
    .spawn()?;
    runner.match_stdout(r"(?i)Starting new processor")?;

    // Run curl and check for a successful run
    let (exitcode, stdout) = CmdBuilder::new(&format!(
        "curl -s -L -H \"Host: ockam.io\" http://127.0.0.1:{RUN_01_PORT}/"
    ))
    .run()?;
    assert_eq!(Some(0), exitcode);
    println!("curl stdout...");
    println!("{stdout}");
    assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}

#[test]
fn run_02_inlet_outlet_seperate_processes() -> Result<(), Error> {
    // Spawn outlet, wait for it to start up
    let outlet = CmdBuilder::new(&format!(
        "cargo run --example 02-outlet ockam.io:80 {RUN_02_ROUTING_PORT}"
    ))
    .spawn()?;
    outlet.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Spawn inlet, wait for it to start up
    let inlet = CmdBuilder::new(&format!(
        "cargo run --example 02-inlet 127.0.0.1:{RUN_02_INLET_PORT} {RUN_02_ROUTING_PORT}"
    ))
    .spawn()?;
    inlet.match_stdout(r"(?i)Binding \w+ to 127.0.0.1")?;

    // Run curl and check for a successful run
    let (exitcode, stdout) = CmdBuilder::new(&format!(
        "curl -s -L -H \"Host: ockam.io\" http://127.0.0.1:{RUN_02_INLET_PORT}/"
    ))
    .run()?;
    assert_eq!(Some(0), exitcode);
    println!("curl stdout...");
    println!("{stdout}");
    assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}

#[test]
fn run_03_inlet_outlet_seperate_processes_secure_channel() -> Result<(), Error> {
    // Spawn outlet, wait for it to start up
    let outlet = CmdBuilder::new(&format!(
        "cargo run --example 03-outlet ockam.io:80 {RUN_03_ROUTING_PORT}"
    ))
    .spawn()?;
    outlet.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Spawn inlet, wait for it to start up
    let inlet = CmdBuilder::new(&format!(
        "cargo run --example 03-inlet 127.0.0.1:{RUN_03_INLET_PORT} {RUN_03_ROUTING_PORT}"
    ))
    .spawn()?;
    inlet.match_stdout(r"(?i)Binding \w+ to 127.0.0.1")?;

    // Run curl and check for a successful run
    let (exitcode, stdout) = CmdBuilder::new(&format!(
        "curl -s -L -H \"Host: ockam.io\" http://127.0.0.1:{RUN_03_INLET_PORT}/"
    ))
    .run()?;
    assert_eq!(Some(0), exitcode);
    println!("curl stdout...");
    println!("{stdout}");
    assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}

#[test]
fn run_04_inlet_outlet_seperate_processes_secure_channel_via_ockam_hub() -> Result<(), Error> {
    // Spawn outlet, wait for it to start up, grab dynamic forwarding address
    let outlet = CmdBuilder::new("cargo run --example 04-outlet ockam.io:80").spawn()?;
    outlet.match_stdout(r"(?i)RemoteForwarder was created on the node")?;
    let fwd_address = outlet.match_stdout(r"(?m)^FWD_(\w+)$")?.swap_remove(0).unwrap();
    println!("Forwarding address: {fwd_address}");

    // Spawn inlet, wait for it to start up
    let inlet = CmdBuilder::new(&format!(
        "cargo run --example 04-inlet 127.0.0.1:{RUN_04_INLET_PORT} {fwd_address}"
    ))
    .spawn()?;
    inlet.match_stdout(r"(?i)Binding \w+ to 127.0.0.1")?;

    // // Run curl and check for a successful run
    let (exitcode, stdout) = CmdBuilder::new(&format!(
        "curl -s -L -H \"Host: ockam.io\" http://127.0.0.1:{RUN_04_INLET_PORT}/"
    ))
    .run()?;
    assert_eq!(Some(0), exitcode);
    println!("curl stdout...");
    println!("{stdout}");
    assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}
