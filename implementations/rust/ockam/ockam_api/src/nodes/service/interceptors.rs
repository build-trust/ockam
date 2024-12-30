use crate::http::interceptor::HttpHeadersInterceptorRequest;
use crate::nodes::models::services::StartServiceRequest;
use crate::nodes::BackgroundNodeClient;
use crate::DefaultAddress;
use ockam_core::api::Request;
use ockam_core::Address;
use ockam_node::Context;

impl BackgroundNodeClient {
    pub async fn create_http_header_overwrite_service(
        &self,
        context: &Context,
        address: &Address,
        headers: Vec<(String, String)>,
    ) -> miette::Result<()> {
        let body = StartServiceRequest::new(
            HttpHeadersInterceptorRequest {
                headers: headers.clone(),
            },
            address.clone(),
        );
        self.ask(
            context,
            Request::post(format!(
                "/node/services/{}",
                DefaultAddress::HTTP_HEADERS_SERVICE
            ))
            .body(body),
        )
        .await
    }
}
