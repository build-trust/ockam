use ockam_core::compat::rand::{self, Rng};
use ockam_core::{route, Address, AllowAll, Result, Routed, Worker};
use ockam_node::{Context, MessageReceiveOptions, MessageSendReceiveOptions};
use ockam_transport_udp::{UdpTransport, UDP};
use std::net::SocketAddr;
use std::time::Duration;
use tracing::{debug, error, trace};

mod utils;

const TIMEOUT: Duration = Duration::from_secs(5);

/// When acting as a server, the transport should reply using the same
/// UDP port that we sent to.
#[ockam_macros::test]
async fn reply_from_correct_server_port(ctx: &mut Context) -> Result<()> {
    // Find an available port
    let bind_addr = *utils::available_local_ports(1).await?.first().unwrap();
    debug!("bind_addr = {:?}", bind_addr);

    // Transport
    let transport = UdpTransport::create(ctx).await?;

    // Listener
    {
        ctx.start_worker("echoer", Echoer::new()).await?;
        transport.listen(bind_addr.to_string()).await?;
    };

    // Sender
    {
        let route = route![(UDP, bind_addr.to_string()), "echoer"];
        let mut child_ctx = ctx
            .new_detached(Address::random_tagged("App.detached"), AllowAll, AllowAll)
            .await?;

        child_ctx.send(route, String::from("Hola")).await?;
        let res = child_ctx
            .receive_extended::<String>(MessageReceiveOptions::new().with_timeout(TIMEOUT))
            .await?;

        trace!(return_route = %res.return_route());

        let src_addr = res
            .return_route()
            .iter()
            .find(|x| x.transport_type() == UDP)
            .map(|x| x.address().parse::<SocketAddr>().unwrap())
            .unwrap();

        assert!(
            src_addr.port() == bind_addr.port(),
            "Reply message does not come from port we sent to"
        );
    };

    ctx.stop().await?;
    Ok(())
}

/// The transport should still allow sending of mesages
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
    let transport = UdpTransport::create(ctx).await?;

    // Listener
    ctx.start_worker("echoer", Echoer::new()).await?;
    transport.listen(addr_ok.clone()).await?;

    // Send message to try and cause a socket send error
    let r = route![(UDP, addr_nok), "echoer"];
    let res: Result<Routed<String>> = ctx
        .send_and_receive_extended(
            r,
            String::from("Hola"),
            MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
        )
        .await;
    assert!(res.is_err(), "Expected an error sending");

    // Send message to working peer
    let r = route![(UDP, addr_ok), "echoer"];
    let res: Result<Routed<String>> = ctx
        .send_and_receive_extended(
            r,
            String::from("Hola"),
            MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
        )
        .await;
    assert!(res.is_ok(), "Should have been able to send message");

    ctx.stop().await?;
    Ok(())
}

/// The transport should send messages to peers, with different
/// destination addresses, from the same UDP port.
///
/// This is important fot NAT hole punching.
#[ockam_macros::test]
async fn send_from_same_client_port(ctx: &mut Context) -> Result<()> {
    // Find available ports
    let bind_addrs = utils::available_local_ports(2).await?;
    debug!("bind_addrs = {:?}", bind_addrs);

    // Transport
    let transport = UdpTransport::create(ctx).await?;

    // Listeners
    // Note: it is the Echoer which is checking the UDP ports for this test
    ctx.start_worker("echoer", Echoer::new()).await?;
    for addr in &bind_addrs {
        transport.listen(addr.to_string()).await?;
    }

    // Send messages
    for addr in &bind_addrs {
        let msg = String::from("Ockam. Testing. 1, 2, 3...");
        let r = route![(UDP, addr.to_string()), "echoer"];
        let reply = ctx
            .send_and_receive_extended::<String>(
                r,
                msg.clone(),
                MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
            )
            .await?
            .body();
        assert_eq!(reply, msg, "Should receive the same message");
    }

    ctx.stop().await?;
    Ok(())
}

#[ockam_macros::test]
async fn send_receive(ctx: &mut Context) -> Result<()> {
    // Find an available port
    let bind_addr = utils::available_local_ports(1)
        .await?
        .first()
        .unwrap()
        .to_string();
    debug!("bind_addr = {:?}", bind_addr);

    // Transport
    let transport = UdpTransport::create(ctx).await?;

    // Listener
    {
        ctx.start_worker("echoer", Echoer::new()).await?;
        transport.listen(bind_addr.clone()).await?;
    };

    // Sender
    {
        for _ in 0..3 {
            let msg: String = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(256)
                .map(char::from)
                .collect();
            let r = route![(UDP, bind_addr.clone()), "echoer"];
            let reply = ctx
                .send_and_receive_extended::<String>(
                    r,
                    msg.clone(),
                    MessageSendReceiveOptions::new().with_timeout(TIMEOUT),
                )
                .await?
                .body();

            assert_eq!(reply, msg, "Should receive the same message");
        }
    };

    ctx.stop().await?;
    Ok(())
}

pub struct Echoer {
    prev_src_addr: Option<String>,
}

impl Echoer {
    fn new() -> Self {
        Self {
            prev_src_addr: None,
        }
    }
}

#[ockam_core::worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
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

        debug!("Replying back to {}", &msg.return_route());
        ctx.send(msg.return_route(), msg.body()).await
    }
}
