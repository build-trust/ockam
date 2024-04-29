use crate::State;

impl State {
    // measure the last second speed based on delta between previous and current values
    // and stores last 10 values it in the speed_stats
    pub fn measure_speed(&self) {
        let delta_sent;
        let delta_received;
        {
            let portals = self.portals.lock().unwrap();
            let total_bytes_sent: u64 = portals
                .values()
                .map(|stats| stats.bytes_sent.load(std::sync::atomic::Ordering::Relaxed))
                .sum();
            let total_bytes_received: u64 = portals
                .values()
                .map(|stats| {
                    stats
                        .bytes_received
                        .load(std::sync::atomic::Ordering::Relaxed)
                })
                .sum();

            // assume it's invoked once per second
            delta_sent = total_bytes_sent
                - self
                    .previous_bytes_sent
                    .load(std::sync::atomic::Ordering::Relaxed);
            delta_received = total_bytes_received
                - self
                    .previous_bytes_received
                    .load(std::sync::atomic::Ordering::Relaxed);

            self.previous_bytes_sent
                .store(total_bytes_sent, std::sync::atomic::Ordering::Relaxed);
            self.previous_bytes_received
                .store(total_bytes_received, std::sync::atomic::Ordering::Relaxed);
        }

        let mut guard = self.speed_stats.lock().unwrap();
        guard.push((delta_sent, delta_received));
        if guard.len() > 10 {
            guard.remove(0);
        }
    }

    pub fn calculate_average_speed(&self) -> (f64, f64) {
        let guard = self.speed_stats.lock().unwrap();
        let (sent, received) = guard.iter().fold((0, 0), |acc, (sent, received)| {
            (acc.0 + sent, acc.1 + received)
        });
        let size = guard.len();
        if size > 0 {
            (sent as f64 / size as f64, received as f64 / size as f64)
        } else {
            (0.0, 0.0)
        }
    }
}
