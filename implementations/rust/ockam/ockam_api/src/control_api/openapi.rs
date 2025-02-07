use super::backend::inlet::*;
use super::backend::outlet::*;
use super::backend::relay::*;
use super::backend::ticket::*;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

struct Authentications;

impl Modify for Authentications {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(schema) = openapi.components.as_mut() {
            schema.add_security_scheme(
                "bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("Plaintext")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Ockam Control API",
        version = "0.1.0",
        description = "API to control Ockam nodes",
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
    ),
    security(
        ("bearer" = [])
    ),
    external_docs(
        url = "https://docs.ockam.io/",
        description = "Ockam documentation"
    )
)]
struct ApiDoc;

pub fn generate_schema() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
