use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Notify;

#[derive(Copy, Clone)]
pub enum ServerMode {
    Echo(usize, u8),
    Null(usize),
}

pub async fn start_server(bind_port: u16, tls: bool, mode: ServerMode) {
    let bind_address = SocketAddr::new(IpAddr::from([0, 0, 0, 0]), bind_port);
    let listener = tokio::net::TcpListener::bind(bind_address).await.unwrap();

    let acceptor = if tls {
        println!("Listening TLS on {}", listener.local_addr().unwrap());
        let self_signed = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let config = Arc::new(
            rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(
                    vec![
                        CertificateDer::from_pem_slice(self_signed.cert.pem().as_bytes()).unwrap(),
                    ],
                    PrivateKeyDer::try_from(self_signed.key_pair.serialize_der()).unwrap(),
                )
                .unwrap(),
        );
        Some(tokio_rustls::TlsAcceptor::from(config))
    } else {
        println!("Listening on {}", listener.local_addr().unwrap());
        None
    };

    loop {
        if let Ok((stream, _)) = listener.accept().await {
            stream.set_nodelay(true).unwrap();
            if let Some(acceptor) = &acceptor {
                let result = acceptor.accept(stream).await;
                match result {
                    Ok(stream) => {
                        let (r, w) = tokio::io::split(stream);
                        tokio::spawn(async move { server(r, w, mode).await });
                    }
                    Err(err) => {
                        println!("Failed to accept TLS connection: {:?}", err);
                    }
                }
            } else {
                let (r, w) = tokio::io::split(stream);
                tokio::spawn(async move { server(r, w, mode).await });
            }
        } else {
            println!("Failed to accept connection");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}

async fn server(
    stream_reader: impl AsyncRead + Unpin + Send + 'static,
    stream_writer: impl AsyncWrite + Unpin + Send + 'static,
    mode: ServerMode,
) {
    match mode {
        ServerMode::Echo(buffer_size, pool_size) => {
            fast_echo(stream_reader, stream_writer, buffer_size, pool_size).await;
        }
        ServerMode::Null(buffer_size) => {
            let mut stream_reader = stream_reader;
            let mut buffer = vec![0u8; buffer_size];
            loop {
                let result = stream_reader.read(&mut buffer).await;
                if let Err(_err) = result {
                    return;
                }
            }
        }
    }
}

// To make echo as fast as possible, we want both reading and writing to be done in parallel and
// avoid any allocations.
// The echo server has two queues of buffers: incoming and outgoing.
// Once the incoming buffer is ready, it's moved to the outgoing buffer queue.
// Once the outgoing buffer is ready, it's moved back to the incoming buffer queue.
async fn fast_echo(
    mut stream_reader: impl AsyncRead + Unpin + Send + 'static,
    mut stream_writer: impl AsyncWrite + Unpin + Send + 'static,
    buffer_size: usize,
    pool_size: u8,
) {
    let new_outgoing_buffer_event = Arc::new(Notify::new());
    let new_incoming_buffer_event = Arc::new(Notify::new());

    let outgoing_buffers = Arc::new(Mutex::new(VecDeque::with_capacity(pool_size as usize)));
    let incoming_buffers = {
        let mut queue = VecDeque::with_capacity(pool_size as usize);
        for _ in 0..queue.capacity() {
            queue.push_back(vec![0u8; buffer_size]);
        }
        Arc::new(Mutex::new(queue))
    };

    {
        let incoming_buffers = incoming_buffers.clone();
        let new_incoming_buffer = new_incoming_buffer_event.clone();
        let outgoing_buffers = outgoing_buffers.clone();
        let new_outgoing_buffer = new_outgoing_buffer_event.clone();

        tokio::spawn(async move {
            loop {
                let incoming_buffer = incoming_buffers.lock().unwrap().pop_front();
                if let Some(mut incoming_buffer) = incoming_buffer {
                    let result = stream_reader.read(&mut incoming_buffer).await;
                    match result {
                        Ok(bytes) => {
                            outgoing_buffers
                                .lock()
                                .unwrap()
                                .push_back((bytes, incoming_buffer));
                            new_outgoing_buffer.notify_one();
                        }
                        Err(_err) => {
                            return;
                        }
                    }
                } else {
                    new_incoming_buffer.notified().await;
                }
            }
        });
    }

    loop {
        let outgoing_buffer = outgoing_buffers.lock().unwrap().pop_front();
        if let Some((bytes, outgoing_buffer)) = outgoing_buffer {
            let result = stream_writer.write_all(&outgoing_buffer[..bytes]).await;
            match result {
                Ok(()) => {
                    incoming_buffers.lock().unwrap().push_back(outgoing_buffer);
                    new_incoming_buffer_event.notify_one();
                }
                Err(_err) => {
                    return;
                }
            }
        } else {
            new_outgoing_buffer_event.notified().await;
        }
    }
}
