use example_test_helper::{find_available_port, CmdBuilder, Error};

#[test]
fn run_01_inlet_outlet_one_process() -> Result<(), Error> {
    let port = find_available_port();
    // Spawn example, wait for it to start up
    let runner = CmdBuilder::new(&format!(
        "cargo run --locked --example 01-inlet-outlet 127.0.0.1:{port} ockam.io:80"
    ))
    .spawn()?;
    runner.match_stdout(r"(?i)Starting new processor")?;

    // Run curl and check for a successful run
    let (exitcode, stdout) =
        CmdBuilder::new(&format!("curl -s -L -H \"Host: ockam.io\" http://127.0.0.1:{port}/")).run()?;
    assert_eq!(Some(0), exitcode);
    println!("curl stdout...");
    println!("{stdout}");
    assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}

#[test]
fn run_02_inlet_outlet_separate_processes() -> Result<(), Error> {
    let routing_port = find_available_port();
    let inlet_port = find_available_port();
    // Spawn outlet, wait for it to start up
    let outlet = CmdBuilder::new(&format!(
        "cargo run --locked --example 02-outlet ockam.io:80 {routing_port}"
    ))
    .spawn()?;
    outlet.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Spawn inlet, wait for it to start up
    let inlet = CmdBuilder::new(&format!(
        "cargo run --locked --example 02-inlet 127.0.0.1:{inlet_port} {routing_port}"
    ))
    .spawn()?;
    inlet.match_stdout(r"(?i)Binding \w+ to 127.0.0.1")?;

    // Run curl and check for a successful run
    let (exitcode, stdout) = CmdBuilder::new(&format!(
        "curl -s -L -H \"Host: ockam.io\" http://127.0.0.1:{inlet_port}/"
    ))
    .run()?;
    assert_eq!(Some(0), exitcode);
    println!("curl stdout...");
    println!("{stdout}");
    assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}

#[test]
fn run_03_inlet_outlet_separate_processes_secure_channel() -> Result<(), Error> {
    let routing_port = find_available_port();
    let inlet_port = find_available_port();
    // Spawn outlet, wait for it to start up
    let outlet = CmdBuilder::new(&format!(
        "cargo run --locked --example 03-outlet ockam.io:80 {routing_port}"
    ))
    .spawn()?;
    outlet.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Spawn inlet, wait for it to start up
    let inlet = CmdBuilder::new(&format!(
        "cargo run --locked --example 03-inlet 127.0.0.1:{inlet_port} {routing_port}"
    ))
    .spawn()?;
    inlet.match_stdout(r"(?i)Binding \w+ to 127.0.0.1")?;

    // Run curl and check for a successful run
    let (exitcode, stdout) = CmdBuilder::new(&format!(
        "curl -s -L -H \"Host: ockam.io\" http://127.0.0.1:{inlet_port}/"
    ))
    .run()?;
    assert_eq!(Some(0), exitcode);
    println!("curl stdout...");
    println!("{stdout}");
    assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}

#[test]
// #[ignore]
fn run_04_inlet_outlet_separate_processes_secure_channel_via_ockam_orchestrator() -> Result<(), Error> {
    let port = find_available_port();
    // Spawn outlet, wait for it to start up, grab dynamic forwarding address
    let outlet = CmdBuilder::new("cargo run --locked --example 04-outlet ockam.io:80").spawn()?;
    outlet.match_stdout(r"(?i)RemoteRelay was created on the node")?;
    let fwd_address = outlet.match_stdout(r"(?m)^FWD_(\w+)$")?.swap_remove(0).unwrap();
    println!("Forwarding address: {fwd_address}");

    // // Spawn inlet, wait for it to start up
    // let inlet = CmdBuilder::new(&format!(
    //     "cargo run --locked --example 04-inlet 127.0.0.1:{port} {fwd_address}"
    // ))
    // .spawn()?;
    // inlet.match_stdout(r"(?i)Binding \w+ to 127.0.0.1")?;

    // // // Run curl and check for a successful run
    // let (exitcode, stdout) =
    //     CmdBuilder::new(&format!("curl -s -L -H \"Host: ockam.io\" http://127.0.0.1:{port}/")).run()?;
    // assert_eq!(Some(0), exitcode);
    // println!("curl stdout...");
    // println!("{stdout}");
    // assert!(stdout.to_lowercase().contains("<html"));
    Ok(())
}
