//! Woof

use std::{io::Read, os::unix::net::UnixStream, path::PathBuf, time::Duration};
use nix::unistd::mkfifo;

/// A node watchdog restarts the node when it crashes
///
/// Watchdogs get restarted by the user-facing CLI
pub struct Watchdog {
    pub socket_path: PathBuf,
}

impl Watchdog {
    pub fn run(self) {
        match UnixStream::connect(self.socket_path) {
            Ok(stream) => {
                loop {
                    let mut buf = vec![0; 64];
                    match stream.peek(&mut buf) {
                        Ok(_) => std::thread::sleep(Duration::from_millis(200)),
                        Err(_) => break,
                    }
                }

                // ???
            }
            Err(_) => {
                eprintln!("failed to connect to node socket.  is it running?")
            }
        }
    }
}
