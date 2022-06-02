use crate::service::models::*;
use crate::Vault;
use minicbor::encode::Write;
use minicbor::{Decoder, Encode, Encoder};
use ockam_api::{CowStr, Error, Id, Method, Request, Response, Status};
use ockam_core::compat::io;
use ockam_core::vault::{
    AsymmetricVault, Hasher, KeyId, SecretVault, Signature, Signer, SymmetricVault, Verifier,
};
use ockam_core::{Result, Routed, Worker};
use ockam_node::Context;
use tracing::trace;

/// Vault Service Worker
pub struct VaultService {
    vault: Vault,
}

impl VaultService {
    /// Constructor
    pub fn new(vault: Vault) -> Self {
        Self { vault }
    }
}

impl VaultService {
    fn response_for_bad_request<W>(req: &Request, msg: &str, enc: &mut Encoder<W>) -> Result<()>
    where
        W: Write<Error = io::Error>,
    {
        let error = Error::new(req.path()).with_message(msg);

        let error = if let Some(m) = req.method() {
            error.with_method(m)
        } else {
            error
        };

        Response::bad_request(req.id())
            .body(error)
            .encode_with_encoder(enc)?;

        Ok(())
    }

    fn ok_response<W, B>(req: &Request, body: Option<B>, enc: &mut Encoder<W>) -> Result<()>
    where
        W: Write<Error = io::Error>,
        B: Encode<()>,
    {
        Response::ok(req.id()).body(body).encode_with_encoder(enc)?;

        Ok(())
    }

    fn response_with_error<W>(
        req: Option<&Request>,
        status: Status,
        error: &str,
        enc: &mut Encoder<W>,
    ) -> Result<()>
    where
        W: Write<Error = io::Error>,
    {
        let (path, req_id) = match req {
            None => ("", Id::fresh()),
            Some(req) => (req.path(), req.id()),
        };

        let error = Error::new(path).with_message(error);

        Response::builder(req_id, status)
            .body(error)
            .encode_with_encoder(enc)?;

        Ok(())
    }

    async fn handle_request<W>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        enc: &mut Encoder<W>,
    ) -> Result<()>
    where
        W: Write<Error = io::Error>,
    {
        trace! {
            target: "ockam_vault::service",
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        let method = match req.method() {
            Some(m) => m,
            None => return Self::response_for_bad_request(req, "empty method", enc),
        };

        use Method::*;

        match method {
            Get => match req.path_segments::<3>().as_slice() {
                ["secrets", key_id] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<GetSecretRequest>()?;

                    let key_id: KeyId = key_id.to_string();

                    match args.operation() {
                        GetSecretRequestOperation::GetAttributes => {
                            let resp = self.vault.secret_attributes_get(&key_id).await?;
                            let body = GetSecretAttributesResponse::new(resp);

                            Self::ok_response(req, Some(body), enc)
                        }
                        GetSecretRequestOperation::GetSecretBytes => {
                            let resp = self.vault.secret_export(&key_id).await?;
                            let body = ExportSecretResponse::new(resp.as_ref());

                            Self::ok_response(req, Some(body), enc)
                        }
                    }
                }
                ["secrets", key_id, "public_key"] => {
                    let key_id: KeyId = key_id.to_string();

                    let public_key = self.vault.secret_public_key_get(&key_id).await?;
                    let body = PublicKeyResponse::new(public_key);

                    Self::ok_response(req, Some(body), enc)
                }
                _ => Self::response_for_bad_request(req, "unknown path", enc),
            },
            Post => match req.path_segments::<3>().as_slice() {
                ["secrets"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<CreateSecretRequest>()?;

                    let attributes = *args.attributes();

                    let key_id = match args.secret() {
                        Some(secret) => {
                            self.vault
                                .secret_import(secret.as_ref(), attributes)
                                .await?
                        }
                        None => self.vault.secret_generate(attributes).await?,
                    };

                    let body = CreateSecretResponse::new(key_id);

                    Self::ok_response(req, Some(body), enc)
                }
                ["ecdh"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<EcdhRequest>()?;

                    let (secret_key_id, public_key) = args.into_parts();
                    let secret_key_id: KeyId = secret_key_id.into_owned();

                    let dh = self
                        .vault
                        .ec_diffie_hellman(&secret_key_id, &public_key)
                        .await?;

                    Self::ok_response(req, Some(EcdhResponse::new(dh)), enc)
                }
                ["compute_key_id"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<ComputeKeyIdRequest>()?;

                    let key_id = self
                        .vault
                        .compute_key_id_for_public_key(args.public_key())
                        .await?;

                    Self::ok_response(req, Some(ComputeKeyIdResponse::new(key_id)), enc)
                }
                ["sha256"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<Sha256Request>()?;

                    let hash = self.vault.sha256(args.data()).await?;

                    Self::ok_response(req, Some(Sha256Response::new(hash)), enc)
                }
                ["hkdf"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<HkdfSha256Request>()?;

                    let salt: KeyId = args.salt().to_string();
                    let ikm = args.ikm().map(|i| i.to_string());

                    let output = self
                        .vault
                        .hkdf_sha256(
                            &salt,
                            args.info(),
                            ikm.as_ref(),
                            args.output_attributes().to_vec(),
                        )
                        .await?;

                    Self::ok_response(
                        req,
                        Some(HkdfSha256Response::new(
                            output.into_iter().map(CowStr::from).collect(),
                        )),
                        enc,
                    )
                }
                ["sign"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<SignRequest>()?;

                    let key_id: KeyId = args.key_id().to_string();

                    let output = self.vault.sign(&key_id, args.data()).await?;

                    Self::ok_response(req, Some(SignResponse::new(output.as_ref())), enc)
                }
                ["verify"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<VerifyRequest>()?;

                    // TODO: Optimize?
                    let signature = Signature::new(args.signature().to_vec());

                    let output = self
                        .vault
                        .verify(&signature, args.public_key(), args.data())
                        .await?;

                    Self::ok_response(req, Some(VerifyResponse::new(output)), enc)
                }
                ["encrypt"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<EncryptRequest>()?;

                    let key_id: KeyId = args.key_id().to_string();

                    let output = self
                        .vault
                        .aead_aes_gcm_encrypt(&key_id, args.plaintext(), args.nonce(), args.aad())
                        .await?;

                    Self::ok_response(req, Some(EncryptResponse::new(output)), enc)
                }
                ["decrypt"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<DecryptRequest>()?;

                    let key_id: KeyId = args.key_id().to_string();

                    let output = self
                        .vault
                        .aead_aes_gcm_decrypt(&key_id, args.ciphertext(), args.nonce(), args.aad())
                        .await?;

                    Self::ok_response(req, Some(DecryptResponse::new(output)), enc)
                }
                _ => Self::response_for_bad_request(req, "unknown path", enc),
            },
            Delete => match req.path_segments::<2>().as_slice() {
                ["secrets", key_id] => {
                    let key_id: KeyId = key_id.to_string();

                    self.vault.secret_destroy(key_id).await?;

                    #[allow(unused_qualifications)]
                    Self::ok_response(req, Option::<()>::None, enc)
                }
                _ => Self::response_for_bad_request(req, "unknown path", enc),
            },
            Put | Patch => Self::response_for_bad_request(req, "unknown method", enc),
        }
    }

    async fn on_request<W>(&mut self, data: &[u8], buf: W) -> Result<()>
    where
        W: Write<Error = io::Error>,
    {
        let mut enc = Encoder::new(buf);

        let mut dec = Decoder::new(data);
        let req: Request = match dec.decode() {
            Ok(r) => r,
            Err(_) => {
                return Self::response_with_error(
                    None,
                    Status::BadRequest,
                    "invalid Request structure",
                    &mut enc,
                )
            }
        };

        match self.handle_request(&req, &mut dec, &mut enc).await {
            Ok(_) => Ok(()),
            Err(err) => Self::response_with_error(
                Some(&req),
                Status::InternalServerError,
                &err.to_string(),
                &mut enc,
            ),
        }
    }
}

#[ockam_core::worker]
impl Worker for VaultService {
    type Message = Vec<u8>;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let mut buf = Vec::new();
        self.on_request(msg.as_body(), &mut buf).await?;
        ctx.send(msg.return_route(), buf).await
    }
}
