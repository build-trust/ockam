use minicbor::Decoder;
use ockam_api::vault::models::{
    CreateSecretRequest, CreateSecretResponse, PublicKeyResponse, SignRequest, SignResponse,
    VerifyRequest, VerifyResponse,
};
use ockam_api::vault::VaultService;
use ockam_core::api::{Request, Response, Status};
use ockam_core::vault::{SecretAttributes, SecretPersistence, SecretType};
use ockam_core::{route, AllowAll, Result};
use ockam_node::Context;
use ockam_vault::Vault;

#[ockam_macros::test]
async fn full_flow(ctx: &mut Context) -> Result<()> {
    // Start service
    let vault = Vault::create();
    let service = VaultService::new(vault);

    ctx.start_worker("vault_service", service, AllowAll, AllowAll)
        .await?;

    // Generate Ed25519 Key
    let body = CreateSecretRequest::new_generate(SecretAttributes::new(
        SecretType::Ed25519,
        SecretPersistence::Ephemeral,
        0,
    ));

    let req = Request::post("secrets").body(body);

    let mut sending_buf = Vec::new();
    req.encode(&mut sending_buf)?;

    let receiving_buf: Vec<u8> = ctx
        .send_and_receive(route!["vault_service"], sending_buf)
        .await?;
    let mut dec = Decoder::new(&receiving_buf);

    let res: Response = dec.decode()?;

    if let Some(Status::Ok) = res.status() {
    } else {
        panic!()
    }

    let res: CreateSecretResponse = dec.decode()?;

    let key_id = res.key_id().to_string();

    // Get public key
    let req = Request::get(format!("secrets/{key_id}/public_key")).body(key_id.clone());

    let mut sending_buf = Vec::new();
    req.encode(&mut sending_buf)?;

    let receiving_buf: Vec<u8> = ctx
        .send_and_receive(route!["vault_service"], sending_buf)
        .await?;
    let mut dec = Decoder::new(&receiving_buf);

    let res: Response = dec.decode()?;

    if let Some(Status::Ok) = res.status() {
    } else {
        panic!()
    }

    let res: PublicKeyResponse = dec.decode()?;

    let public_key = res.public_key().clone();

    // Sign some data
    let body = SignRequest::new(key_id, b"test".as_slice());
    let req = Request::post("sign").body(body);

    let mut sending_buf = Vec::new();
    req.encode(&mut sending_buf)?;

    let receiving_buf: Vec<u8> = ctx
        .send_and_receive(route!["vault_service"], sending_buf)
        .await?;
    let mut dec = Decoder::new(&receiving_buf);

    let res: Response = dec.decode()?;

    if let Some(Status::Ok) = res.status() {
    } else {
        panic!()
    }

    let res: SignResponse = dec.decode()?;

    let signature = res.signature();

    // Verify the signature
    let body = VerifyRequest::new(signature, public_key, b"test".as_slice());
    let req = Request::post("verify").body(body);

    let mut sending_buf = Vec::new();
    req.encode(&mut sending_buf)?;

    let receiving_buf: Vec<u8> = ctx
        .send_and_receive(route!["vault_service"], sending_buf)
        .await?;
    let mut dec = Decoder::new(&receiving_buf);

    let res: Response = dec.decode()?;

    if let Some(Status::Ok) = res.status() {
    } else {
        panic!()
    }

    let res: VerifyResponse = dec.decode()?;

    assert!(res.verified());

    ctx.stop().await?;

    Ok(())
}
