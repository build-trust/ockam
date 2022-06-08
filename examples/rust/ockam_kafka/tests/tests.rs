use example_test_helper::{CmdBuilder, Error};

#[test]
fn alice_bob_securechannel() -> Result<(), Error> {
    // Spawn BOB and get the dynamic stream addresses
    let bob = CmdBuilder::new("cargo run --example ockam_kafka_bob").spawn()?;
    let regex = r"(?im)^bob_to_alice stream address is: (\w+)$";
    let addr_bob_to_alice = bob.match_stdout(regex)?.swap_remove(1).unwrap();
    let regex = r"(?im)^alice_to_bob stream address is: (\w+)$";
    let addr_alice_to_bob = bob.match_stdout(regex)?.swap_remove(1).unwrap();
    println!("Address bob -> alice: {addr_bob_to_alice}");
    println!("Address alice -> bob: {addr_alice_to_bob}");

    // Prepare stdin for ALICE
    let stdin = format!("{addr_bob_to_alice}\n{addr_alice_to_bob}\n");

    // Spawn ALICE and wait for 'success'
    let alice = CmdBuilder::new("cargo run --example ockam_kafka_alice")
        .set_stdin(stdin.as_bytes())
        .spawn()?;
    alice.match_stdout(r"(?i)End-to-end encrypted secure channel was established.")?;

    Ok(())
}
