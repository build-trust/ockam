use ockam_core::compat::rand::{self, Rng};
use ockam_core::{route, Result, Routed, Worker};
use ockam_node::{Context, MessageSendReceiveOptions};
use ockam_transport_core::MAXIMUM_MESSAGE_LENGTH;
use ockam_transport_udp::{UdpBindArguments, UdpBindOptions, UdpTransport, UDP};
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{debug, error, trace};

mod utils;

const TIMEOUT: Duration = Duration::from_secs(5);

/// When acting as a server, the transport should reply using the same
/// UDP port that we sent to.
#[ockam_macros::test]
async fn reply_from_correct_server_port(ctx: &mut Context) -> Result<()> {
    // Transport
    let transport = UdpTransport::create(ctx)?;

    // Listener
    ctx.start_worker("echoer", Echoer::new(true))?;
    let bind = transport
        .bind(UdpBindArguments::new(), UdpBindOptions::new())
        .await?;

    ctx.flow_controls()
        .add_consumer(&"echoer".into(), bind.flow_control_id());

    // Sender
    {
        let route = route![
            bind.sender_address().clone(),
            (UDP, bind.bind_address().to_string()),
            "echoer"
        ];

        let res: Routed<String> = ctx
            .send_and_receive_extended(
                route,
                String::from("Hola"),
                MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
            )
            .await?;

        trace!(return_route = %res.return_route());

        let src_addr = res
            .return_route()
            .iter()
            .find(|x| x.transport_type() == UDP)
            .map(|x| x.address().parse::<SocketAddr>().unwrap())
            .unwrap();

        assert_eq!(
            src_addr.port(),
            bind.bind_address().port(),
            "Reply message does not come from port we sent to"
        );
    };

    Ok(())
}

/// The transport should still allow sending of messages
/// even after a send socket error.
///
/// Examples of errors are when an IPv4 socket is asked to send to
/// an IPv6 address, or when we ask an IPv4 socket to send to port 0.
#[ockam_macros::test]
async fn recover_from_sender_error(ctx: &mut Context) -> Result<()> {
    // Find an available port
    let addr_ok = utils::available_local_ports(1)
        .await?
        .first()
        .unwrap()
        .to_string();
    let addr_nok = "192.168.1.10:0";
    debug!("addr_ok = {:?}", addr_ok);
    debug!("addr_nok = {:?}", addr_nok);

    // Transport
    let transport = UdpTransport::create(ctx)?;

    // Listener
    ctx.start_worker("echoer", Echoer::new(true))?;
    let bind = transport
        .bind(
            UdpBindArguments::new().with_bind_address(addr_ok.clone())?,
            UdpBindOptions::new(),
        )
        .await?;
    ctx.flow_controls()
        .add_consumer(&"echoer".into(), bind.flow_control_id());

    // Send message to try and cause a socket send error
    let r = route![bind.sender_address().clone(), (UDP, addr_nok), "echoer"];
    let res: Result<Routed<String>> = ctx
        .send_and_receive_extended(
            r,
            String::from("Hola"),
            MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
        )
        .await;
    assert!(res.is_err(), "Expected an error sending");

    // Send message to working peer
    let r = route![bind.sender_address().clone(), (UDP, addr_ok), "echoer"];
    let res: Result<Routed<String>> = ctx
        .send_and_receive_extended(
            r,
            String::from("Hola"),
            MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
        )
        .await;
    assert!(res.is_ok(), "Should have been able to send message");

    Ok(())
}

/// The transport should send messages to peers, with different
/// destination addresses, from the same UDP port.
///
/// This is important for NAT puncture.
#[ockam_macros::test]
async fn send_from_same_client_port(ctx: &mut Context) -> Result<()> {
    // Find available ports
    let bind_addrs = utils::available_local_ports(2).await?;
    debug!("bind_addrs = {:?}", bind_addrs);

    // Transport
    let transport = UdpTransport::create(ctx)?;

    // Listeners
    // Note: it is the Echoer which is checking the UDP ports for this test
    ctx.start_worker("echoer", Echoer::new(true))?;
    let mut binds = vec![];
    for addr in &bind_addrs {
        let bind = transport
            .bind(
                UdpBindArguments::new().with_bind_address(addr.to_string())?,
                UdpBindOptions::new(),
            )
            .await?;

        ctx.flow_controls()
            .add_consumer(&"echoer".into(), bind.flow_control_id());

        binds.push(bind);
    }

    // Send messages
    for addr in &bind_addrs {
        let msg = String::from("Ockam. Testing. 1, 2, 3...");
        let r = route![
            binds[0].sender_address().clone(),
            (UDP, addr.to_string()),
            "echoer"
        ];
        let reply = ctx
            .send_and_receive_extended::<String>(
                r,
                msg.clone(),
                MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
            )
            .await?
            .into_body()?;
        assert_eq!(reply, msg, "Should receive the same message");
    }

    Ok(())
}

#[ockam_macros::test]
async fn send_receive_arbitrary_udp_peer(ctx: &mut Context) -> Result<()> {
    // Transport
    let transport = UdpTransport::create(ctx)?;

    ctx.start_worker("echoer", Echoer::new(true))?;
    let bind1 = transport
        .bind(UdpBindArguments::new(), UdpBindOptions::new())
        .await?;
    let bind2 = transport
        .bind(UdpBindArguments::new(), UdpBindOptions::new())
        .await?;
    let bind3 = transport
        .bind(UdpBindArguments::new(), UdpBindOptions::new())
        .await?;

    ctx.flow_controls()
        .add_consumer(&"echoer".into(), bind2.flow_control_id());
    ctx.flow_controls()
        .add_consumer(&"echoer".into(), bind3.flow_control_id());

    // Sender
    {
        for _ in 0..3 {
            let msg: String = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(256)
                .map(char::from)
                .collect();

            let r = route![
                bind1.sender_address().clone(),
                (UDP, bind2.bind_address().to_string()),
                "echoer"
            ];
            let reply = ctx
                .send_and_receive_extended::<String>(
                    r,
                    msg.clone(),
                    MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
                )
                .await?
                .into_body()?;

            assert_eq!(reply, msg, "Should receive the same message");

            let r = route![
                bind1.sender_address().clone(),
                (UDP, bind3.bind_address().to_string()),
                "echoer"
            ];
            let reply = ctx
                .send_and_receive_extended::<String>(
                    r,
                    msg.clone(),
                    MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
                )
                .await?
                .into_body()?;

            assert_eq!(reply, msg, "Should receive the same message");
        }
    };
    Ok(())
}

#[ockam_macros::test]
async fn send_receive_one_known_udp_peer(ctx: &mut Context) -> Result<()> {
    // Transport
    let transport = UdpTransport::create(ctx)?;

    ctx.start_worker("echoer", Echoer::new(false))?;
    let bind1 = transport
        .bind(UdpBindArguments::new(), UdpBindOptions::new())
        .await?;
    let bind2 = transport
        .bind(
            UdpBindArguments::new()
                .with_peer_address(bind1.bind_address().to_string())
                .await?,
            UdpBindOptions::new(),
        )
        .await?;

    ctx.flow_controls()
        .add_consumer(&"echoer".into(), bind1.flow_control_id());
    ctx.flow_controls()
        .add_consumer(&"echoer".into(), bind2.flow_control_id());

    // Sender
    {
        for _ in 0..3 {
            let msg: String = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(256)
                .map(char::from)
                .collect();

            let r = route![bind2.sender_address().clone(), "echoer"];
            let reply = ctx
                .send_and_receive_extended::<String>(
                    r,
                    msg.clone(),
                    MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
                )
                .await?
                .into_body()?;

            assert_eq!(reply, msg, "Should receive the same message");

            let r = route![
                bind1.sender_address().clone(),
                (UDP, bind2.bind_address().to_string()),
                "echoer"
            ];
            let reply = ctx
                .send_and_receive_extended::<String>(
                    r,
                    msg.clone(),
                    MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
                )
                .await?
                .into_body()?;

            assert_eq!(reply, msg, "Should receive the same message");
        }
    };
    Ok(())
}

#[ockam_macros::test]
async fn send_receive_two_known_udp_peers(ctx: &mut Context) -> Result<()> {
    // Find available ports
    let bind_addrs = utils::available_local_ports(2).await?;
    debug!("bind_addrs = {:?}", bind_addrs);

    // Transport
    let transport = UdpTransport::create(ctx)?;

    ctx.start_worker("echoer", Echoer::new(false))?;
    let bind1 = transport
        .bind(
            UdpBindArguments::new()
                .with_bind_address(bind_addrs[0].to_string())?
                .with_peer_address(bind_addrs[1].to_string())
                .await?,
            UdpBindOptions::new(),
        )
        .await?;
    let bind2 = transport
        .bind(
            UdpBindArguments::new()
                .with_bind_address(bind_addrs[1].to_string())?
                .with_peer_address(bind_addrs[0].to_string())
                .await?,
            UdpBindOptions::new(),
        )
        .await?;

    ctx.flow_controls()
        .add_consumer(&"echoer".into(), bind1.flow_control_id());
    ctx.flow_controls()
        .add_consumer(&"echoer".into(), bind2.flow_control_id());

    // Sender
    {
        for _ in 0..3 {
            let msg: String = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(256)
                .map(char::from)
                .collect();

            let r = route![bind2.sender_address().clone(), "echoer"];
            let reply = ctx
                .send_and_receive_extended::<String>(
                    r,
                    msg.clone(),
                    MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
                )
                .await?
                .into_body()?;

            assert_eq!(reply, msg, "Should receive the same message");

            let r = route![bind1.sender_address().clone(), "echoer"];
            let reply = ctx
                .send_and_receive_extended::<String>(
                    r,
                    msg.clone(),
                    MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
                )
                .await?
                .into_body()?;

            assert_eq!(reply, msg, "Should receive the same message");
        }
    };
    Ok(())
}

#[ockam_macros::test]
async fn send_receive_large_message(ctx: &mut Context) -> Result<()> {
    // Find available ports
    let bind_addrs = utils::available_local_ports(2).await?;
    debug!("bind_addrs = {:?}", bind_addrs);

    // Transport
    let transport = UdpTransport::create(ctx)?;

    ctx.start_worker("echoer", Echoer::new(false))?;
    let bind1 = transport
        .bind(
            UdpBindArguments::new()
                .with_bind_address(bind_addrs[0].to_string())?
                .with_peer_address(bind_addrs[1].to_string())
                .await?,
            UdpBindOptions::new(),
        )
        .await?;
    let bind2 = transport
        .bind(
            UdpBindArguments::new()
                .with_bind_address(bind_addrs[1].to_string())?
                .with_peer_address(bind_addrs[0].to_string())
                .await?,
            UdpBindOptions::new(),
        )
        .await?;

    ctx.flow_controls()
        .add_consumer(&"echoer".into(), bind1.flow_control_id());

    let msg: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(MAXIMUM_MESSAGE_LENGTH)
        .map(char::from)
        .collect();

    let r = route![bind2.sender_address().clone(), "echoer"];
    let reply = ctx
        .send_and_receive_extended::<String>(
            r,
            msg.clone(),
            MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
        )
        .await?
        .into_body()?;

    assert_eq!(reply, msg, "Should receive the same message");

    Ok(())
}

pub struct Echoer {
    check_sender_is_the_same: bool,
    prev_src_addr: Option<String>,
}

impl Echoer {
    fn new(check_sender_is_the_same: bool) -> Self {
        Self {
            check_sender_is_the_same,
            prev_src_addr: None,
        }
    }
}

#[ockam_core::worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        if self.check_sender_is_the_same {
            // Get source UDP address
            let src_addr = match msg
                .return_route()
                .iter()
                .find(|x| x.transport_type() == UDP)
            {
                Some(addr) => String::from(addr.address()),
                None => {
                    error!(
                        "TEST FAIL: Failed to find UDP source address: {:?}",
                        &msg.return_route()
                    );
                    panic!("TEST FAIL: Failed to find UDP source address");
                }
            };

            // Check source address matches previous received messages
            // This is part of the testing
            match &self.prev_src_addr {
                Some(addr) => {
                    if addr != &src_addr {
                        error!(
                        "TEST FAIL: Source UDP address does not match previous messages: prev {}, now {}",
                        addr, src_addr
                    );
                        panic!("TEST FAIL: Source UDP address does not match previous messages");
                    }
                }
                None => self.prev_src_addr = Some(src_addr),
            }
        }

        debug!("Replying back to {}", &msg.return_route());
        ctx.send(msg.return_route().clone(), msg.into_body()?).await
    }
}
