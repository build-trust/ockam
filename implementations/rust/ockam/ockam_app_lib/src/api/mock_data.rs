use crate::api::state::rust::Invitee;
use crate::api::state::{c, convert_to_c, rust, OrchestratorStatus};

/// This function serves to create a mock application state for the UI.
/// The sole purpose is to have a quick preview without requiring an initialized state.
#[no_mangle]
extern "C" fn mock_application_state() -> c::ApplicationState {
    let state = rust::ApplicationState {
        enrolled: true,
        orchestrator_status: OrchestratorStatus::Connected,
        enrollment_name: Some("Davide Baldo".into()),
        enrollment_email: Some("davide@baldo.me".into()),
        enrollment_image: Some("https://avatars.githubusercontent.com/u/408088?v=4".into()),
        enrollment_github_user: Some("davide-baldo".into()),
        local_services: vec![
            rust::LocalService {
                name: "Super Cool Web Demo".into(),
                address: "localhost".into(),
                port: 8080,
                scheme: Some("http".into()),
                shared_with: vec![rust::Invitee {
                    name: Some("Adrian Benavides".into()),
                    email: "adrian@ockam.io".into(),
                }],
                available: true,
            },
            rust::LocalService {
                name: "My Router Admin Page".into(),
                address: "localhost".into(),
                port: 8080,
                scheme: Some("http".into()),
                shared_with: vec![rust::Invitee {
                    name: Some("Adrian Benavides".into()),
                    email: "adrian@ockam.io".into(),
                }],
                available: true,
            },
        ],
        groups: vec![
            rust::ServiceGroup {
                email: "mrinal@ockam.io".to_string(),
                name: Some("Mrinal Wadhwa".into()),
                image_url: Some("https://avatars.githubusercontent.com/u/159583?v=4".into()),
                invitations: vec![rust::Invitation {
                    id: "5373".into(),
                    service_name: "New Concept".into(),
                    service_scheme: Some("http".into()),
                    accepting: false,
                }],
                incoming_services: vec![],
            },
            rust::ServiceGroup {
                name: Some("Adrian Benavides".into()),
                email: "adrian@ockam.io".into(),
                image_url: Some("https://avatars.githubusercontent.com/u/12375782?v=4".into()),
                invitations: vec![
                    rust::Invitation {
                        id: "1234".into(),
                        service_name: "Local Web Deployment".into(),
                        service_scheme: Some("http".into()),
                        accepting: false,
                    },
                    rust::Invitation {
                        id: "5678".into(),
                        service_name: "Secret Wiki".into(),
                        service_scheme: Some("http".into()),
                        accepting: true,
                    },
                ],
                incoming_services: vec![rust::Service {
                    id: "id-1".into(),
                    source_name: "ssh".into(),
                    address: Some("127.0.0.1".into()),
                    port: Some(22),
                    scheme: Some("ssh".into()),
                    available: false,
                    enabled: true,
                }],
            },
            rust::ServiceGroup {
                name: Some("Eric Torreborre".into()),
                email: "eric.torreborre@ockam.io".into(),
                image_url: Some("https://avatars.githubusercontent.com/u/10988?v=4".into()),
                invitations: vec![],
                incoming_services: vec![
                    rust::Service {
                        id: "id-2".into(),
                        source_name: "Production Database".into(),
                        address: Some("localhost".into()),
                        port: Some(5432),
                        scheme: Some("postgresql".into()),
                        available: true,
                        enabled: true,
                    },
                    rust::Service {
                        id: "id-3".into(),
                        source_name: "Test Database".into(),
                        address: Some("localhost".into()),
                        port: Some(8776),
                        scheme: Some("postgresql".into()),
                        available: true,
                        enabled: false,
                    },
                ],
            },
        ],
        sent_invitations: vec![
            Invitee {
                name: Some("Adrian Benavides".into()),
                email: "adrian@ockam.io".into(),
            },
            Invitee {
                name: None,
                email: "eric.torreborre@ockam.io".into(),
            },
        ],
    };

    convert_to_c(state)
}
