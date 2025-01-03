use low_latency_portal::run_inlet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::unix::prelude::CommandExt;
use std::process::Stdio;

const CALLBACK_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4321);

fn main() {
    let mut args: Vec<String> = std::env::args().collect();

    if args.last() == Some("CHILD".to_string()).as_ref() {
        args.pop();
        run_inlet(args.get(1).cloned(), Some(CALLBACK_ADDRESS));
    } else {
        let executable_path = args.remove(0);

        args.push("CHILD".to_string());

        let socket = std::net::UdpSocket::bind(CALLBACK_ADDRESS).unwrap();

        let mut buf = [0; 32];

        println!("Spawning inlet process");
        unsafe {
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
                .unwrap();
        }

        // Possible race condition
        println!("Waiting for the callback");

        socket.recv(&mut buf).unwrap();
        println!("Received callback");
    }
}
