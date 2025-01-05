use low_latency_portal::run_inlet;
use std::os::unix::prelude::CommandExt;
use std::process::Stdio;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut args: Vec<String> = std::env::args().collect();

    if args.last() == Some("CHILD".to_string()).as_ref() {
        args.pop();
        run_inlet(args.get(1).cloned(), true);
    } else {
        let executable_path = args.remove(0);

        args.push("CHILD".to_string());

        println!("Spawning inlet process");
        let child = unsafe {
            std::process::Command::new(executable_path)
                .args(args)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                // This unsafe block will only panic if the closure panics, which shouldn't happen
                .pre_exec(|| {
                    // Detach the process from the parent
                    nix::unistd::setsid().map_err(std::io::Error::from)?;
                    Ok(())
                })
                .spawn()
                .unwrap()
        };

        println!("Spawned inlet pid: {}", child.id());

        // Possible race condition
        println!("Waiting for the callback");
        sleep(Duration::from_secs(5));

        println!("Didn't receive the callback");
    }
}
