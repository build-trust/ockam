use ockam_core::compat::net::SocketAddr;
use ockam_core::flow_control::{FlowControlId, FlowControlPolicy, FlowControls};
use ockam_core::{route, Address, AllowAll, Result, Route};
use ockam_identity::{Identity, SecureChannelListenerTrustOptions, SecureChannelTrustOptions};
use ockam_node::{Context, MessageReceiveOptions};
use ockam_transport_tcp::{TcpConnectionTrustOptions, TcpListenerTrustOptions, TcpTransport};
use ockam_vault::Vault;
use rand::random;

#[allow(dead_code)]
pub async fn message_should_pass(ctx: &Context, address: &Address) -> Result<()> {
    check_message_flow(ctx, route![address.clone()], true).await
}

#[allow(dead_code)]
pub async fn message_should_not_pass(ctx: &Context, address: &Address) -> Result<()> {
    check_message_flow(ctx, route![address.clone()], false).await
}

async fn check_message_flow(ctx: &Context, route: Route, should_pass: bool) -> Result<()> {
    let address = Address::random_local();
    let mut receiving_ctx = ctx
        .new_detached(address.clone(), AllowAll, AllowAll)
        .await?;

    let msg: [u8; 4] = random();
    let msg = hex::encode(msg);
    ctx.send(route![route, address], msg.clone()).await?;

    if should_pass {
        let msg_received = receiving_ctx.receive::<String>().await?.body();
        assert_eq!(msg_received, msg);
    } else {
        let res = receiving_ctx
            .receive_extended::<String>(MessageReceiveOptions::new().with_timeout_secs(1))
            .await;
        assert!(res.is_err(), "Messages should not pass for given route");
    }

    Ok(())
}

#[allow(dead_code)]
pub async fn message_should_pass_with_ctx(
    ctx: &Context,
    address: &Address,
    receiving_ctx: &mut Context,
) -> Result<()> {
    check_message_flow_with_ctx(ctx, address, receiving_ctx, true).await
}

#[allow(dead_code)]
pub async fn message_should_not_pass_with_ctx(
    ctx: &Context,
    address: &Address,
    receiving_ctx: &mut Context,
) -> Result<()> {
    check_message_flow_with_ctx(ctx, address, receiving_ctx, false).await
}

async fn check_message_flow_with_ctx(
    ctx: &Context,
    address: &Address,
    receiving_ctx: &mut Context,
    should_pass: bool,
) -> Result<()> {
    let msg: [u8; 4] = random();
    let msg = hex::encode(msg);
    ctx.send(
        route![address.clone(), receiving_ctx.address()],
        msg.clone(),
    )
    .await?;

    if should_pass {
        let msg_received = receiving_ctx.receive::<String>().await?.body();
        assert_eq!(msg_received, msg);
    } else {
        let res = receiving_ctx
            .receive_extended::<String>(MessageReceiveOptions::new().with_timeout_secs(1))
            .await;
        assert!(res.is_err(), "Messages should not pass for given route");
    }

    Ok(())
}

#[allow(dead_code)]
pub struct TcpListenerInfo {
    pub tcp: TcpTransport,
    pub socket_addr: SocketAddr,
    pub flow_control: Option<(FlowControls, FlowControlId)>,
}

impl TcpListenerInfo {
    #[allow(dead_code)]
    pub fn get_connection(&self) -> TcpConnectionInfo {
        let senders = self.tcp.registry().get_all_sender_workers();
        assert_eq!(senders.len(), 1);

        let sender = senders.first().unwrap().clone();

        let flow_control = match &self.flow_control {
            Some((flow_controls, _flow_control_id)) => {
                let receivers = self.tcp.registry().get_all_receiver_processors();
                assert_eq!(receivers.len(), 1);
                let receiver = receivers.first().unwrap();
                let flow_control_id = flow_controls.get_flow_control_with_producer(receiver);
                Some((
                    flow_controls.clone(),
                    flow_control_id
                        .map(|x| x.flow_control_id().clone())
                        .unwrap(),
                ))
            }
            None => None,
        };

        TcpConnectionInfo {
            address: sender,
            flow_control,
        }
    }
}

#[allow(dead_code)]
pub async fn create_tcp_listener_with_flow_control(ctx: &Context) -> Result<TcpListenerInfo> {
    create_tcp_listener(ctx, true).await
}

#[allow(dead_code)]
pub async fn create_tcp_listener_without_flow_control(ctx: &Context) -> Result<TcpListenerInfo> {
    create_tcp_listener(ctx, false).await
}

async fn create_tcp_listener(ctx: &Context, with_flow_control: bool) -> Result<TcpListenerInfo> {
    let tcp = TcpTransport::create(ctx).await?;
    let (socket_addr, flow_control) = if with_flow_control {
        let flow_controls = FlowControls::default();
        let flow_control_id = flow_controls.generate_id();
        let trust_options = TcpListenerTrustOptions::as_spawner(&flow_controls, &flow_control_id);
        let (socket_addr, _) = tcp.listen("127.0.0.1:0", trust_options).await?;
        (socket_addr, Some((flow_controls, flow_control_id)))
    } else {
        let (socket_addr, _) = tcp
            .listen("127.0.0.1:0", TcpListenerTrustOptions::new())
            .await?;
        (socket_addr, None)
    };

    let info = TcpListenerInfo {
        tcp,
        socket_addr,
        flow_control,
    };

    Ok(info)
}

#[allow(dead_code)]
pub struct TcpConnectionInfo {
    pub address: Address,
    pub flow_control: Option<(FlowControls, FlowControlId)>,
}

#[allow(dead_code)]
pub async fn create_tcp_connection_with_flow_control(
    ctx: &Context,
    socket_addr: &SocketAddr,
) -> Result<TcpConnectionInfo> {
    create_tcp_connection(ctx, socket_addr, true).await
}

#[allow(dead_code)]
pub async fn create_tcp_connection_without_flow_control(
    ctx: &Context,
    socket_addr: &SocketAddr,
) -> Result<TcpConnectionInfo> {
    create_tcp_connection(ctx, socket_addr, false).await
}

async fn create_tcp_connection(
    ctx: &Context,
    socket_addr: &SocketAddr,
    with_flow_control: bool,
) -> Result<TcpConnectionInfo> {
    let tcp = TcpTransport::create(ctx).await?;
    let (address, flow_control) = if with_flow_control {
        let flow_controls = FlowControls::default();
        let flow_control_id = flow_controls.generate_id();
        let trust_options =
            TcpConnectionTrustOptions::as_producer(&flow_controls, &flow_control_id);
        let address = tcp.connect(socket_addr.to_string(), trust_options).await?;
        (address, Some((flow_controls, flow_control_id)))
    } else {
        let address = tcp
            .connect(socket_addr.to_string(), TcpConnectionTrustOptions::new())
            .await?;
        (address, None)
    };

    let info = TcpConnectionInfo {
        address,
        flow_control,
    };

    Ok(info)
}

#[allow(dead_code)]
pub struct SecureChannelListenerInfo {
    pub identity: Identity,
}

impl SecureChannelListenerInfo {
    #[allow(dead_code)]
    pub fn get_channel(&self) -> Address {
        self.identity
            .secure_channel_registry()
            .get_channel_list()
            .first()
            .unwrap()
            .encryptor_messaging_address()
            .clone()
    }
}

#[allow(dead_code)]
pub async fn create_secure_channel_listener(
    ctx: &Context,
    flow_control: &Option<(FlowControls, FlowControlId)>,
    with_tcp_listener: bool,
) -> Result<SecureChannelListenerInfo> {
    let identity = Identity::create(ctx, Vault::create()).await?;

    let trust_options = SecureChannelListenerTrustOptions::new();
    let trust_options = if let Some((flow_controls, flow_control_id)) = flow_control {
        let policy = if with_tcp_listener {
            FlowControlPolicy::SpawnerAllowOnlyOneMessage
        } else {
            FlowControlPolicy::ProducerAllowMultiple
        };

        trust_options.as_consumer_with_flow_control_id(flow_controls, flow_control_id, policy)
    } else {
        trust_options
    };

    identity
        .create_secure_channel_listener("listener", trust_options)
        .await?;

    let info = SecureChannelListenerInfo { identity };

    Ok(info)
}

#[allow(dead_code)]
pub struct SecureChannelInfo {
    pub identity: Identity,
    pub address: Address,
}

#[allow(dead_code)]
pub async fn create_secure_channel(
    ctx: &Context,
    connection: &TcpConnectionInfo,
) -> Result<SecureChannelInfo> {
    let identity = Identity::create(ctx, Vault::create()).await?;

    let trust_options = SecureChannelTrustOptions::new();
    let trust_options = if let Some((flow_controls, _flow_control_id)) = &connection.flow_control {
        trust_options.as_consumer(flow_controls)
    } else {
        trust_options
    };

    let address = identity
        .create_secure_channel(
            route![connection.address.clone(), "listener"],
            trust_options,
        )
        .await?;

    let info = SecureChannelInfo { identity, address };

    Ok(info)
}
