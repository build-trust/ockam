use core::time::Duration;
use ockam_core::compat::rand::{self, Rng};
use ockam_core::{route, Result};
use ockam_node::workers::Echoer;
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport};

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn tcp_lifecycle__two_connections__should_both_work(ctx: &mut Context) -> Result<()> {
    let options = TcpListenerOptions::new();
    ctx.flow_controls()
        .add_consumer(&"echoer".into(), &options.spawner_flow_control_id());
    ctx.start_worker("echoer", Echoer)?;

    let transport = TcpTransport::create(ctx)?;
    let listener = transport.listen("127.0.0.1:0", options).await?;

    let msg1: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();
    let msg2: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();

    let tx_address1 = transport
        .connect(&listener.socket_string(), TcpConnectionOptions::new())
        .await?;

    let reply1: String = ctx
        .send_and_receive(route![tx_address1.clone(), "echoer"], msg1.clone())
        .await?;
    assert_eq!(reply1, msg1, "Should receive the same message");

    let tx_address2 = transport
        .connect(&listener.socket_string(), TcpConnectionOptions::new())
        .await?;
    let reply2: String = ctx
        .send_and_receive(route![tx_address2.clone(), "echoer"], msg2.clone())
        .await?;
    assert_eq!(reply2, msg2, "Should receive the same message");

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn tcp_lifecycle__disconnect__should_stop_worker(ctx: &mut Context) -> Result<()> {
    let options = TcpListenerOptions::new();
    ctx.flow_controls()
        .add_consumer(&"echoer".into(), &options.spawner_flow_control_id());
    ctx.start_worker("echoer", Echoer)?;

    let transport = TcpTransport::create(ctx)?;
    let listener = transport.listen("127.0.0.1:0", options).await?;

    let msg1: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();
    let msg2: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();
    let msg3: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();

    let connection1 = transport
        .connect(&listener.socket_string(), TcpConnectionOptions::new())
        .await?;

    let reply1: String = ctx
        .send_and_receive(route![connection1.clone(), "echoer"], msg1.clone())
        .await?;
    assert_eq!(reply1, msg1, "Should receive the same message");

    let connection2 = transport
        .connect(&listener.socket_string(), TcpConnectionOptions::new())
        .await?;
    let reply2: String = ctx
        .send_and_receive(route![connection2.clone(), "echoer"], msg2.clone())
        .await?;
    assert_eq!(reply2, msg2, "Should receive the same message");

    transport.disconnect(&connection1)?;
    let res = ctx
        .send(route![connection1.clone(), "echoer"], msg1.clone())
        .await;
    assert!(res.is_err(), "Should not send messages after disconnection");

    let reply3: String = ctx
        .send_and_receive(route![connection2.clone(), "echoer"], msg3.clone())
        .await?;
    assert_eq!(reply3, msg3, "Should receive the same message");

    transport.disconnect(&connection2)?;
    let res = ctx
        .send(route![connection2.clone(), "echoer"], msg3.clone())
        .await;
    assert!(res.is_err(), "Should not send messages after disconnection");
    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn tcp_lifecycle__stop_listener__should_stop_accepting_connections(
    ctx: &mut Context,
) -> Result<()> {
    let options = TcpListenerOptions::new();
    ctx.flow_controls()
        .add_consumer(&"echoer".into(), &options.spawner_flow_control_id());

    ctx.start_worker("echoer", Echoer)?;

    let transport = TcpTransport::create(ctx)?;
    let listener = transport.listen("127.0.0.1:0", options).await?;

    let msg1: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();
    let msg2: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(256)
        .map(char::from)
        .collect();

    let tx_address = transport
        .connect(listener.socket_string(), TcpConnectionOptions::new())
        .await?;

    let reply1: String = ctx
        .send_and_receive(route![tx_address.clone(), "echoer"], msg1.clone())
        .await?;
    assert_eq!(reply1, msg1, "Should receive the same message");

    transport.stop_listener(listener.processor_address())?;
    ctx.sleep(Duration::from_millis(10)).await;

    let res = transport
        .connect(listener.socket_string(), TcpConnectionOptions::new())
        .await;
    assert!(
        res.is_err(),
        "Should not accept connection after listener is stopped"
    );
    let reply2: String = ctx
        .send_and_receive(route![tx_address.clone(), "echoer"], msg2.clone())
        .await?;
    assert_eq!(reply2, msg2, "Should receive the same message");
    Ok(())
}
