use anyhow::{anyhow, Result};
use duct::cmd;
use rand::RngCore;
use ron::de::from_str;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

const MATCH_TIMEOUT_SECS: u64 = 120;
const OUTPUT_FILE_PREFIX: &str = "/tmp/exrun-";

pub type Step = String;

pub type Stage = Vec<Step>;

#[derive(Deserialize, Debug)]
pub struct Script {
    title: String,
    stages: Vec<Stage>,
}

/// Run the steps of a stage.
///
/// A stage is considered as passed if we reach the end of a stage
/// without signalling a failure.
///
/// A failure is signalled if one of the below happens...
/// - a 'match' step times out before finding its match
/// - a spawned process exits with an error
fn run_stage(stage: Stage) -> Result<()> {
    let finished = Arc::new(AtomicBool::new(false));
    let failed = Arc::new(AtomicBool::new(false));

    let join_handles = Arc::new(Mutex::new(Vec::new()));

    let mut match_stack: Vec<String> = Vec::new();
    let mut out_order: Vec<String> = Vec::new();
    let mut outputs: Vec<String> = Vec::new();
    let mut arg_order: Vec<String> = Vec::new();

    let mut stage_iter = stage.into_iter();
    while let (Some(mut step), false) = (stage_iter.next(), finished.load(Relaxed)) {
        println!("STEP: {}", step);

        if step.starts_with("sleep ") {
            let duration = step.split_off(6);
            let duration = duration.trim();
            let duration: u64 = duration.parse()?;
            sleep(Duration::from_secs(duration));
            continue;
        }

        if step.starts_with("match ") {
            // Try to do case-insensitve match and grab the contents
            // of the match and upto a newline.
            let pattern = step.split_off(6);
            let pattern_lower = pattern.to_lowercase();
            let output_path = outputs.last().unwrap();
            println!(
                "Matching '{}' in output (case insensitive, see file {})",
                pattern, output_path
            );

            let start_time = Instant::now();
            while !finished.load(Relaxed) {
                if let Ok(mut f) = File::open(output_path) {
                    let mut s = String::new();
                    if f.read_to_string(&mut s).is_ok() {
                        let s_lower = s.to_lowercase();
                        if let Some(index) = s_lower.find(pattern_lower.as_str()) {
                            let mut matching = s.split_off(index);
                            if let Some(end) = matching.find('\n') {
                                matching.truncate(end);
                            }
                            println!("Matched '{}'", matching);
                            match_stack.push(matching.to_string());
                            break;
                        }
                    }
                }

                // Timeout when needed. Signal failure.
                if start_time.elapsed().as_secs() >= MATCH_TIMEOUT_SECS {
                    println!("Match timed out ({} seconds)", MATCH_TIMEOUT_SECS);
                    failed.store(true, Relaxed);
                    finished.store(true, Relaxed);
                    break;
                }

                sleep(Duration::from_secs(1))
            }
            continue;
        }

        if step.starts_with("arg ") {
            // Grab a previous match by index and add it as a command line
            // argument for the next spawn.
            let index = step.split_off(4);
            let index: usize = index.parse()?;
            let matching = match_stack.get(index).unwrap();
            arg_order.push(matching.clone());
            continue;
        }

        if step.starts_with("out ") {
            // Grab a previous match by index and add it to stdin for the next spawn.
            let index = step.split_off(4);
            let index: usize = index.parse()?;
            let matching = match_stack.get(index).unwrap();
            out_order.push(matching.clone());
            continue;
        }

        // If step starts with 'cmd', launch its suffix as a command.
        // Otherwise, take step as the name of an example to be run with cargo.
        let mut cmd_line = if step.starts_with("cmd ") {
            step.split_off(4)
        } else {
            format!("cargo run --example {}", step)
        };

        cmd_line.push_str(format!(" {}", arg_order.join(" ")).as_str());
        arg_order.clear();

        let output_file = format!("{}{}", OUTPUT_FILE_PREFIX, rand::thread_rng().next_u32());
        outputs.push(output_file.clone());

        let stdin = format!("{}\n", out_order.join("\n"));
        out_order.clear();

        println!("  Command line: {:?}", cmd_line);
        println!("  Output file: {}", output_file);
        println!("  Stdin: {:?}", stdin);

        let finished_clone = finished.clone();
        let failed_clone = failed.clone();
        let join_handle = std::thread::spawn(move || {
            let cmd_line = cmd_line.split_whitespace().collect::<Vec<_>>();
            let handle = cmd(cmd_line[0], &cmd_line[1..])
                .stdout_file(File::create(output_file).unwrap())
                .stdin_bytes(stdin)
                .start()
                .unwrap();

            // Wait till stage has finished, or handle has finshed with an error.
            while !finished_clone.load(Relaxed) {
                if let Err(e) = handle.try_wait() {
                    println!("Error: {}", e);
                    failed_clone.store(true, Relaxed);
                    finished_clone.store(true, Relaxed);
                }
                sleep(Duration::from_secs(1));
            }
            handle.kill().unwrap();
        });

        join_handles.lock().unwrap().push(join_handle);
    }

    // We've run out of steps. Signal we've finished this stage.
    finished.store(true, Relaxed);

    // Wait for spawned to shut themselves down
    let mut join_handles = join_handles.lock().unwrap();
    while !join_handles.is_empty() {
        let h = join_handles.pop().unwrap();
        let r = h.join();
        if r.is_err() {
            failed.store(true, Relaxed);
        }
    }

    if failed.load(Relaxed) {
        println!("FAILED");
        Err(anyhow!("Failed, check logs."))
    } else {
        println!("PASSED");
        Ok(())
    }
}

fn run(script: Script) -> Result<()> {
    println!("Running {}", script.title);
    let stage_count = script.stages.len();
    for (i, stage) in script.stages.into_iter().enumerate() {
        println!("==============================");
        println!("STAGE #{} of {}", i + 1, stage_count);
        run_stage(stage)?;
    }
    Ok(())
}

/// Exit code is 0 if all stages passed, or non-0 if a stage fails.
///
/// Execution stops as soon as a stage fails.
fn main() -> Result<()> {
    let file = std::env::args()
        .nth(1)
        .expect("missing script file argument");
    let mut file = File::open(file).expect("unable to open script file");
    let mut guide = String::new();

    file.read_to_string(&mut guide)?;

    let script: Script = from_str(guide.as_str()).expect("script is invalid");
    run(script)?;
    Ok(())
}
