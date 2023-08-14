use minicbor::Decoder;

use ockam::identity::identity::IdentityHistoryComparison;
use ockam::node;
use ockam_api::cli_state::CliState;
use ockam_api::identity::models::*;
use ockam_api::identity::IdentityService;
use ockam_api::nodes::service::NodeIdentities;
use ockam_core::api::{Request, Response, Status};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, AsyncTryClone, Error, Result};
use ockam_node::Context;

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

#[ockam_macros::test]
async fn full_flow(ctx: &mut Context) -> Result<()> {
    let cli_state = CliState::test().unwrap();
    let node1 = node(ctx.async_try_clone().await?);
    let node2 = node(ctx.async_try_clone().await?);

    // Start services
    ctx.start_worker(
        "1",
        IdentityService::new(NodeIdentities::new(node1.identities(), cli_state.clone())).await?,
    )
    .await?;
    ctx.start_worker(
        "2",
        IdentityService::new(NodeIdentities::new(node2.identities(), cli_state)).await?,
    )
    .await?;

    let (identity1, _identity_id1) = create_identity(ctx, "1").await?;
    let (identity2, _identity_id2) = create_identity(ctx, "2").await?;

    // Identity is updated here
    let _identity_id1 = validate_identity_change_history(ctx, &identity1, "2").await?;
    let _identity_id2 = validate_identity_change_history(ctx, &identity2, "1").await?;

    let comparison1 = compare_identity_change_history(ctx, &identity2, &[], "1").await?;
    let comparison2 = compare_identity_change_history(ctx, &identity1, &[], "2").await?;

    assert_eq!(comparison1, IdentityHistoryComparison::Newer);
    assert_eq!(comparison2, IdentityHistoryComparison::Newer);

    ctx.stop().await?;

    Ok(())
}
