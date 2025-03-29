use std::io::Write;

use ockam_core::async_trait;
use ockam_node::Context;
use ockam_transport_tcp::{Direction, PortalInterceptor, PortalInterceptorFactory};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::token_lease_refresher::TokenLeaseRefresher;
use crate::http::state::{ClientRequestWriter, RequestState};
use tracing::{debug, error};

struct HttpAuthInterceptorState {
    state: RequestState,
}

struct HttpAuthInterceptor {
    state: Arc<Mutex<HttpAuthInterceptorState>>,
    token_refresher: TokenLeaseRefresher,
}

impl HttpAuthInterceptor {
    fn new(token_refresher: TokenLeaseRefresher) -> Self {
        let state = HttpAuthInterceptorState {
            state: RequestState::ParsingHeader(None),
        };
        Self {
            state: Arc::new(Mutex::new(state)),
            token_refresher,
        }
    }
}

pub struct HttpAuthInterceptorFactory {
    token_refresher: TokenLeaseRefresher,
}

impl HttpAuthInterceptorFactory {
    pub fn new(token_refresher: TokenLeaseRefresher) -> Self {
        Self { token_refresher }
    }
}

impl PortalInterceptorFactory for HttpAuthInterceptorFactory {
    fn create(&self) -> Arc<dyn PortalInterceptor> {
        Arc::new(HttpAuthInterceptor::new(self.token_refresher.clone()))
    }
}

fn attach_auth_token_and_serialize_into(
    req: &httparse::Request,
    token: &str,
    buffer: &mut Vec<u8>,
) {
    debug!("Serializing http req header");
    write!(
        buffer,
        "{} {} HTTP/1.{}\r\n",
        req.method.unwrap(),
        req.path.unwrap(),
        req.version.unwrap()
    )
    .unwrap();

    write!(buffer, "Authorization: Token {}\r\n", token).unwrap();
    for h in &*req.headers {
        if !h.name.eq_ignore_ascii_case("Authorization") {
            write!(buffer, "{}: ", h.name).unwrap();
            buffer.extend_from_slice(h.value);
            buffer.extend_from_slice(b"\r\n");
        }
    }
    buffer.extend_from_slice(b"\r\n");
}

#[async_trait]
impl PortalInterceptor for HttpAuthInterceptor {
    async fn intercept(
        &self,
        _context: &mut Context,
        direction: Direction,
        buffer: &[u8],
    ) -> ockam_core::Result<Option<Vec<u8>>> {
        match direction {
            Direction::FromOutletToInlet => Ok(Some(buffer.to_vec())),
            Direction::FromInletToOutlet => {
                let mut guard = self.state.lock().await;
                let out = guard.state.process_http_buffer(buffer, self)?;
                Ok(Some(out))
            }
        }
    }
}

impl ClientRequestWriter for &HttpAuthInterceptor {
    fn write_headers(
        &self,
        request: &httparse::Request,
        buffer: &mut Vec<u8>,
    ) -> ockam_core::Result<()> {
        let token = self.token_refresher.get_token();
        if token.is_none() {
            error!("No authorization token available");
        }

        attach_auth_token_and_serialize_into(request, &token.unwrap_or_default(), buffer);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::http::state::{ClientRequestWriter, RequestState};
    use crate::influxdb::gateway::interceptor::attach_auth_token_and_serialize_into;

    const REQ: &str = "POST / HTTP/1.1\r\n\
Host: www.example.com\r\n\
User-Agent: Mozilla/5.0\r\n\
Accept-Encoding: gzip, deflate, br\r\n\
Transfer-Encoding: gzip, chunked\r\n\r\n\
4\r\nWiki\r\n7\r\npedia i\r\n0\r\n\r\n";

    const TOKEN: &str = "SAMPLE-TOKEN";

    const EXPECTED: &str = "POST / HTTP/1.1\r\n\
Authorization: Token SAMPLE-TOKEN\r\n\
Host: www.example.com\r\n\
User-Agent: Mozilla/5.0\r\n\
Accept-Encoding: gzip, deflate, br\r\n\
Transfer-Encoding: gzip, chunked\r\n\r\n\
4\r\nWiki\r\n7\r\npedia i\r\n0\r\n\r\n";

    struct RequestWriterSimulator;

    impl ClientRequestWriter for RequestWriterSimulator {
        fn write_headers(
            &self,
            request: &httparse::Request,
            buffer: &mut Vec<u8>,
        ) -> ockam_core::Result<()> {
            attach_auth_token_and_serialize_into(request, TOKEN, buffer);
            Ok(())
        }
    }

    #[test]
    fn parse_post_with_chunked_transfers() {
        let mut data = Vec::new();
        data.extend_from_slice(REQ.as_bytes());
        data.extend_from_slice(REQ.as_bytes());

        for size in [1, 5, 32, 1024] {
            let mut result = Vec::new();
            let mut request_state = RequestState::ParsingHeader(None);
            for chunk in data.chunks(size) {
                let data_out = request_state
                    .process_http_buffer(chunk, RequestWriterSimulator)
                    .unwrap();
                result.extend_from_slice(&data_out);
            }
            assert_eq!(
                String::from_utf8(result).unwrap(),
                EXPECTED.to_owned() + EXPECTED
            );
            assert_eq!(request_state, RequestState::ParsingHeader(None));
        }
    }

    #[test]
    fn parse_post_with_content_length() {
        let req = "POST /test HTTP/1.1\r\n\
Host: foo.example\r\n\
Content-Type: application/x-www-form-urlencoded\r\n\
Content-Length: 27\r\n\r\n\
field1=value1&field2=value2";
        let expected_r = format!(
            "POST /test HTTP/1.1\r\n\
Authorization: Token {}\r\n\
Host: foo.example\r\n\
Content-Type: application/x-www-form-urlencoded\r\n\
Content-Length: 27\r\n\r\n\
field1=value1&field2=value2",
            TOKEN
        );

        let data = [req.as_bytes(), req.as_bytes()].concat();
        let expected = [expected_r.as_bytes(), expected_r.as_bytes()].concat();

        for size in [1, 5, 32, 1024] {
            let mut result = Vec::new();
            let mut request_state = RequestState::ParsingHeader(None);
            for chunk in data.chunks(size) {
                let data_out = request_state
                    .process_http_buffer(chunk, RequestWriterSimulator)
                    .unwrap();
                result.extend_from_slice(&data_out);
            }
            assert_eq!(
                String::from_utf8(result).unwrap(),
                String::from_utf8(expected.clone()).unwrap()
            );
            assert_eq!(request_state, RequestState::ParsingHeader(None));
        }
    }

    #[test]
    fn parse_get_requests() {
        let req = "GET /home/user/example.txt HTTP/1.1\r\n\r\n";
        let mut data = Vec::new();
        data.extend_from_slice(req.as_bytes());
        data.extend_from_slice(req.as_bytes());

        let mut expected = format!(
            "GET /home/user/example.txt HTTP/1.1\r\nAuthorization: Token {}\r\n\r\n",
            TOKEN
        );
        expected = expected.clone() + &expected;

        for size in [1, 5, 32, 1024] {
            let mut result = Vec::new();
            let mut request_state = RequestState::ParsingHeader(None);
            for chunk in data.chunks(size) {
                let data_out = request_state
                    .process_http_buffer(chunk, RequestWriterSimulator)
                    .unwrap();
                result.extend_from_slice(&data_out);
            }
            assert_eq!(String::from_utf8(result).unwrap(), expected);
            assert_eq!(request_state, RequestState::ParsingHeader(None));
        }
    }
}
