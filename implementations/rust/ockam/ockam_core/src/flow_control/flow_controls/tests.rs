use crate::flow_control::FlowControls;
use crate::Address;
use rand::distributions::Distribution;
use rand::distributions::Uniform;
use rand::prelude::{IteratorRandom, SliceRandom, ThreadRng};
use rand::Rng;

fn add_random_consumer(flow_controls: &FlowControls, mut rng: &mut ThreadRng) -> Vec<Address> {
    let address = Address::random_local();

    let choice = Uniform::from(0..3).sample(&mut rng);

    match choice {
        0 => {
            // Add a Consumer to a non-existent FlowControlId
            let flow_control_id = FlowControls::generate_flow_control_id();

            flow_controls.add_consumer(&address, &flow_control_id);

            vec![address]
        }
        1 => {
            // Add a Consumer to a random Spawner
            let spawner_flow_control_id = flow_controls
                .spawners
                .read()
                .unwrap()
                .values()
                .choose(&mut rng)
                .cloned();
            match spawner_flow_control_id {
                Some(spawner_flow_control_id) => {
                    flow_controls.add_consumer(&address, &spawner_flow_control_id);
                    vec![address]
                }
                _ => vec![],
            }
        }
        2 => {
            // Add a Consumer to a random Producer
            let producer_flow_control_id = flow_controls
                .producers
                .read()
                .unwrap()
                .iter()
                .map(|(_addr, info)| info.flow_control_id())
                .choose(&mut rng)
                .cloned();
            match producer_flow_control_id {
                None => vec![],
                Some(producer_flow_control_id) => {
                    flow_controls.add_consumer(&address, &producer_flow_control_id);
                    vec![address]
                }
            }
        }
        _ => panic!(),
    }
}

fn add_random_spawner(flow_controls: &FlowControls, mut rng: &mut ThreadRng) -> Vec<Address> {
    let address = Address::random_local();

    let choice: f32 = rng.gen();

    if choice < 0.8 {
        // Add a Spawner to a new FlowControlId
        let flow_control_id = FlowControls::generate_flow_control_id();

        flow_controls.add_spawner(&address, &flow_control_id);

        vec![address]
    } else {
        // Add a Spawner to an existing FlowControlId
        let spawner_flow_control_id = flow_controls
            .spawners
            .read()
            .unwrap()
            .values()
            .choose(&mut rng)
            .cloned();

        match spawner_flow_control_id {
            Some(spawner_flow_control_id) => {
                flow_controls.add_spawner(&address, &spawner_flow_control_id);
                vec![address]
            }
            _ => vec![],
        }
    }
}

fn add_random_producer(flow_controls: &FlowControls, mut rng: &mut ThreadRng) -> Vec<Address> {
    let address = Address::random_local();

    let choice: f32 = rng.gen();

    if choice < 0.5 {
        // Add a Producer without a Spawner
        let flow_control_id = FlowControls::generate_flow_control_id();

        flow_controls.add_producer(&address, &flow_control_id, None, vec![]);

        vec![address]
    } else {
        // Add a Producer with a Spawner
        let flow_control_id = FlowControls::generate_flow_control_id();

        let spawner_flow_control_id = flow_controls
            .spawners
            .read()
            .unwrap()
            .values()
            .choose(&mut rng)
            .cloned();

        match spawner_flow_control_id {
            Some(spawner_flow_control_id) => {
                let choice2: f32 = rng.gen();
                if choice2 < 0.5 {
                    // No additional address
                    flow_controls.add_producer(
                        &address,
                        &flow_control_id,
                        Some(&spawner_flow_control_id),
                        vec![],
                    );
                    vec![address]
                } else {
                    // Random additional address
                    let additional_address = Address::random_local();
                    flow_controls.add_producer(
                        &address,
                        &flow_control_id,
                        Some(&spawner_flow_control_id),
                        vec![additional_address.clone()],
                    );
                    vec![address, additional_address]
                }
            }
            None => vec![],
        }
    }
}

#[test]
fn test_cleanup() {
    let mut rng = rand::thread_rng();
    let flow_controls = FlowControls::new();
    let mut addresses = Vec::<Address>::new();

    let n = 100;
    for _ in 0..n {
        // Generate an event:
        // 0.0..0.4 => Add an consumer
        // 0.4..0.7 => Add a producer
        // 0.7..0.9 => Add a spawner
        // 0.9..1.0 => Delete an Address
        let x: f64 = rng.gen();

        let mut new_addresses = if x < 0.4 {
            add_random_consumer(&flow_controls, &mut rng)
        } else if x < 0.7 {
            add_random_producer(&flow_controls, &mut rng)
        } else if x < 0.9 {
            add_random_spawner(&flow_controls, &mut rng)
        } else {
            match addresses.iter().enumerate().choose(&mut rng) {
                None => {}
                Some((index, address)) => {
                    let address = address.clone();
                    addresses.remove(index);
                    flow_controls.cleanup_address(&address);
                }
            };

            vec![]
        };

        addresses.append(&mut new_addresses);
    }

    addresses.shuffle(&mut rng);

    for address in addresses.into_iter() {
        flow_controls.cleanup_address(&address);
    }

    assert!(flow_controls.consumers.read().unwrap().is_empty());
    assert!(flow_controls.producers.read().unwrap().is_empty());
    assert!(flow_controls
        .producers_additional_addresses
        .read()
        .unwrap()
        .is_empty());
    assert!(flow_controls.spawners.read().unwrap().is_empty());
}
