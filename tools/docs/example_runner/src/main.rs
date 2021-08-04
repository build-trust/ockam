use anyhow::Result;
use duct::cmd;
use ron::de::from_str;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

pub type Step = String;

pub type Stage = Vec<Step>;

#[derive(Deserialize, Debug)]
pub struct Script {
    title: String,
    stages: Vec<Stage>,
}

fn run_stage(stage: Stage) -> Result<()> {
    let stop = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));

    let join_handles = Arc::new(Mutex::new(Vec::new()));

    for mut step in stage {
        let stop = stop.clone();
        let finished = finished.clone();
        let join_handles = join_handles.clone();

        if step.starts_with("sleep ") {
            let duration = step.split_off(6);
            let duration = duration.trim();
            let duration: u64 = duration.parse()?;
            println!("Sleeping for {} seconds", duration);
            sleep(Duration::from_secs(duration));
            continue;
        }
        let join_handle = std::thread::spawn(move || {
            let handle = cmd!("cargo", "run", "--example", step).start().unwrap();
            while !stop.load(Ordering::Relaxed) {
                if let Some(_) = match handle.try_wait() {
                    Ok(x) => x,
                    Err(_) => {
                        std::process::exit(1);
                    }
                } {
                    finished.store(true, Ordering::Relaxed);
                    break;
                }
                sleep(Duration::from_secs(1));
            }
            handle.kill().unwrap();
        });
        join_handles.lock().unwrap().push(join_handle);
        sleep(Duration::from_secs(1));
    }

    while !finished.load(Ordering::Relaxed) {
        sleep(Duration::from_secs(1));
    }
    stop.store(true, Ordering::Relaxed);
    let join_handles = join_handles.clone();
    let mut join_handles = join_handles.lock().unwrap();
    while !join_handles.is_empty() {
        join_handles.pop().unwrap().join().unwrap();
    }
    Ok(())
}

fn run(script: Script) -> Result<()> {
    println!("Running {}", script.title);
    for stage in script.stages {
        run_stage(stage)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let file = std::env::args()
        .skip(1)
        .next()
        .expect("missing script file");
    let mut file = File::open(file).expect("unable to open script");
    let mut guide = String::new();

    file.read_to_string(&mut guide)?;

    let script: Script = from_str(guide.as_str()).expect("script is invalid");
    run(script)?;
    Ok(())
}
