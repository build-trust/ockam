use crate::State;
use ockam::abac::tokio;
use ockam::abac::tokio::runtime::Runtime;

impl State {
    pub fn display_loop(&self, runtime: Runtime) {
        runtime.block_on(async {
            loop {
                self.measure_speed();
                self.print_summary();
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }

    pub fn print_summary(&self) {
        let elapsed = self.begin.elapsed().as_secs();
        // print the header every 10 seconds
        if elapsed % 10 == 0 {
            println!("|  Elapsed  | Portals | Relays | M. sent | M. recv | In-fli |  B. sent  |  B. recv  | Spe. sent  | Spe. recv  | M. OOO | Errors |")
        }

        let (speed_sent, speed_received) = self.calculate_average_speed();

        let errors: u32 = {
            let relays = self.relays.lock().unwrap();
            relays
                .values()
                .map(|relay| relay.failures_detected)
                .sum::<u32>()
        } + self
            .relay_creation_failed
            .load(std::sync::atomic::Ordering::Relaxed)
            + self
                .portal_creation_failed
                .load(std::sync::atomic::Ordering::Relaxed);

        {
            let portals = self.portals.lock().unwrap();
            let total_messages_sent: u64 = portals
                .values()
                .map(|stats| {
                    stats
                        .messages_sent
                        .load(std::sync::atomic::Ordering::Relaxed)
                })
                .sum();
            let total_bytes_sent: u64 = portals
                .values()
                .map(|stats| stats.bytes_sent.load(std::sync::atomic::Ordering::Relaxed))
                .sum();
            let total_messages_received: u64 = portals
                .values()
                .map(|stats| {
                    stats
                        .messages_received
                        .load(std::sync::atomic::Ordering::Relaxed)
                })
                .sum();
            let messages_out_of_order: u64 = portals
                .values()
                .map(|stats| {
                    stats
                        .messages_out_of_order
                        .load(std::sync::atomic::Ordering::Relaxed)
                })
                .sum();
            let total_bytes_received: u64 = portals
                .values()
                .map(|stats| {
                    stats
                        .bytes_received
                        .load(std::sync::atomic::Ordering::Relaxed)
                })
                .sum();

            let total_portals = portals.len();
            let total_relays = self.relays.lock().unwrap().len();
            let in_fly = total_messages_sent - total_messages_received;

            println!(
                "| {:^9} | {:^7} | {:^6} | {:^7} | {:^7} | {:^6} | {:^9} | {:^9} | {:^10} | {:^10} | {:^6} | {:^6} |",
                time_to_human_readable(elapsed),
                total_portals,
                total_relays,
                total_messages_sent,
                total_messages_received,
                in_fly,
                bytes_to_human_readable(total_bytes_sent),
                bytes_to_human_readable(total_bytes_received),
                speed_to_human_readable(speed_sent),
                speed_to_human_readable(speed_received),
                messages_out_of_order,
                errors,
            );
        }
    }
}

fn time_to_human_readable(elapsed: u64) -> String {
    let hours = elapsed / 3600;
    let minutes = (elapsed % 3600) / 60;
    let seconds = elapsed % 60;

    if hours > 0 {
        format!("{hours}h{minutes}m{seconds:02}s")
    } else if minutes > 0 {
        format!("{minutes}m{seconds:02}s")
    } else {
        format!("{seconds:02}s")
    }
}

fn bytes_to_human_readable(bytes: u64) -> String {
    let bytes = bytes as f64;
    let kb = bytes / 1024.0;
    let mb = kb / 1024.0;
    let gb = mb / 1024.0;
    if gb > 0.1 {
        format!("{:.2} GB", gb)
    } else if mb > 0.1 {
        format!("{:.2} MB", mb)
    } else if kb > 0.1 {
        format!("{:.2} KB", kb)
    } else {
        format!("{} B", bytes)
    }
}

fn speed_to_human_readable(bytes: f64) -> String {
    let bits = bytes * 8.0;
    let kb = bits / 1000.0;
    let mb = kb / 1000.0;
    let gb = mb / 1000.0;
    if gb >= 0.1 {
        format!("{:.2} Gbps", gb)
    } else if mb >= 0.1 {
        format!("{:.2} Mbps", mb)
    } else if kb >= 0.1 {
        format!("{:.2} Kbps", kb)
    } else {
        format!("{:.2} bps", bits)
    }
}
