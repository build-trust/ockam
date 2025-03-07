use super::backend::authority_member::*;
use super::backend::inlet::*;
use super::backend::outlet::*;
use super::backend::relay::*;
use super::backend::ticket::*;
use crate::control_api::protocol::common::NodeName;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

struct Authentications;

impl Modify for Authentications {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(schema) = openapi.components.as_mut() {
            schema.add_security_scheme(
                "token",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .description(Some("A simple token authentication, this token can be set as startup parameter"))
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("Plaintext")
                        .build(),
                ),
            );
        }
    }
}

const MAIN_DESCRIPTION: &str = r#"
## Overview
This API is designed to control Ockam nodes via HTTP requests without having to use Ockam command or Ockam library.
You can expect a similar abstraction level as Ockam command, but with a more programmatic approach.

The endpoint will receive the HTTP request and forward it to the selected node using the `node`
parameter. How the node will be selected starting from the name is configuration dependent.

## Versioning
Current major `0.Y.Z` is considered unstable and may have breaking changes.
Future versions will follow semantic versioning, breaking changes will be reflected in the major version number.

## Authentication
Only a simple bearer token is required to authenticate. The token is provided by the user in the
configuration or via environment variable, it's then passed in the `Authorization` header:
 `Authorization: Bearer my-secret-token`.

## Getting Started
The easiest way to get development started is to use this API with the Ockam command and run a single node acting both as a frontend and backend.
```sh
ockam node create --foreground -vv --configuration '{
  "services": {
    "control-api": {
      "authentication-token": "my-secret-token",
      "backend": true,
      "frontend": true,
      "http-bind-address": "127.0.0.1:8080"
    }
  }
}'
```

When an undocumented error is returned, such as internal server error, a message contained by `ErrorResponse` is usually present for debugging purposes.
It's highly recommended to check the logs of the Ockam nodes for more information.
"#;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Ockam Node Control API",
        version = "0.1.0",
        description = MAIN_DESCRIPTION,
    ),
    tags((
        name = "Portals",
        external_docs(
            url = "https://docs.ockam.io/reference/command/advanced-routing#portal",
            description = "Learn more about Portals on Ockam command documentation"
        ),
        description =
"
A TCP Inlet and TCP Outlet together form a Portal, working hand in hand with Relays. A TCP Inlet
defines where a node, running on another machine, listens for connections. The Inlet's route
provides information on how to forward traffic to the Outlet (its address). Relays allow you to
establish end-to-end protocols with services that operate in remote private networks.
"
        ),(
        name = "Relays",
        external_docs(
            url = "https://docs.ockam.io/reference/command/advanced-routing#relays",
            description = "Learn more about Relays on Ockam command documentation"
        ),
        description =
"
Relays make it possible to establish end-to-end protocols with services operating in a remote
private network, without requiring a remote service to expose listening ports to an outside hostile
network like the Internet.
",
        ),(
        name = "Tickets",
        description =
"
The ticket is plain text representing a one-time use token and the non-sensitive data about the
Project, like the route to reach it and the Project Identity Identifier, which will be used to
validate the Project Identity. The ticket itself can be stored in an environment variable, or a
file.
",
        ),(
            name = "Authority Members",
            description =
"
An Ockam Authority is a Ockam node running a set of services dedicated to managing credentials
and identities.
The Ockam Authority keeps a list of members, which are entitled to receive credentials with proper
attributes.
"
        )
    ),
    modifiers(&Authentications),
    paths(
        handle_tcp_inlet_create,
        handle_tcp_inlet_update,
        handle_tcp_inlet_list,
        handle_tcp_inlet_delete,
        handle_tcp_inlet_get,
        handle_tcp_outlet_create,
        handle_tcp_outlet_update,
        handle_tcp_outlet_list,
        handle_tcp_outlet_delete,
        handle_tcp_outlet_get,
        handle_relay_create,
        handle_relay_list,
        handle_relay_get,
        handle_relay_delete,
        handle_ticket_create,
        handle_ticket_enroll,
        handle_authority_member_add_or_update,
        handle_authority_member_list,
        handle_authority_member_get,
        handle_authority_member_remove
    ),
    security(
        ("token" = [])
    ),
    external_docs(
        url = "https://docs.ockam.io/",
        description = "Ockam documentation"
    ),
    components(
        schemas(NodeName),
    )
)]
struct ApiDoc;

pub fn generate_schema() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
