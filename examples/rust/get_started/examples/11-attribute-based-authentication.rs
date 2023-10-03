/// INTRODUCTION
///
/// This example shows how to use attribute-based credential in order to have
/// several devices connecting to a server, via the Ockam Orchestrator.
///
/// The corresponding example using the command-line can be found here: https://docs.ockam.io/use-cases/apply-fine-grained-permissions-with-attribute-based-access-control-abac.
///
/// You first need to:
///
///  - create a project with `ockam enroll`
///  - export the project information with `ockam project information > project.json`
///
/// Then you can start:
///
///  - a local Python webserver: `python3 -m http.server --bind 127.0.0.1 5000`
///  - the control node in `11-attribute-based-authentication-control-plane.rs`
///  - the edge node in `11-attribute-based-authentication-edge-plane.rs`
///
/// This will set up a TCP outlet on the control node, connected to the Python webserver
/// and a TCP inlet on the edge node which can be used to send HTTP requests (at 127.0.0.1:7000).
///
/// Then if you execute `curl --fail --head --max-time 10 127.0.0.1:7000` you should get back
/// a successful response like:
///
/// HTTP/1.0 200 OK
/// Server: SimpleHTTP/0.6 Python/3.9.6
/// Date: Tue, 07 Feb 2023 15:05:59 GMT
/// Content-type: text/html; charset=utf-8
/// Content-Length: 870
///
/// and observe that a successful connection has been made on the Python webserver:
///
/// ± python3 -m http.server --bind 127.0.0.1 5000                                                                                                                                                                                                                                                                                                                                                                                      default
///  Serving HTTP on 127.0.0.1 port 5000 (http://127.0.0.1:5000/) ...
///  127.0.0.1 - - [06/Feb/2023 15:52:20] "HEAD / HTTP/1.1" 200 -
///
/// TOPOLOGY
///
/// The network we establish between the control node, the edge node and the Orchestrator is the following
///
///       get credential         +-------------------------------------+
///       via secure channel     |                                     | Inlet <-- 127.0.0.1:7000
///              +---------------+              Edge node              | connected to "outlet"
///              |               |                                     | via secure channel
///              |               +-------------------------------------+
///              |                   |                           |
///              |                   |                           | create secure channel to control
///              |                   |                           | via the relay
///              v                   v                           |
///      +--------------+        +-------------------------------+-------+
///      | Authority    |        |                               |       |
///      |              |        |             Orchestrator      |       |
///      |              |        |                               |       |
///      +--------------+        +---------------------- forwarder ------+
///              ^                  ^                    to control
///              |                  |                   ^        |
///              |                  |            create |        |
///              |                  |                   |        v
///              |                  |                   |     "untrusted"  secure channel
///              |               +---------------------------------------+ listener
///              |               |                                       |
///              +---------------|              Control node             | "outlet" --> 127.0.0.1:5000
///       get credential         |                                       |
///       via secure channel     +---------------------------------------+
///
///
///   - we create initially some secure channels to the Authority in order to retrieve credential
///     based on a one-time token generated with `ockam project ticket --attribute component=<name of node>`
///
///   - then the control node creates a relay on the Orchestrator in order to accept TCP traffic without
///     having to open a port to the internet. It also starts a channel listener ("untrusted", accept all incoming requests for now)
///
///   - on its side the edge node starts a secure channel via relay (named "forward_to_control_plane1"), to the "untrusted" listener
///     with the secure channel address it creates an Inlet which will direct TCP traffic via the secure channel to get to the
///     control node and then to the "outlet" worker to reach the Python webserver
///
///   - the outlet is configured to only receive messages from the edge node by checking its authenticated attributes
///   - the inlet is configured to only receive messages from the control node by checking its authenticated attributes
///
/// IMPLEMENTATION
///
/// The code for this example can be found in:
///
///  - examples/11-attribute-based-authentication-control-plane.rs: for the control node
///  - examples/11-attribute-based-authentication-edge-plane.rs: for the edge node
///  - src/project.rs: read the content of the project.json file
///  - src/token.rs: generate a one-time token using the ockam command line
///

/// unused main function
fn main() {}
