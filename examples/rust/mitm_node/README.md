# TCP MitM Attack
### This attack is based on the existence of an active adversary, who is able to intercept our TCP traffic as well as modify it and insert any packets
1. Build ockam binary from the latest develop using
    ```
    cargo build --features debugger
    ```
1. Put it in one of the `$PATH` locations
1. `ockam enroll`
1. Add an entry to `/etc/hosts` to redirect traffic from Ockam Orchestrator to the mitm_node, in my case it's
    ```
    127.0.0.1       k8s-hub-nginxing-2df5f8bd32-50456abbf63cd63c.elb.us-west-1.amazonaws.com
    ```
1. Clone ockam repo again to a separate directory using the branch you are currently reading these instructions from
1. Build and run `examples/rust/mitm_node/src/bin/tcp_mitm.rs`
    ```
    cargo run --package mitm_node --bin tcp_mitm
    ```
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
1. `Address`es to check:
   - Static:
    ```
    echo
    hop
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

     11 Addresses total, so you should see 11 messages in the log saying that outgoing access control did not pass

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
    SecureChannel.initiator.decryptor.remote_36
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

    Also `TcpSendWorker_tx_addr_responder_62` and `TcpRecvProcessor_responder_64` were not found since it's the TCP connection used to handle `ockam worker list` command and it's termninated immediately after you see the result in the command line.

1. Don't forget to clean your `/etc/hosts`

# Malicious node attack

### This attack is based on the existence of a malicious node that possesses a valid credential (i.e. is a member node) and tries to achieve privilege escalation when connected to the other member node

1. Build ockam binary from the latest develop using
    ```
    cargo build --features debugger
    ```
1. Put it in one of the `$PATH` locations
1. Run following commands:
   ```
   ockam enroll
   ockam project information --output json > project.json
   ockam node create defender --project ./project.json
   ockam relay create defender --at /project/default --to /node/defender
   ockam node create attacker --project ./project.json
   ockam secure-channel create --from /node/attacker --to "/project/default/service/forward_to_defender/service/api"
   ```
1. At this point we have an active secure channel to the defender node and valid credentials exchange, we now can try to reach different `Address`es on the defender node by running:
   ```
   ockam message send --from attacker --to /service/SecureChannel.initiator.encryptor_{NUMBER}/service/{ADDRESS_TO_REACH} {MESSAGE}
   ```
   For example this `Address` should be reachable and respond with the same message:
   ```
   ockam message send --from attacker --to /service/SecureChannel.initiator.encryptor_66/service/echo HELLO
   ```
1. Start inspecting the node's log file - `~/.ockam/nodes/defender/stdout.log`
1. Start sending messages to `Address`es that should not be reachable, in this case log should have the following entries:
    ```
    Message forwarded from {RANDOM_ADDRESS} to {ENTERED_ADDRESS} did not pass outgoing access control
    ```
1. `Address`es to check:
   - Static:
    ```
    hop
    identity_service
    authenticated
    _internal.nodemanager
    app
    forwarding_service
    ockam.ping.collector
    rpc_proxy_service
    ```

   - Dynamic Addresses can be found by running `ockam worker list --at defender`, in my case those were:
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
    SecureChannel.initiator.decryptor.remote_36
    SecureChannel.initiator.decryptor.remote_47
    SecureChannel.initiator.decryptor.remote_75
    SecureChannel.initiator.encryptor_39
    SecureChannel.initiator.encryptor_50
    SecureChannel.initiator.encryptor_78
    SecureChannel.responder.decryptor.remote_64
    SecureChannel.responder.encryptor_67
    TcpListenProcessor_3
    TcpRecvProcessor_initiator_33
    TcpRecvProcessor_responder_88
    TcpSendWorker_tx_addr_initiator_31
    TcpSendWorker_tx_addr_initiator_42
    TcpSendWorker_tx_addr_initiator_70
    TcpSendWorker_tx_addr_responder_86
    ```
    Also `TcpSendWorker_tx_addr_responder_86` and `TcpRecvProcessor_responder_88` were not found since it's the TCP connection used to handle `ockam worker list` command and it's termninated immediately after you see the result in the command line.

   - Reachable `Address`es:
      ```
      api
      credentials
      echo
      uppercase
      ```
