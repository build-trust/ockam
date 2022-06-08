use example_test_helper::{CmdBuilder, Error};
use serial_test::serial;

#[test]
#[serial] // Serialise tests that may clash on TCP ports
fn run_01_inlet_outlet_one_process() -> Result<(), Error> {
    // Spawn example, wait for it to start up
    let runner = CmdBuilder::new("cargo run --example 01-inlet-outlet 127.0.0.1:4001 ockam.io:80").spawn()?;
    runner.match_stdout(r"(?i)Starting new processor")?;

    // Run curl and check for a successful run
    let (exitcode, stdout) = CmdBuilder::new("curl -s -H \"Host: ockam.io\" http://127.0.0.1:4001/").run()?;
    assert_eq!(Some(0), exitcode);
    println!("curl stdout...");
    println!("{stdout}");
    assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}

#[test]
#[serial] // Serialise tests that may clash on TCP ports
fn run_02_inlet_outlet_seperate_processes() -> Result<(), Error> {
    // Spawn outlet, wait for it to start up
    let outlet = CmdBuilder::new("cargo run --example 02-outlet ockam.io:80").spawn()?;
    outlet.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Spawn inlet, wait for it to start up
    let inlet = CmdBuilder::new("cargo run --example 02-inlet 127.0.0.1:4001").spawn()?;
    inlet.match_stdout(r"(?i)Binding \w+ to 127.0.0.1:4001")?;

    // Run curl and check for a successful run
    let (exitcode, stdout) = CmdBuilder::new("curl -s -H \"Host: ockam.io\" http://127.0.0.1:4001/").run()?;
    assert_eq!(Some(0), exitcode);
    println!("curl stdout...");
    println!("{stdout}");
    assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}

#[test]
#[serial] // Serialise tests that may clash on TCP ports
fn run_03_inlet_outlet_seperate_processes_secure_channel() -> Result<(), Error> {
    // Spawn outlet, wait for it to start up
    let outlet = CmdBuilder::new("cargo run --example 03-outlet ockam.io:80").spawn()?;
    outlet.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Spawn inlet, wait for it to start up
    let inlet = CmdBuilder::new("cargo run --example 03-inlet 127.0.0.1:4001").spawn()?;
    inlet.match_stdout(r"(?i)Binding \w+ to 127.0.0.1:4001")?;

    // Run curl and check for a successful run
    let (exitcode, stdout) = CmdBuilder::new("curl -s -H \"Host: ockam.io\" http://127.0.0.1:4001/").run()?;
    assert_eq!(Some(0), exitcode);
    println!("curl stdout...");
    println!("{stdout}");
    assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}

#[test]
#[serial] // Serialise tests that may clash on TCP ports
fn run_04_inlet_outlet_seperate_processes_secure_channel_via_ockam_hub() -> Result<(), Error> {
    // Spawn outlet, wait for it to start up, grab dynamic forwarding address
    let outlet = CmdBuilder::new("cargo run --example 04-outlet ockam.io:80").spawn()?;
    outlet.match_stdout(r"(?i)RemoteForwarder was created on the node")?;
    let fwd_address = outlet.match_stdout(r"(?m)^FWD_(\w+)$")?.swap_remove(0).unwrap();
    println!("Forwarding address: {fwd_address}");

    // Spawn inlet, wait for it to start up
    let inlet = CmdBuilder::new(&format!("cargo run --example 04-inlet 127.0.0.1:4001 {fwd_address}")).spawn()?;
    inlet.match_stdout(r"(?i)Binding \w+ to 127.0.0.1:4001")?;

    // // Run curl and check for a successful run
    let (exitcode, stdout) = CmdBuilder::new("curl -s -H \"Host: ockam.io\" http://127.0.0.1:4001/").run()?;
    assert_eq!(Some(0), exitcode);
    println!("curl stdout...");
    println!("{stdout}");
    assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}
