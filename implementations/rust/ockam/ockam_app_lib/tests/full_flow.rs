use bytes::BytesMut;
use ockam::Context;
use ockam_api::CliState;
use ockam_app_lib::state::AppState;
use ockam_core::compat::rand;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::info;

async fn start_echo_server() -> SocketAddr {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind server to address");

    let chosen_addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener
                .accept()
                .await
                .expect("Failed to accept connection");

            tokio::spawn(async move {
                let mut buf = vec![0; 1024];
                // In a loop, read data from the socket and write the data back.
                loop {
                    let n = match socket.read(&mut buf).await {
                        // socket closed
                        Ok(n) if n == 0 => return,
                        Ok(n) => n,
                        Err(e) => {
                            println!("Failed to read from socket; err = {:?}", e);
                            return;
                        }
                    };

                    // Write the data back
                    if let Err(e) = socket.write_all(&buf[0..n]).await {
                        println!("Failed to write to socket; err = {:?}", e);
                        return;
                    }
                }
            });
        }
    });

    chosen_addr
}

#[ignore]
#[ockam::test(crate = "ockam", timeout = 300_000)]
async fn test_inlet_data_from_invitation(context: &mut Context) -> ockam::Result<()> {
    let random_service_name: String = "test_".to_string() + &rand::random::<u64>().to_string();

    let cli_state = CliState::test().await?;
    info!("OCKAM_HOME={:?}", cli_state.dir());

    info!("initializing test app state");
    let app_state: AppState = AppState::test(context, cli_state).await;
    app_state.load_model_state().await;

    info!("enrolling user");
    app_state.enroll_user().await.unwrap();

    info!("waiting for app state to load");
    let mut state;
    loop {
        state = app_state.snapshot().await.unwrap();
        if state.loaded {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    let current_user = app_state.state().await.get_default_user().await.unwrap();

    let echo_server_address = start_echo_server().await;
    app_state
        .create_local_service(
            random_service_name.clone(),
            Some("tcp".to_string()),
            echo_server_address.to_string(),
        )
        .await
        .unwrap();

    info!("sending self-invitation");
    app_state
        .create_service_invitation_by_alias(&current_user.email, &random_service_name)
        .await
        .unwrap();

    info!("waiting for invitation to arrive");
    let mut state;
    let mut group;
    let invitation;
    loop {
        state = app_state.snapshot().await.unwrap();
        if let Some(found_group) = state
            .groups
            .into_iter()
            .find(|group| group.email == current_user.email)
        {
            group = found_group;
            if let Some(found_invitation) = group
                .invitations
                .iter()
                .find(|invitation| invitation.service_name == random_service_name)
            {
                invitation = found_invitation;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    let service = state
        .local_services
        .into_iter()
        .find(|service| service.name == random_service_name)
        .unwrap();
    assert_eq!(1, service.shared_with.len());

    let invitee = service.shared_with[0].clone();
    assert_eq!(invitee.email, current_user.email);

    // name currently not available in sent invitation
    // assert_eq!(invitee.name, Some(current_user.name));

    assert_eq!(random_service_name, invitation.service_name);
    assert_eq!("tcp", invitation.service_scheme.as_ref().unwrap());

    assert_eq!(current_user.email, group.email);

    // The inviter name and picture are currently
    // not populated
    // assert_eq!(
    //     Some(current_user.picture.as_str()),
    //     group.image_url.as_deref()
    // );
    // assert_eq!(Some(current_user.name.as_str()), group.name.as_deref());

    info!("accepting invitation");
    app_state.accept_invitation(&invitation.id).await.unwrap();

    info!("waiting for inlet portal to be created");
    let incoming_portal;
    loop {
        let state = app_state.snapshot().await.unwrap();
        if let Some(portal) = state
            .groups
            .into_iter()
            .flat_map(|group| group.incoming_services)
            .find(|portal| portal.source_name == random_service_name)
        {
            incoming_portal = portal;
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    assert_eq!(Some("tcp"), incoming_portal.scheme.as_deref());
    info!("waiting for portal to be usable");

    let incoming_portal;
    loop {
        let state = app_state.snapshot().await.unwrap();
        if let Some(portal) = state
            .groups
            .into_iter()
            .flat_map(|group| group.incoming_services)
            .find(|portal| portal.source_name == random_service_name)
        {
            if portal.available {
                incoming_portal = portal;
                break;
            }
            info!(?portal, "portal not available yet");
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }

    info!("validating portal data exchange");

    // raw tcp connection
    let mut stream = tokio::net::TcpStream::connect(format!(
        "{}:{}",
        incoming_portal.address.as_ref().unwrap(),
        incoming_portal.port.unwrap()
    ))
    .await
    .unwrap();

    stream.write_all(b"hello world").await.unwrap();
    let mut buffer = BytesMut::with_capacity(11);
    let read = stream.read_buf(&mut buffer).await.unwrap();
    assert_eq!(read, 11);
    assert_eq!(&buffer[0..read], b"hello world");

    context.stop().await
}
