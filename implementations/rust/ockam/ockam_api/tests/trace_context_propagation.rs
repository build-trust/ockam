mod common;

use crate::common::trace_code::*;
use itertools::Itertools;
use ockam::identity::{SecureChannelListenerOptions, SecureChannelOptions};
use ockam::node;
use ockam_api::echoer::Echoer;
use ockam_core::{route, Address, AsyncTryClone, OpenTelemetryContext, Routed, Worker};
use ockam_node::Context;
use ockam_transport_core::Transport;
use ockam_transport_tcp::{TcpListenerOptions, TcpTransportExtension, TCP};
use std::time::Duration;
use tonic::async_trait;
use tracing::instrument;

/// These tests need to be integration tests
/// They need to run in isolation because
/// they set up some global spans / logs exporters that might interact with other tests

/// This test checks that the tracing context that we propagate to other systems contains
/// a proper span id when spans are created with instrumented methods
#[test]
fn test_context_propagation_across_instrumented_methods() {
    let (propagated_context, mut spans) = trace_code(|_| function());
    spans.reverse();

    // there must be 3 spans
    assert_eq!(
        spans.len(),
        3,
        "{}",
        spans.iter().map(|s| s.name.to_string()).join(", ")
    );
    let span1 = spans.first().unwrap();
    let span2 = spans.get(1).unwrap();
    let span3 = spans.get(2).unwrap();

    // the spans must have proper parent / child relationships
    assert_eq!(span1.name, "root");

    assert_eq!(span2.name, "function");
    assert_eq!(span2.parent_span_id, span1.span_context.span_id());

    assert_eq!(span3.name, "nested_function");
    assert_eq!(span3.parent_span_id, span2.span_context.span_id());

    // the propagated context must use the span id of the most nested span id
    let context = propagated_context.as_map();
    let traceparent = context.get("traceparent").unwrap();
    assert_eq!(
        traceparent.to_string(),
        format!(
            "00-{}-{}-01",
            span1.span_context.trace_id(),
            span3.span_context.span_id()
        )
    );
}

#[tracing::instrument]
async fn function() -> OpenTelemetryContext {
    nested_function().await
}

#[tracing::instrument]
async fn nested_function() -> OpenTelemetryContext {
    OpenTelemetryContext::current()
}

/// This test checks that the tracing context is correctly propagated intra-nodes
/// when several workers are involved and inter-nodes when TransportMessages are sent
#[test]
fn test_context_propagation_across_workers_and_nodes() {
    let (received, spans) = trace_code(|ctx| send_echo_message(ctx, "hello"));
    assert_eq!(received.unwrap(), "hello".to_string());

    // There are 3 traces:
    //  - 2 for the TCP processors, just waiting to process incoming TCP data
    //  - 1 for the sent message
    let actual = display_traces(spans);
    let expected = r#"
TcpTransportMessage::start_trace
└── TcpRecvProcessor::process

TcpTransportMessage::start_trace
└── TcpRecvProcessor::process
    └── Echoer::handle_message
        └── TcpSendWorker::handle_message
            └── TcpTransportMessage::end_trace

root
└── send_echo_message
    ├── create tcp transport
    ├── TcpListenProcessor::start
    ├── create tcp transport
    ├── connect
    ├── TcpSendWorker::start
    ├── TcpRecvProcessor::start
    └── MessageSender::handle_message
        └── TcpSendWorker::handle_message
            └── TcpTransportMessage::end_trace
"#;
    pretty_assertions::assert_eq!(format!("\n{actual}"), expected);
}

/// This test checks that the tracing context is correctly propagated intra-nodes
/// when several workers are involved and inter-nodes when TransportMessages are sent
/// over a secure channel
#[test]
fn test_context_propagation_across_workers_and_nodes_over_secure_channel() {
    let (received, spans) = trace_code(|ctx| send_echo_message_over_secure_channel(ctx, "hello"));
    assert_eq!(received.unwrap(), "hello".to_string());

    // There are many traces:
    //  - The main "root" trace is for creating the secure channel and sending a message
    //  - The other traces are mostly the reception of a transport message (which starts a new trace)
    //    and the processing of that message until a response is sent back
    //    (if any, sometimes the received message is just a shutdown signal)
    let actual = display_traces(spans);
    let expected = r#"
TcpTransportMessage::start_trace
└── TcpRecvProcessor::process
    └── DecryptorWorker::handle_message
        └── handle_decrypt
            └── decrypt
                ├── mark
                ├── get_key
                ├── aead_decrypt
                └── update_key

TcpTransportMessage::start_trace
└── TcpRecvProcessor::process
    └── DecryptorWorker::handle_message
        └── handle_decrypt
            └── decrypt
                ├── mark
                ├── get_key
                ├── aead_decrypt
                └── update_key

TcpTransportMessage::start_trace
└── TcpRecvProcessor::process
    └── DecryptorWorker::handle_message
        └── handle_decrypt
            ├── decrypt
            |   ├── mark
            |   ├── get_key
            |   ├── aead_decrypt
            |   └── update_key
            └── Echoer::handle_message
                └── EncryptorWorker::handle_message
                    └── handle_encrypt
                        ├── encrypt
                        |   └── aead_encrypt
                        └── TcpSendWorker::handle_message
                            └── TcpTransportMessage::end_trace

TcpTransportMessage::start_trace
└── TcpRecvProcessor::process
    └── HandshakeWorker::handle_message
        ├── aead_decrypt
        ├── delete_aead_secret_key
        ├── aead_decrypt
        └── delete_aead_secret_key

TcpTransportMessage::start_trace
└── TcpRecvProcessor::process
    └── HandshakeWorker::handle_message
        ├── aead_decrypt
        ├── delete_aead_secret_key
        ├── aead_decrypt
        ├── aead_encrypt
        ├── delete_aead_secret_key
        ├── aead_encrypt
        ├── delete_aead_secret_key
        └── TcpSendWorker::handle_message
            └── TcpTransportMessage::end_trace

TcpTransportMessage::start_trace
└── TcpRecvProcessor::process
    └── HandshakeWorker::handle_message
        ├── aead_encrypt
        ├── delete_aead_secret_key
        ├── aead_encrypt
        └── TcpSendWorker::handle_message
            └── TcpTransportMessage::end_trace

root
└── send_echo_message_over_secure_channel
    ├── create tcp transport
    ├── TcpListenProcessor::start
    ├── create tcp transport
    ├── connect
    ├── TcpSendWorker::start
    ├── TcpRecvProcessor::start
    ├── TcpSendWorker::handle_message
    |   └── TcpTransportMessage::end_trace
    └── MessageSender::handle_message
        └── EncryptorWorker::handle_message
            └── handle_encrypt
                ├── encrypt
                |   └── aead_encrypt
                └── TcpSendWorker::handle_message
                    └── TcpTransportMessage::end_trace
"#;
    pretty_assertions::assert_eq!(format!("\n{actual}"), expected);
}

/// HELPERS

/// Start 2 nodes:
///
///  - 1 node with a MessageSender worker
///  - 1 node with an Echoer worker
///
/// and send an "hello" message from the MessageSender to the Echoer using a TCP connection
#[instrument(skip_all, fields(message = message))]
async fn send_echo_message(ctx: Context, message: &str) -> ockam_core::Result<String> {
    // Create a node with an Echoer service, listening on a TCP port
    let node1 = node(ctx.async_try_clone().await?).await?;
    let tcp1 = node1.create_tcp_transport().await?;
    node1.start_worker("echoer", Echoer).await?;
    let listener = tcp1
        .listen("127.0.0.1:4000", TcpListenerOptions::new())
        .await?;
    node1
        .flow_controls()
        .add_consumer("echoer", listener.flow_control_id());

    // Create a second node which will send messages to the first node
    let node2 = node(ctx.async_try_clone().await?).await?;
    let tcp2 = node2.create_tcp_transport().await?;
    let tcp_sender = tcp2
        .resolve_address(Address::new_with_string(TCP, "127.0.0.1:4000"))
        .await?;

    let message_sender = MessageSender {
        sender_to: tcp_sender,
    };

    node1.start_worker("message_sender", message_sender).await?;
    let result = node2
        .context()
        .send_and_receive::<String>(route!["message_sender".to_string()], message.to_string())
        .await?;
    ctx.stop().await?;
    Ok(result)
}

/// Start 2 nodes:
///
///  - 1 node with a MessageSender worker
///  - 1 node with an Echoer worker
///
/// and send an "hello" message from the MessageSender to the Echoer using
/// secure channel over a TCP connection
#[instrument(skip_all, fields(message = message))]
async fn send_echo_message_over_secure_channel(
    ctx: Context,
    message: &str,
) -> ockam_core::Result<String> {
    // Create a node with an Echoer service, listening on a TCP port
    let node1 = node(ctx.async_try_clone().await?).await?;
    let identity1 = node1.create_identity().await?;
    let tcp1 = node1.create_tcp_transport().await?;
    node1.start_worker("echoer", Echoer).await?;
    let listener = tcp1
        .listen("127.0.0.1:4000", TcpListenerOptions::new())
        .await?;
    let secure_channel_listener = node1
        .create_secure_channel_listener(
            &identity1,
            "secure_channel_listener",
            SecureChannelListenerOptions::new().as_consumer(listener.flow_control_id()),
        )
        .await?;
    node1
        .flow_controls()
        .add_consumer("echoer", secure_channel_listener.flow_control_id());

    // Create a second node which will send messages to the first node
    let node2 = node(ctx.async_try_clone().await?).await?;
    let identity2 = node2.create_identity().await?;
    let tcp2 = node2.create_tcp_transport().await?;
    let tcp_sender = tcp2
        .resolve_address(Address::new_with_string(TCP, "127.0.0.1:4000"))
        .await?;
    let channel = node2
        .create_secure_channel(
            &identity2,
            route![tcp_sender, secure_channel_listener.address().clone()],
            SecureChannelOptions::new(),
        )
        .await?;

    let message_sender = MessageSender {
        sender_to: channel.encryptor_address().clone(),
    };

    node1.start_worker("message_sender", message_sender).await?;
    let result = node2
        .context()
        .send_and_receive::<String>(route!["message_sender".to_string()], message.to_string())
        .await?;

    ctx.stop_worker(channel.encryptor_address().clone()).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(result)
}

/// MessageSender worker.
/// It uses either a TcpSender address or an Encryptor worker address to send a message to an echoer
struct MessageSender {
    sender_to: Address,
}

#[async_trait]
impl Worker for MessageSender {
    type Message = String;
    type Context = Context;

    #[instrument(skip_all, name = "MessageSender::handle_message")]
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<String>,
    ) -> ockam_core::Result<()> {
        let return_route = msg.return_route();
        let received = ctx
            .send_and_receive::<String>(route![self.sender_to.clone(), "echoer"], msg.into_body()?)
            .await?;
        ctx.send(return_route, received).await
    }
}
