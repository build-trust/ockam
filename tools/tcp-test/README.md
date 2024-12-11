This tool is useful to test the portal capabilities and operates as a **generic** TCP client and server.
It also implements (unsafe) TLS, which is useful for performance comparison.

# Deployment

Can be easily deployed via ssh using the `deploy` script:
```bash
$ ./deploy user@host
```

# Actions

# latency
Measures the latency of a TCP packets to a given host and port.
It must be used with an echo server.

```bash
$ tcp-test echo 1234
Listening on 0.0.0.0:1234

$ tcp-test latency 127.0.0.1:1234
Connection + First RTT: 0ms 440us, Second RTT: 0ms 143us
Connection + First RTT: 0ms 192us, Second RTT: 0ms 160us
Connection + First RTT: 0ms 276us, Second RTT: 0ms 162us
Connection + First RTT: 0ms 326us, Second RTT: 0ms 155us
Connection + First RTT: 0ms 227us, Second RTT: 0ms 172us
Connection + First RTT: 0ms 250us, Second RTT: 0ms 111us
Connection + First RTT: 0ms 461us, Second RTT: 0ms 122us
Connection + First RTT: 0ms 326us, Second RTT: 0ms 116us
Connection + First RTT: 0ms 320us, Second RTT: 0ms 104us
Connection + First RTT: 0ms 270us, Second RTT: 0ms 101us
Average - Connection + First RTT: 0ms 309us, Second RTT: 0ms 135us
```

# flood
Floods a given host and port with connections until it fails or reaches the provided maximum.
It's meant to measure the maximum number of connections a portal can handle.
It must be used with an echo server.

Example:
```bash
$ tcp-test echo 1234
Listening on 0.0.0.0:1234

$ tcp-test flood 127.0.0.1:1234
Failed to connect: Os { code: 49, kind: AddrNotAvailable, message: "Can't assign requested address" }
Flooded 16344 connections
Pinging each...
All connections succeeded
```

# throughput
Measures the throughput of a TCP connection to a given host and port.
It can be used to measure the throughput of a portal, especially to compare a portal against TLS.
Can be used in conjunction with a null server for results similar to `iperf3`, but echo can also be
used for a full bandwidth test.

```bash
$ tcp-test null 1234
Listening on 0.0.0.0:1234

$ tcp-test throughput 127.0.0.1:1234
Outgoing Throughput: 70.44 Gbps
Outgoing Throughput: 75.12 Gbps
Outgoing Throughput: 75.44 Gbps
Outgoing Throughput: 78.43 Gbps
Outgoing Throughput: 71.89 Gbps
Outgoing Throughput: 69.80 Gbps
Outgoing Throughput: 75.13 Gbps
^C
```

## echo
It starts a TCP server that listens to on a given port and **echoes back** any data it receives.
It's meant to be used in conjunction with the other actions.

```bash
$ tcp-test echo 1234
Listening on 0.0.0.0:1234
```

# null
It starts a TCP server that listens to on a given port and **discards** any data it receives.
It's meant to be used in conjunction with the other actions.

```bash
$ tcp-test null 1234
Listening on 0.0.0.0:1234
```
