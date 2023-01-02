use example_test_helper::{CmdBuilder, Error};
use rand::Rng;

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

// TODO: Uncomment me when we manage to solve ports collision problems during CI runs
// #[test]
// fn run_04_routing_over_transport() -> Result<(), Error> {
//     let rand_port = rand::thread_rng().gen_range(10000..65535);
//     // Launch responder, wait for it to start up
//     let resp = CmdBuilder::new(&format!(
//         "cargo run --example 04-routing-over-transport-responder {rand_port}"
//     ))
//     .spawn()?;
//     resp.match_stdout(r"(?i)Waiting for incoming TCP connection")?;
//
//     // Run initiator to completion
//     let (exitcode, stdout) = CmdBuilder::new(&format!(
//         "cargo run --example 04-routing-over-transport-initiator {rand_port}",
//     ))
//     .run()?;
//
//     // Assert successful run conditions
//     assert_eq!(Some(0), exitcode);
//     assert!(stdout.to_lowercase().contains("goodbye"));
//     Ok(())
// }

#[test]
fn run_04_routing_over_transport_two_hops() -> Result<(), Error> {
    let rand_port_responder = rand::thread_rng().gen_range(10000..65535);
    let rand_port_middle = rand::thread_rng().gen_range(10000..65535);
    // Launch responder, wait for it to start up
    let resp = CmdBuilder::new(&format!(
        "cargo run --example 04-routing-over-transport-two-hops-responder {rand_port_responder}"
    ))
    .spawn()?;
    resp.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Launch middle, wait for it to start up
    let mid = CmdBuilder::new(&format!(
        "cargo run --example 04-routing-over-transport-two-hops-middle {rand_port_middle}"
    ))
    .spawn()?;
    mid.match_stdout(r"(?i)Waiting for incoming TCP connection")?;

    // Run initiator to completion
    let (exitcode, stdout) = CmdBuilder::new(&format!(
        "cargo run --example 04-routing-over-transport-two-hops-initiator {rand_port_middle} {rand_port_responder}"
    ))
    .run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}

#[test]
fn run_04_udp() -> Result<(), Error> {
    let rand_port = rand::thread_rng().gen_range(10000..65535);
    // Launch responder, wait for it to start up
    let resp = CmdBuilder::new(&format!("cargo run --example 04-udp-transport-responder {rand_port}")).spawn()?;
    resp.match_stdout(r"(?i)Waiting for incoming UDP datagram")?;

    // Run initiator to completion
    let (exitcode, stdout) =
        CmdBuilder::new(&format!("cargo run --example 04-udp-transport-initiator {rand_port}",)).run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.to_lowercase().contains("goodbye"));
    Ok(())
}
// TODO: Uncomment me when we manage to solve ports collision problems during CI runs
// #[test]
// fn run_05_secure_channel_over_two_transport_hops() -> Result<(), Error> {
//     let rand_port_responder = rand::thread_rng().gen_range(10000..65535);
//     let rand_port_middle = rand::thread_rng().gen_range(10000..65535);
//     // Launch responder, wait for it to start up
//     let resp = CmdBuilder::new(&format!(
//         "cargo run --example 05-secure-channel-over-two-transport-hops-responder {rand_port_responder}"
//     ))
//     .spawn()?;
//     resp.match_stdout(r"(?i)Waiting for incoming TCP connection")?;
//
//     // Launch middle, wait for it to start up
//     let mid = CmdBuilder::new(&format!(
//         "cargo run --example 05-secure-channel-over-two-transport-hops-middle {rand_port_middle}"
//     ))
//     .spawn()?;
//     mid.match_stdout(r"(?i)Waiting for incoming TCP connection")?;
//
//     // Run initiator to completion
//     let (exitcode, stdout) = CmdBuilder::new(&format!(
//         "cargo run --example 05-secure-channel-over-two-transport-hops-initiator {rand_port_middle} {rand_port_responder}"
//     ))
//     .run()?;
//
//     // Assert successful run conditions
//     assert_eq!(Some(0), exitcode);
//     assert!(stdout.to_lowercase().contains("goodbye"));
//     Ok(())
// }

#[test]
fn run_hello() -> Result<(), Error> {
    let (exitcode, stdout) = CmdBuilder::new("cargo run --example hello").run()?;

    // Assert successful run conditions
    assert_eq!(Some(0), exitcode);
    assert!(stdout.contains("App Received: Hello Ockam!"));
    Ok(())
}
