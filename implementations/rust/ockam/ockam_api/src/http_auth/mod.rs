use std::io::Write;

use httparse::{Header, Status};
use ockam_core::async_trait;
use ockam_node::Context;
use ockam_transport_tcp::{Direction, PortalInterceptor, PortalInterceptorFactory};
use std::sync::Arc;
use tokio::sync::Mutex;

use ockam::errcode::{Kind, Origin};

use tracing::{debug, error};

use crate::TokenLeaseRefresher;

#[derive(Debug, Clone, PartialEq)]
enum RequestState {
    ParsingHeader(Option<Vec<u8>>),
    ParsingChunkedHeader(Option<Vec<u8>>),
    RemainingInChunk(usize),
    RemainingBody(usize),
}

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

fn body_state(method: &str, headers: &[Header]) -> ockam_core::Result<RequestState> {
    match method.to_uppercase().as_str() {
        "POST" | "PUT" => {
            for h in headers {
                if h.name.eq_ignore_ascii_case("Content-Length") {
                    if let Ok(str) = std::str::from_utf8(h.value) {
                        return str.parse().map(RequestState::RemainingBody).map_err(|e| {
                            ockam_core::Error::new(Origin::Transport, Kind::Invalid, e)
                        });
                    }
                } else if h.name.eq_ignore_ascii_case("Transfer-Encoding")
                    && String::from_utf8(h.value.to_vec()).is_ok_and(|s| s.contains("chunked"))
                {
                    return Ok(RequestState::ParsingChunkedHeader(None));
                }
            }
            // Not content-length, no chunked encoding, fail.
            Err(ockam_core::Error::new(
                Origin::Transport,
                Kind::Invalid,
                "No Content-Length nor chunked Transfer-Encoding",
            ))
        }
        _ => Ok(RequestState::ParsingHeader(None)),
    }
}

impl RequestState {
    /* Parse the incoming data,  attaching an Authorization header token to it.
     * data is received in chunks, and there is no warranty on what we get on each:
     * incomplete requests,  multiple requests, etc.
     */
    fn process_http_buffer(&mut self, buf: &[u8], token: &str) -> ockam_core::Result<Vec<u8>> {
        let mut acc = Vec::with_capacity(buf.len());
        let mut cursor = buf;
        loop {
            if cursor.is_empty() {
                return Ok(acc);
            }
            match self {
                RequestState::ParsingHeader(prev) => {
                    let (to_parse, prev_size): (&[u8], usize) = if let Some(b) = prev {
                        let prev_size = b.len();
                        b.extend_from_slice(cursor);
                        (b, prev_size)
                    } else {
                        (cursor, 0usize)
                    };
                    let mut headers = [httparse::EMPTY_HEADER; 64];
                    let mut req = httparse::Request::new(&mut headers);
                    match req.parse(to_parse) {
                        Ok(httparse::Status::Partial) if prev_size == 0 => {
                            // No previous buffered, need to copy and own the unparsed data
                            *self = RequestState::ParsingHeader(Some(cursor.to_vec()));
                            return Ok(acc);
                        }
                        Ok(httparse::Status::Partial) => {
                            // There was a previous buffer, and we already added the newly data to it
                            return Ok(acc);
                        }
                        Ok(httparse::Status::Complete(body_offset)) => {
                            cursor = &cursor[body_offset - prev_size..];
                            attach_auth_token_and_serialize_into(&req, token, &mut acc);
                            *self = body_state(req.method.unwrap(), req.headers)?;
                        }
                        Err(e) => {
                            error!("Error parsing header: {:?}", e);
                            return Err(ockam_core::Error::new(
                                Origin::Transport,
                                Kind::Invalid,
                                e,
                            ));
                        }
                    }
                }
                RequestState::RemainingBody(remaining) => {
                    if *remaining <= cursor.len() {
                        acc.extend_from_slice(&cursor[..*remaining]);
                        cursor = &cursor[*remaining..];
                        *self = RequestState::ParsingHeader(None);
                    } else {
                        acc.extend_from_slice(cursor);
                        *remaining -= cursor.len();
                        return Ok(acc);
                    }
                }
                RequestState::ParsingChunkedHeader(prev) => {
                    let (to_parse, prev_size): (&[u8], usize) = if let Some(b) = prev {
                        let prev_size = b.len();
                        b.extend_from_slice(cursor);
                        (b, prev_size)
                    } else {
                        (cursor, 0usize)
                    };
                    match httparse::parse_chunk_size(to_parse) {
                        Ok(Status::Complete((2, 0))) => {
                            // this is just a final \r\n.  The spec said it should end in a 0-sized
                            // chunk.. but having seen this on the wild as well.
                            acc.extend_from_slice(&to_parse[..2]);
                            cursor = &cursor[2 - prev_size..];
                            *self = RequestState::ParsingHeader(None);
                        }
                        Ok(Status::Complete((3, 0))) => {
                            // this is just a proper 0\r\n final chunk.
                            acc.extend_from_slice(&to_parse[..3]);
                            cursor = &cursor[3 - prev_size..];
                            // There must be a final \r\n.  And no more chunks,
                            // so just reuse the RemainingBody state for this
                            *self = RequestState::RemainingBody(2);
                        }
                        Ok(Status::Complete((pos, chunk_size))) => {
                            acc.extend_from_slice(&to_parse[..pos]);
                            cursor = &cursor[pos - prev_size..];
                            let complete_size = chunk_size + 2; //chunks ends in \r\n
                            *self =
                                RequestState::RemainingInChunk(complete_size.try_into().unwrap());
                        }
                        Ok(Status::Partial) if prev_size == 0 => {
                            // No previous buffered, need to copy and own the unparsed data
                            *self = RequestState::ParsingChunkedHeader(Some(cursor.to_vec()));
                            return Ok(acc);
                        }
                        Ok(Status::Partial) => {
                            // There was a previous buffer, and we already added the newly data to it
                            return Ok(acc);
                        }
                        Err(e) => {
                            error!("Error parsing chunk size: {:?}.  Buffer: {:?}", e, prev);
                            return Err(ockam_core::Error::new(
                                Origin::Transport,
                                Kind::Invalid,
                                format!("Can't parse chunked body {:?}", e),
                            ));
                        }
                    }
                }
                RequestState::RemainingInChunk(size) => {
                    if cursor.len() >= *size {
                        acc.extend_from_slice(&cursor[..*size]);
                        cursor = &cursor[*size..];
                        *self = RequestState::ParsingChunkedHeader(None);
                    } else {
                        acc.extend_from_slice(cursor);
                        *size -= cursor.len();
                        return Ok(acc);
                    }
                }
            }
        }
    }
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
            Direction::FromOutletToInlet => ockam_core::Result::Ok(Some(buffer.to_vec())),

            Direction::FromInletToOutlet => {
                let mut guard = self.state.lock().await;
                let token = self.token_refresher.get_token().await;
                if token.is_none() {
                    error!("No authorization token available");
                }
                let out = guard
                    .state
                    .process_http_buffer(buffer, &token.unwrap_or_default())?;
                Ok(Some(out))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn parse_post_with_chunked_transfers() {
        let mut data = Vec::new();
        data.extend_from_slice(REQ.as_bytes());
        data.extend_from_slice(REQ.as_bytes());

        for size in [1, 5, 32, 1024] {
            let mut result = Vec::new();
            let mut request_state = RequestState::ParsingHeader(None);
            for chunk in data.chunks(size) {
                let data_out = request_state.process_http_buffer(chunk, TOKEN).unwrap();
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
                let data_out = request_state.process_http_buffer(chunk, TOKEN).unwrap();
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
                let data_out = request_state.process_http_buffer(chunk, TOKEN).unwrap();
                result.extend_from_slice(&data_out);
            }
            assert_eq!(String::from_utf8(result).unwrap(), expected);
            assert_eq!(request_state, RequestState::ParsingHeader(None));
        }
    }
}
