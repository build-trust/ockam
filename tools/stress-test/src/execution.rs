use crate::{portal_simulator, Relay, State};
use ockam::abac::tokio::task::JoinSet;
use ockam_core::Address;
use std::cmp;

impl State {
    /// Creates resources based on the elapsed time
    pub async fn create_resources_for_delta_time(&self) {
        let elapsed = self.begin.elapsed().as_secs_f32();

        self.create_relays(self.config.calculate_relays(elapsed))
            .await;

        self.create_portals(self.config.calculate_portals(elapsed))
            .await;
    }

    pub async fn create_relays(&self, count: usize) {
        let existing_relays = self.relays.lock().unwrap().len();
        let mut new_relays = count - existing_relays;

        while new_relays > 0 {
            let batch_size = cmp::min(new_relays, 100);

            let mut join_set = JoinSet::new();
            for _ in 0..batch_size {
                let node = self.node.clone();
                let project_addr = self.config.project_addr();
                let context = self.context.clone();
                join_set.spawn(async move {
                    let id = Self::random_id();
                    node.create_relay(&context, &project_addr, id.clone(), None, Some(id))
                        .await
                });
            }

            while let Some(result) = join_set.join_next().await {
                let result = result.expect("cannot join next future");
                match result {
                    Ok(info) => {
                        self.relays.lock().unwrap().insert(
                            info.name().to_string(),
                            Relay {
                                failures_detected: 0,
                                usages: 0,
                            },
                        );
                    }
                    Err(_err) => {
                        self.relay_creation_failed
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }

            new_relays -= batch_size;
        }
    }

    fn random_id() -> String {
        format!("{:x}", rand::random::<u64>())
    }

    /// Returns the relay with the least amount of usage
    fn select_relay_and_increment_usage(&self) -> Option<String> {
        let mut guard = self.relays.lock().unwrap();
        let relay_info = guard.iter_mut().reduce(|acc, relay| {
            if acc.1.usages < relay.1.usages {
                acc
            } else {
                relay
            }
        });

        if let Some((id, relay)) = relay_info {
            relay.usages += 1;
            Some(id.clone())
        } else {
            None
        }
    }

    async fn create_portals(&self, count: usize) {
        let existing_portals = self.portals.lock().unwrap().len();
        let new_portals = count - existing_portals;

        if new_portals == 0 {
            return;
        }

        let relays = self.node.get_relays().await;

        let mut join_set = JoinSet::new();
        for _ in 0..new_portals {
            if let Some(relay_address_id) = self.select_relay_and_increment_usage() {
                let node = self.node.clone();
                let project_addr = self.config.project_addr();
                let context = self.context.clone();
                let throughput = self.config.throughput();

                let relay_flow_control_id = relays
                    .iter()
                    .find(|relay| relay.name() == relay_address_id)
                    .unwrap()
                    .flow_control_id()
                    .clone()
                    .unwrap();

                join_set.spawn(async move {
                    let id = Self::random_id();
                    portal_simulator::create(
                        context,
                        node,
                        id.clone(),
                        project_addr,
                        Address::from_string(format!("forward_to_{relay_address_id}")),
                        throughput,
                        relay_flow_control_id,
                    )
                    .await
                    .map(|stats| (id, stats))
                });
            } else {
                println!("No relays available to create a portal, skipping.");
            }
        }

        while let Some(result) = join_set.join_next().await {
            let result = result.expect("cannot join next future");
            match result {
                Ok((id, portal_stats)) => {
                    self.portals
                        .lock()
                        .unwrap()
                        .insert(id.clone(), portal_stats);
                }
                Err(_err) => {
                    self.portal_creation_failed
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }
}
