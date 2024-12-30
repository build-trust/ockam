use httparse::{Header, Status};
use ockam_core::errcode::{Kind, Origin};
use std::convert::TryInto;
use tracing::error;

#[derive(Debug, Clone, PartialEq)]
pub enum RequestState {
    ParsingHeader(Option<Vec<u8>>),
    ParsingChunkedHeader(Option<Vec<u8>>),
    RemainingInChunk(usize),
    RemainingBody(usize),
}

pub(crate) trait ClientRequestWriter {
    fn write_headers(
        &self,
        request: &httparse::Request,
        buffer: &mut Vec<u8>,
    ) -> ockam_core::Result<()>;
}

impl RequestState {
    /* Parse the incoming data, attaching an Authorization header token to it.
     * data is received in chunks, and there is no warranty on what we get on each:
     * incomplete requests,  multiple requests, etc.
     */
    pub(crate) fn process_http_buffer(
        &mut self,
        buf: &[u8],
        request_writer: impl ClientRequestWriter,
    ) -> ockam_core::Result<Vec<u8>> {
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
                        Ok(Status::Partial) if prev_size == 0 => {
                            // No previous buffered, need to copy and own the unparsed data
                            *self = RequestState::ParsingHeader(Some(cursor.to_vec()));
                            return Ok(acc);
                        }
                        Ok(Status::Partial) => {
                            // There was a previous buffer, and we already added the newly data to it
                            return Ok(acc);
                        }
                        Ok(Status::Complete(body_offset)) => {
                            cursor = &cursor[body_offset - prev_size..];
                            request_writer.write_headers(&req, &mut acc)?;
                            // interceptor::attach_auth_token_and_serialize_into(&req, &mut acc);
                            *self = body_state(req.headers)?;
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

fn body_state(headers: &[Header]) -> ockam_core::Result<RequestState> {
    for h in headers {
        if h.name.eq_ignore_ascii_case("Content-Length") {
            if let Ok(str) = std::str::from_utf8(h.value) {
                return str
                    .parse()
                    .map(RequestState::RemainingBody)
                    .map_err(|e| ockam_core::Error::new(Origin::Transport, Kind::Invalid, e));
            }
        } else if h.name.eq_ignore_ascii_case("Transfer-Encoding")
            && String::from_utf8(h.value.to_vec()).is_ok_and(|s| s.contains("chunked"))
        {
            return Ok(RequestState::ParsingChunkedHeader(None));
        }
    }
    Ok(RequestState::ParsingHeader(None))
}
