use minicbor::Decoder;
use ockam_api::identity::models::*;
use ockam_api::identity::IdentityService;
use ockam_core::api::{Request, Response, Status};
use ockam_core::compat::rand::random;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, AsyncTryClone, Error, Result};
use ockam_identity::change_history::IdentityHistoryComparison;
use ockam_node::Context;
use ockam_vault::Vault;

async fn create_identity(ctx: &mut Context, service_address: &str) -> Result<(Vec<u8>, String)> {
    let req = Request::post("").to_vec()?;

    let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
    let mut dec = Decoder::new(&receiving_buf);

    let res: Response = dec.decode()?;

    if let Some(Status::Ok) = res.status() {
    } else {
        return Err(Error::new(
            Origin::Identity,
            Kind::Other,
            "consistency error",
        ));
    }

    let res: CreateResponse = dec.decode()?;

    Ok((res.identity().to_vec(), res.identity_id().to_string()))
}

async fn validate_identity_change_history(
    ctx: &mut Context,
    identity: &[u8],
    service_address: &str,
) -> Result<String> {
    let body = ValidateIdentityChangeHistoryRequest::new(identity);
    let req = Request::post("actions/validate_identity_change_history")
        .body(body)
        .to_vec()?;

    let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
    let mut dec = Decoder::new(&receiving_buf);

    let res: Response = dec.decode()?;

    if let Some(Status::Ok) = res.status() {
    } else {
        return Err(Error::new(
            Origin::Identity,
            Kind::Other,
            "consistency error",
        ));
    }

    let res: ValidateIdentityChangeHistoryResponse = dec.decode()?;

    Ok(res.identity_id().to_string())
}

async fn compare_identity_change_history(
    ctx: &mut Context,
    current_identity: &[u8],
    known_identity: &[u8],
    service_address: &str,
) -> Result<IdentityHistoryComparison> {
    let body = CompareIdentityChangeHistoryRequest::new(current_identity, known_identity);
    let req = Request::post("actions/compare_identity_change_history")
        .body(body)
        .to_vec()?;

    let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
    let mut dec = Decoder::new(&receiving_buf);

    let res: Response = dec.decode()?;

    if let Some(Status::Ok) = res.status() {
    } else {
        return Err(Error::new(
            Origin::Identity,
            Kind::Other,
            "consistency error",
        ));
    }

    let res: IdentityHistoryComparison = dec.decode()?;

    Ok(res)
}

async fn create_signature(
    ctx: &mut Context,
    identity: &[u8],
    data: &[u8],
    service_address: &str,
) -> Result<Vec<u8>> {
    let body = CreateSignatureRequest::new(identity, data);
    let req = Request::post("actions/create_signature")
        .body(body)
        .to_vec()?;

    let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
    let mut dec = Decoder::new(&receiving_buf);

    let res: Response = dec.decode()?;

    if let Some(Status::Ok) = res.status() {
    } else {
        return Err(Error::new(
            Origin::Identity,
            Kind::Other,
            "consistency error",
        ));
    }

    let res: CreateSignatureResponse = dec.decode()?;

    Ok(res.signature().to_vec())
}

async fn verify_signature(
    ctx: &mut Context,
    signer_identity: &[u8],
    data: &[u8],
    signature: &[u8],
    service_address: &str,
) -> Result<bool> {
    let body = VerifySignatureRequest::new(signer_identity, data, signature);
    let req = Request::post("actions/verify_signature")
        .body(body)
        .to_vec()?;

    let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
    let mut dec = Decoder::new(&receiving_buf);

    let res: Response = dec.decode()?;

    if let Some(Status::Ok) = res.status() {
    } else {
        return Err(Error::new(
            Origin::Identity,
            Kind::Other,
            "consistency error",
        ));
    }

    let res: VerifySignatureResponse = dec.decode()?;

    Ok(res.verified())
}

#[ockam_macros::test]
async fn full_flow(ctx: &mut Context) -> Result<()> {
    let vault1 = Vault::create();
    let vault2 = Vault::create();

    // Start services
    IdentityService::create(ctx, "1", vault1.async_try_clone().await?).await?;
    IdentityService::create(ctx, "2", vault2.async_try_clone().await?).await?;

    let (identity1, _identity_id1) = create_identity(ctx, "1").await?;
    let (identity2, _identity_id2) = create_identity(ctx, "2").await?;

    // Identity is updated here
    let _identity_id2 = validate_identity_change_history(ctx, &identity2, "1").await?;
    let _identity_id1 = validate_identity_change_history(ctx, &identity2, "2").await?;

    let comparison1 = compare_identity_change_history(ctx, &identity2, &[], "1").await?;
    let comparison2 = compare_identity_change_history(ctx, &identity1, &[], "2").await?;

    assert_eq!(comparison1, IdentityHistoryComparison::Newer);
    assert_eq!(comparison2, IdentityHistoryComparison::Newer);

    let state: [u8; 32] = random();

    let proof1 = create_signature(ctx, &identity1, &state, "1").await?;
    let proof2 = create_signature(ctx, &identity2, &state, "2").await?;

    let verified1 = verify_signature(ctx, &identity2, &state, &proof2, "1").await?;
    let verified2 = verify_signature(ctx, &identity1, &state, &proof1, "2").await?;

    assert!(verified1);
    assert!(verified2);

    ctx.stop().await?;

    Ok(())
}
