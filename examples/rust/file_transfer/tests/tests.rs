use example_test_helper::{CmdBuilder, Error};
use file_diff::diff;
use rand::Rng;
use std::fmt::Write as _;
use std::fs::{remove_file, OpenOptions};
use std::io::{BufWriter, Write};
use std::path;

// These are all in bytes
const SMALL_CHUNK_SIZE: u32 = 32;
const LARGE_CHUNK_SIZE: u32 = 50 * 1024;
const TINY_FILE_SIZE: u32 = 100;
const MEDIUM_FILE_SIZE: u32 = 100 * 1024;

#[test]
fn tiny_file_transfer() -> Result<(), Error> {
    do_file_transfer(TINY_FILE_SIZE, None)
}

#[test]
fn tiny_file_transfer_small_chunks() -> Result<(), Error> {
    do_file_transfer(TINY_FILE_SIZE, Some(SMALL_CHUNK_SIZE))
}

#[test]
fn medium_file_transfer() -> Result<(), Error> {
    do_file_transfer(MEDIUM_FILE_SIZE, None)
}

#[test]
fn medium_file_transfer_small_chunks() -> Result<(), Error> {
    do_file_transfer(MEDIUM_FILE_SIZE, Some(SMALL_CHUNK_SIZE))
}

#[test]
fn medium_file_transfer_large_chunks() -> Result<(), Error> {
    do_file_transfer(MEDIUM_FILE_SIZE, Some(LARGE_CHUNK_SIZE))
}

fn do_file_transfer(file_size: u32, chunk_size: Option<u32>) -> Result<(), Error> {
    // Spawn receiver, wait for & grab dynamic forwarding address
    let receiver = CmdBuilder::new("cargo run --example receiver").spawn()?;
    let fwd_address = receiver.match_stdout(r"(?m)^FWD_(\w+)$")?.swap_remove(0).unwrap();
    println!("Forwarding address: {fwd_address}");

    // Create temporary binary file to transfer
    // Use unique filenames to stop tests clashing when run in parallel
    //
    // THIS ASSUMES NO TWO TESTS CALL THIS FUNCTION WITH THE SAME ARGUMENTS
    //
    let filename = format!("{}_{}_test.bin", file_size, chunk_size.unwrap_or(0));
    let source_path = format!("tests{}{}", path::MAIN_SEPARATOR, filename);
    let target_path = filename;
    // Scope file and its writer so they are dropped before we continue
    {
        let f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&source_path)?;

        let mut writer = BufWriter::new(f);
        let mut rng = rand::thread_rng();
        for _ in 0..file_size {
            writer.write_all(&[rng.gen::<u8>()])?;
        }
        writer.flush()?;
    }

    // Spawn sender
    let mut cmd_line = format!("cargo run --example sender {source_path} --address {fwd_address}");
    if let Some(val) = chunk_size {
        let _ = write!(cmd_line, " --chunk-size {val}");
    }
    let sender = CmdBuilder::new(&cmd_line).spawn()?;
    sender.match_stdout(r"(?i)End-to-end encrypted secure channel was established")?;

    // Wait for receiver to complete
    let (exitcode, _) = receiver.wait()?;
    assert_eq!(Some(0), exitcode);

    // Check for a successful test
    assert!(diff(&target_path, &source_path));

    // Cleanup
    remove_file(source_path)?;
    remove_file(target_path)?;

    Ok(())
}
