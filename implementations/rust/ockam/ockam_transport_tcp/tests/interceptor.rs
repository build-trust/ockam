use ockam_core::{async_trait, route, AllowAll};
use ockam_node::Context;
use ockam_transport_tcp::{
    read_portal_payload_length, Direction, PortalInletInterceptor, PortalInterceptor,
    PortalInterceptorFactory, TcpInletOptions, TcpOutletOptions, TcpTransport,
};
use rand::random;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Default)]
struct MockPortalInterceptor {
    from_inlet_to_outlet: Arc<Mutex<Vec<u8>>>,
    from_outlet_to_inlet: Arc<Mutex<Vec<u8>>>,
}

#[async_trait]
impl PortalInterceptor for MockPortalInterceptor {
    async fn intercept(
        &self,
        _context: &mut Context,
        direction: Direction,
        buffer: &[u8],
    ) -> ockam_core::Result<Option<Vec<u8>>> {
        let mut guard = match direction {
            Direction::FromInletToOutlet => self.from_inlet_to_outlet.lock().unwrap(),
            Direction::FromOutletToInlet => self.from_outlet_to_inlet.lock().unwrap(),
        };
        guard.extend_from_slice(buffer);
        Ok(Some(buffer.to_vec()))
    }
}

struct MockPortalInterceptorFactory {
    interceptor: Arc<MockPortalInterceptor>,
}

impl PortalInterceptorFactory for MockPortalInterceptorFactory {
    fn create(&self) -> Arc<dyn PortalInterceptor> {
        self.interceptor.clone()
    }
}

async fn setup(
    context: &mut Context,
) -> ockam_core::Result<(String, TcpListener, Arc<MockPortalInterceptor>)> {
    let tcp = TcpTransport::create(context)?;

    let listener = {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bind_address = listener.local_addr().unwrap().to_string();
        tcp.create_outlet(
            "outlet",
            bind_address.try_into().unwrap(),
            TcpOutletOptions::new(),
        )?;
        listener
    };

    let mock_portal_interceptor = Arc::new(MockPortalInterceptor::default());

    PortalInletInterceptor::create(
        context,
        "interceptor_listener".into(),
        Arc::new(MockPortalInterceptorFactory {
            interceptor: mock_portal_interceptor.clone(),
        }),
        Arc::new(AllowAll),
        Arc::new(AllowAll),
        read_portal_payload_length(),
    )
    .unwrap();

    let inlet = tcp
        .create_inlet(
            "127.0.0.1:0",
            route!["interceptor_listener", "outlet"],
            TcpInletOptions::new(),
        )
        .await?;

    Ok((
        inlet.socket_address().to_string(),
        listener,
        mock_portal_interceptor,
    ))
}

const LENGTH: usize = 32;

fn generate_binary() -> [u8; LENGTH] {
    random()
}

async fn write_binary(stream: &mut TcpStream, payload: [u8; LENGTH]) {
    stream.write_all(&payload).await.unwrap();
}

async fn read_assert_binary(stream: &mut TcpStream, expected_payload: [u8; LENGTH]) {
    let mut payload = [0u8; LENGTH];
    let length = stream.read_exact(&mut payload).await.unwrap();
    assert_eq!(length, LENGTH);
    assert_eq!(payload, expected_payload);
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 5_000)]
async fn interceptor__simple_payload__received(context: &mut Context) -> ockam_core::Result<()> {
    let payload1 = generate_binary();
    let payload2 = generate_binary();

    let (inlet_addr, listener, mock_portal_interceptor) = setup(context).await?;

    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        read_assert_binary(&mut stream, payload1).await;
        write_binary(&mut stream, payload2).await;

        stream
    });

    // Wait till the listener is up
    tokio::time::sleep(Duration::from_millis(250)).await;

    let mut stream = TcpStream::connect(inlet_addr).await.unwrap();
    write_binary(&mut stream, payload1).await;
    read_assert_binary(&mut stream, payload2).await;

    let res = handle.await;
    assert!(res.is_ok());

    let from_inlet_to_outlet = mock_portal_interceptor
        .from_inlet_to_outlet
        .lock()
        .unwrap()
        .to_vec();
    let from_outlet_to_inlet = mock_portal_interceptor
        .from_outlet_to_inlet
        .lock()
        .unwrap()
        .to_vec();
    assert_eq!(from_inlet_to_outlet.as_slice(), payload1);
    assert_eq!(from_outlet_to_inlet.as_slice(), payload2);

    Ok(())
}
