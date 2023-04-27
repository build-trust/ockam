1. Build ockam binary from the latest develop using
    ```
    cargo build --features debugger
    ```
1. Put it in one of PATH locations
1. `ockam enroll`
1. Add an entry to /etc/hosts to redirect traffic from Ockam Orchestrator to the mitm_node, in my case it's
    ```
    127.0.0.1       k8s-hub-nginxing-2df5f8bd32-50456abbf63cd63c.elb.us-west-1.amazonaws.com
    ```
1. Clone ockam repo again to a separate directory using the branch you are currently reading these instructions from
1. Build and run `examples/rust/mitm_node/src/bin/tcp_mitm.rs`
1. Create a node
    ```
    ockam node create green
    ```
1. Start inspecting the node's log file - `~/.ockam/nodes/green/stdout.log`
1. Run some command by that node to initiate a connection to the project node, e.g.
    ```
    ockam relay create green --at /project/default --to /node/green
    ```
    That should succeed
1. Enter Addresses that should not pass FlowControls to the command line of the `mith-node`
1. Check node's log for entries consisting
    ```
    Message forwarded from {RANDOM_ADDRESS} to {ENTERED_ADDRESS} did not pass outgoing access control
    ```
1. Addresses to check:
   - Static:
    ```
    echo
    hop
    vault_service
    identity_service
    uppercase
    authenticated
    credentials
    _internal.nodemanager
    app
    forwarding_service
    ockam.ping.collector
    rpc_proxy_service
    ```

     12 Addresses total, so you should see 12 messages in the log saying that outgoing access control did not pass

 - Dynamic Addresses can be found by running `ockam worker list`, in my case those were:
    ```
    Context.async_try_clone.detached_1
    Context.async_try_clone.detached_2
    Context.async_try_clone.detached_4
    Context.async_try_clone.detached_5
    Context.async_try_clone.detached_61
    DelayedEvent.create_60
    Detached.embedded_node.not_stopped_0
    Medic.ctx_7
    RemoteForwarder.static.main_internal_57
    SecureChannel.initiator.decryptor.remote_36??
    SecureChannel.initiator.decryptor.remote_47
    SecureChannel.initiator.encryptor_39
    SecureChannel.initiator.encryptor_50
    TcpListenProcessor_3
    TcpRecvProcessor_initiator_33
    TcpRecvProcessor_initiator_44
    TcpRecvProcessor_responder_64
    TcpSendWorker_tx_addr_initiator_31
    TcpSendWorker_tx_addr_initiator_42
    TcpSendWorker_tx_addr_responder_62
    ```

In my case `SecureChannel.initiator.decryptor.remote_36` Address was reachable, since this is the Secure Channel using the TCP connection we intercepted, but messages to this worker are checked cryptographically.

Also `TcpSendWorker_tx_addr_responder_62` was not found since it's the TCP connection used to handle `ockam worker list` command and it's termninated immediately after you see the result in the command line.