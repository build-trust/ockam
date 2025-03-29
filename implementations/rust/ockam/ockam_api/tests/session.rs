use crate::common::session::{MockEchoer, MockReplacer};
use core::sync::atomic::Ordering;
use ockam::{Address, Context};
use ockam_api::session::connection_status::ConnectionStatus;
use ockam_api::session::session::Session;
use ockam_core::compat::sync::Arc;
use ockam_core::{AllowAll, DenyAll, Result};
use rand::random;
use std::time::Duration;

mod common;

#[allow(non_snake_case)]
#[ockam::test]
async fn connect__unavailable__should_fail(ctx: &mut Context) -> Result<()> {
    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::default(),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        None,
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
    assert!(!mock_replacer
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

    Ok(())
}

#[allow(non_snake_case)]
#[ockam::test]
async fn connect__available__should_succeed(ctx: &mut Context) -> Result<()> {
    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::default(),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        None,
        Duration::from_secs(1),
        Duration::from_secs(120),
    );

    // Session relies on echo to verify if a session is alive
    ctx.start_worker(Address::from_string("echo"), MockEchoer::new())?;

    let res = session.initial_connect().await;
    assert!(res.is_ok());

    Ok(())
}

#[allow(non_snake_case)]
#[ockam::test]
async fn start_monitoring__available__should_be_up_fast(ctx: &mut Context) -> Result<()> {
    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::default(),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        None,
        Duration::from_secs(1),
        Duration::from_secs(120),
    );

    // Session relies on echo to verify if a session is alive
    ctx.start_worker(Address::from_string("echo"), MockEchoer::new())?;

    assert!(!ctx.is_worker_registered_at(session.collector_address())?);

    // Start the Session in a separate task
    session.start_monitoring()?;

    assert!(ctx.is_worker_registered_at(session.collector_address())?);

    let mut time_to_restore = 0;

    loop {
        // Check that the session is now up, since we don't have any
        // synchronization we keep to keep checking until it's up
        if session.connection_status() == ConnectionStatus::Up {
            assert!(!session.is_being_replaced());
            assert!(session.last_outcome().is_some());
            break;
        }

        if time_to_restore > 1 {
            assert!(session.is_being_replaced());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        time_to_restore += 1;
        continue;
    }

    // It can't take less than it takes to create that session
    assert!(time_to_restore >= 4);
    // Should not take much longer than it takes to create that session
    assert!(time_to_restore <= 6);

    // Shut down the test
    session.stop().await;

    assert!(mock_replacer
        .lock()
        .await
        .close_called
        .load(Ordering::Relaxed));

    Ok(())
}

#[allow(non_snake_case)]
#[ockam::test]
async fn start_monitoring__temporary_unavailable__should_eventually_be_up(
    ctx: &mut Context,
) -> Result<()> {
    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::default(),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        None,
        Duration::from_secs(1),
        Duration::from_secs(120),
    );

    // Session relies on echo to verify if a session is alive
    ctx.start_worker(Address::from_string("echo"), MockEchoer::new())?;

    mock_replacer
        .lock()
        .await
        .succeeds
        .store(false, Ordering::Relaxed);

    // Start the Session in a separate task
    session.start_monitoring()?;

    ctx.sleep(Duration::from_millis(250)).await;

    assert!(session.last_outcome().is_none());
    assert_eq!(session.connection_status(), ConnectionStatus::Down);
    assert!(session.is_being_replaced());
    assert!(mock_replacer
        .lock()
        .await
        .create_called
        .load(Ordering::Relaxed));

    // Now we allow the replacer to return and replace the route
    mock_replacer
        .lock()
        .await
        .succeeds
        .store(true, Ordering::Relaxed);

    let mut time_to_restore = 0;

    loop {
        if session.connection_status() == ConnectionStatus::Up {
            assert!(!session.is_being_replaced());
            assert!(session.last_outcome().is_some());
            break;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        time_to_restore += 1;
        continue;
    }

    // It can't take less than it takes to create that session
    assert!(time_to_restore >= 4);
    // It shouldn't take longer than it takes to create that session + retry_delay
    assert!(time_to_restore <= 16);

    // Shut down the test
    session.stop().await;

    Ok(())
}

#[allow(non_snake_case)]
#[ockam::test]
async fn start_monitoring__go_down__should_notice(ctx: &mut Context) -> Result<()> {
    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::default(),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        None,
        Duration::from_secs(120),
        Duration::from_secs(1),
    );

    // Session relies on echo to verify if a session is alive
    let echoer = MockEchoer::new();
    let echoer_responsive = echoer.responsive.clone();
    ctx.start_worker(Address::from_string("echo"), echoer)?;

    session.initial_connect().await?;

    // Start the Session in a separate task
    session.start_monitoring()?;

    ctx.sleep(Duration::from_secs(5)).await;

    assert!(session.last_outcome().is_some());
    assert_eq!(session.connection_status(), ConnectionStatus::Up);
    assert!(!session.is_being_replaced());

    echoer_responsive.store(false, Ordering::Relaxed);
    mock_replacer
        .lock()
        .await
        .succeeds
        .store(false, Ordering::Relaxed);

    let mut time_to_go_down = 0;

    loop {
        if session.connection_status() == ConnectionStatus::Down {
            break;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        time_to_go_down += 1;
        continue;
    }

    // It can't take less than 3 pings to notice that the session is down
    assert!(time_to_go_down >= 29);
    // It shouldn't take longer than 3 pings + some delay to notice that the session is down
    assert!(time_to_go_down <= 45);

    // Shut down the test
    session.stop().await;

    Ok(())
}

#[allow(non_snake_case)]
#[ockam::test]
async fn start_monitoring__packet_lost__should_be_up(ctx: &mut Context) -> Result<()> {
    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::default(),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        None,
        Duration::from_secs(120),
        Duration::from_secs(1),
    );

    // Session relies on echo to verify if a session is alive
    let echoer = MockEchoer::new();
    let echoer_drop_every = echoer.drop_every.clone();
    ctx.start_worker(Address::from_string("echo"), echoer)?;

    echoer_drop_every.store(2, Ordering::Relaxed);

    session.initial_connect().await?;

    // Start the Session in a separate task
    session.start_monitoring()?;

    for _ in 0..100 {
        assert!(session.last_outcome().is_some());
        assert_eq!(session.connection_status(), ConnectionStatus::Up);
        assert!(!session.is_being_replaced());

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Shut down the test
    session.stop().await;

    Ok(())
}

#[allow(non_snake_case)]
#[ockam::test(timeout = 100_000)]
async fn start_monitoring__unstable_connection__should_be_resilient(
    ctx: &mut Context,
) -> Result<()> {
    let mock_replacer = Arc::new(ockam_node::compat::asynchronous::Mutex::new(
        MockReplacer::default(),
    ));

    let session_ctx = ctx.new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)?;

    // Create a new Session instance
    let mut session = Session::new(
        session_ctx,
        mock_replacer.clone(),
        None,
        Duration::from_secs(1),
        Duration::from_secs(1),
    );

    // Session relies on echo to verify if a session is alive
    let echoer = MockEchoer::new();
    let echoer_responsive = echoer.responsive.clone();
    ctx.start_worker(Address::from_string("echo"), echoer)?;

    session.initial_connect().await?;

    // Start the Session in a separate task
    session.start_monitoring()?;

    for _ in 0..5 {
        echoer_responsive.store(false, Ordering::Relaxed);
        mock_replacer
            .lock()
            .await
            .succeeds
            .store(false, Ordering::Relaxed);

        tokio::time::sleep(Duration::from_secs(4)).await;

        assert_eq!(session.connection_status(), ConnectionStatus::Down);

        echoer_responsive.store(true, Ordering::Relaxed);
        mock_replacer
            .lock()
            .await
            .succeeds
            .store(true, Ordering::Relaxed);

        tokio::time::sleep(Duration::from_secs(2)).await;

        assert_eq!(session.connection_status(), ConnectionStatus::Up);

        let sleep_secs: u64 = random();
        tokio::time::sleep(Duration::from_secs(sleep_secs % 10)).await;
    }

    // Shut down the test
    session.stop().await;

    Ok(())
}

// TODO: Check recreate is called
