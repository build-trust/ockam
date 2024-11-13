use crate::common::session::{MockEchoer, MockHop, MockReplacer};
use core::sync::atomic::Ordering;
use ockam::{Address, Context};
use ockam_api::session::connection_status::ConnectionStatus;
use ockam_api::session::session::{AdditionalSessionOptions, Session};
use ockam_core::compat::sync::Arc;
use ockam_core::{route, AllowAll, DenyAll, Result};
use std::time::Duration;
use tokio::time::sleep;

mod common;

#[allow(non_snake_case)]
#[ockam::test]
async fn connect__unavailable__should_fail(ctx: &mut Context) -> Result<()> {
    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("main", route![]),
    ));
    let additional_mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("additional", route![]),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        Some(AdditionalSessionOptions::new(
            additional_mock_replacer.clone(),
            false,
            Duration::from_secs(120),
            Duration::from_secs(1),
        )),
        Duration::from_secs(1),
        Duration::from_secs(120),
    );

    assert!(session.last_outcome().is_none());
    assert_eq!(session.connection_status(), ConnectionStatus::Down);
    assert!(!session.is_being_replaced());
    assert!(!mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));

    assert_eq!(
        session.additional_connection_status(),
        Some(ConnectionStatus::Down)
    );
    assert!(!session.additional_is_being_replaced().unwrap());
    assert!(!additional_mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));
    assert!(!additional_mock_replacer
        .lock()
        .await
        .close_called
        .load(Ordering::Relaxed));

    // Session relies on echo to verify if a session is alive
    ctx.start_worker(Address::from_string("echo"), MockEchoer::new())?;

    mock_replacer
        .lock()
        .await
        .succeeds
        .store(false, Ordering::Relaxed);

    let res = session.initial_connect().await;
    assert!(res.is_err());

    assert!(mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));
    assert!(!additional_mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));

    Ok(())
}

#[allow(non_snake_case)]
#[ockam::test]
async fn connect__only_main_available_no_fallback__should_fail(ctx: &mut Context) -> Result<()> {
    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("main", route![]),
    ));
    let additional_mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("additional", route![]),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        Some(AdditionalSessionOptions::new(
            additional_mock_replacer.clone(),
            false,
            Duration::from_secs(120),
            Duration::from_secs(1),
        )),
        Duration::from_secs(1),
        Duration::from_secs(120),
    );

    // Session relies on echo to verify if a session is alive
    ctx.start_worker(Address::from_string("echo"), MockEchoer::new())?;

    additional_mock_replacer
        .lock()
        .await
        .succeeds
        .store(false, Ordering::Relaxed);

    let res = session.initial_connect().await;
    assert!(res.is_err());

    assert!(mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));
    assert!(additional_mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));

    Ok(())
}

#[allow(non_snake_case)]
#[ockam::test]
async fn connect__only_main_available_fallback__should_succeed(ctx: &mut Context) -> Result<()> {
    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("main", route![]),
    ));
    let additional_mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("additional", route![]),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        Some(AdditionalSessionOptions::new(
            additional_mock_replacer.clone(),
            true,
            Duration::from_secs(120),
            Duration::from_secs(1),
        )),
        Duration::from_secs(1),
        Duration::from_secs(120),
    );

    // Session relies on echo to verify if a session is alive
    ctx.start_worker(Address::from_string("echo"), MockEchoer::new())?;

    additional_mock_replacer
        .lock()
        .await
        .succeeds
        .store(false, Ordering::Relaxed);

    let res = session.initial_connect().await;
    assert!(res.is_ok());

    assert!(mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));

    session.start_monitoring()?;

    sleep(Duration::from_millis(250)).await;

    assert!(additional_mock_replacer
        .lock()
        .await
        .close_called
        .load(Ordering::Relaxed));

    assert!(additional_mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));

    session.stop().await;

    assert!(additional_mock_replacer
        .lock()
        .await
        .close_called
        .load(Ordering::Relaxed));

    Ok(())
}

#[allow(non_snake_case)]
#[ockam::test]
async fn start_monitoring__available__should_succeed(ctx: &mut Context) -> Result<()> {
    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("main", route![]),
    ));
    let additional_mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("additional", route![]),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        Some(AdditionalSessionOptions::new(
            additional_mock_replacer.clone(),
            false,
            Duration::from_secs(120),
            Duration::from_secs(1),
        )),
        Duration::from_secs(1),
        Duration::from_secs(120),
    );

    // Session relies on echo to verify if a session is alive
    ctx.start_worker(Address::from_string("echo"), MockEchoer::new())?;

    session.start_monitoring()?;

    sleep(Duration::from_millis(250)).await;

    assert!(mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));

    assert!(!additional_mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));

    sleep(Duration::from_millis(1500)).await;

    assert!(additional_mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));

    session.stop().await;

    assert!(additional_mock_replacer
        .lock()
        .await
        .close_called
        .load(Ordering::Relaxed));

    Ok(())
}

#[allow(non_snake_case)]
#[ockam::test]
async fn start_monitoring__additional_is_down__should_recreate(ctx: &mut Context) -> Result<()> {
    let hop = MockHop::new();
    let hop_responsive = hop.responsive.clone();

    ctx.start_worker("hop", hop)?;

    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("main", route![]),
    ));
    let additional_mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("additional", route!["hop"]),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        Some(AdditionalSessionOptions::new(
            additional_mock_replacer.clone(),
            false,
            Duration::from_secs(1),
            Duration::from_secs(1),
        )),
        Duration::from_secs(1),
        Duration::from_secs(120),
    );

    // Session relies on echo to verify if a session is alive
    ctx.start_worker(Address::from_string("echo"), MockEchoer::new())?;

    session.initial_connect().await?;

    session.start_monitoring()?;

    additional_mock_replacer
        .lock()
        .await
        .succeeds
        .store(false, Ordering::Relaxed);
    hop_responsive.store(false, Ordering::Relaxed);

    assert_eq!(
        session.additional_connection_status().unwrap(),
        ConnectionStatus::Up
    );
    sleep(Duration::from_millis(1000)).await;

    assert_eq!(
        session.additional_connection_status().unwrap(),
        ConnectionStatus::Up
    );
    sleep(Duration::from_millis(1000)).await;

    assert_eq!(
        session.additional_connection_status().unwrap(),
        ConnectionStatus::Up
    );
    sleep(Duration::from_millis(2500)).await;

    assert_eq!(
        session.additional_connection_status().unwrap(),
        ConnectionStatus::Down
    );

    additional_mock_replacer
        .lock()
        .await
        .succeeds
        .store(true, Ordering::Relaxed);
    hop_responsive.store(true, Ordering::Relaxed);

    sleep(Duration::from_millis(1500)).await;

    assert_eq!(
        session.additional_connection_status().unwrap(),
        ConnectionStatus::Up
    );

    session.stop().await;

    Ok(())
}

#[allow(non_snake_case)]
#[ockam::test]
async fn start_monitoring__both_down__should_recreate(ctx: &mut Context) -> Result<()> {
    let hop_main = MockHop::new();
    let hop_main_responsive = hop_main.responsive.clone();

    ctx.start_worker("hop_main", hop_main)?;

    let hop_additional = MockHop::new();
    let hop_additional_responsive = hop_additional.responsive.clone();

    ctx.start_worker("hop_additional", hop_additional)?;

    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("main", route!["hop_main"]),
    ));
    let additional_mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::new("additional", route!["hop_additional"]),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        Some(AdditionalSessionOptions::new(
            additional_mock_replacer.clone(),
            false,
            Duration::from_secs(1),
            Duration::from_secs(1),
        )),
        Duration::from_secs(1),
        Duration::from_secs(1),
    );

    // Session relies on echo to verify if a session is alive
    ctx.start_worker(Address::from_string("echo"), MockEchoer::new())?;

    session.initial_connect().await?;

    mock_replacer
        .lock()
        .await
        .succeeds
        .store(false, Ordering::Relaxed);

    hop_main_responsive.store(false, Ordering::Relaxed);
    hop_additional_responsive.store(false, Ordering::Relaxed);

    session.start_monitoring()?;

    sleep(Duration::from_millis(4500)).await;

    assert_eq!(session.connection_status(), ConnectionStatus::Down);
    assert_eq!(
        session.additional_connection_status().unwrap(),
        ConnectionStatus::Down
    );

    hop_main_responsive.store(false, Ordering::Relaxed);
    hop_additional_responsive.store(true, Ordering::Relaxed);

    sleep(Duration::from_millis(1500)).await;

    assert_eq!(session.connection_status(), ConnectionStatus::Down);
    assert_eq!(
        session.additional_connection_status().unwrap(),
        ConnectionStatus::Down
    );

    mock_replacer
        .lock()
        .await
        .succeeds
        .store(true, Ordering::Relaxed);
    additional_mock_replacer
        .lock()
        .await
        .succeeds
        .store(true, Ordering::Relaxed);

    sleep(Duration::from_millis(2500)).await;

    assert_eq!(session.connection_status(), ConnectionStatus::Up);
    assert_eq!(
        session.additional_connection_status().unwrap(),
        ConnectionStatus::Up
    );

    session.stop().await;

    Ok(())
}

// TODO: Check recreate is called
