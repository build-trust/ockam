pub mod models;

mod identity_service;

pub use identity_service::*;

#[cfg(test)]
mod tests {
    use crate::service::models::*;
    use crate::service::IdentityService;
    use crate::IdentityError;
    use minicbor::Decoder;
    use ockam_api::{Request, Response, Status};
    use ockam_core::{route, AsyncTryClone, Result};
    use ockam_node::Context;
    use ockam_vault::Vault;
    use rand::random;

    async fn create_identity(
        ctx: &mut Context,
        service_address: &str,
    ) -> Result<(Vec<u8>, String)> {
        let req = Request::post("identities").to_vec()?;

        let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
        let mut dec = Decoder::new(&receiving_buf);

        let res: Response = dec.decode()?;

        if let Some(Status::Ok) = res.status() {
        } else {
            return Err(IdentityError::ConsistencyError.into());
        }

        let res: CreateResponse = dec.decode()?;

        Ok((res.identity().to_vec(), res.identity_id().to_string()))
    }

    async fn export_as_contact(
        ctx: &mut Context,
        identity: &[u8],
        service_address: &str,
    ) -> Result<Vec<u8>> {
        let body = ContactRequest::new(identity);
        let req = Request::get("identities/contact").body(body).to_vec()?;

        let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
        let mut dec = Decoder::new(&receiving_buf);

        let res: Response = dec.decode()?;

        if let Some(Status::Ok) = res.status() {
        } else {
            return Err(IdentityError::ConsistencyError.into());
        }

        let res: ContactResponse = dec.decode()?;

        Ok(res.contact().to_vec())
    }

    async fn verify_and_add_contact(
        ctx: &mut Context,
        identity: &[u8],
        contact: Vec<u8>,
        service_address: &str,
    ) -> Result<(Vec<u8>, String)> {
        let body = VerifyAndAddContactRequest::new(identity, contact);
        let req = Request::post("identities/verify_and_add_contact")
            .body(body)
            .to_vec()?;

        let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
        let mut dec = Decoder::new(&receiving_buf);

        let res: Response = dec.decode()?;

        if let Some(Status::Ok) = res.status() {
        } else {
            return Err(IdentityError::ConsistencyError.into());
        }

        let res: VerifyAndAddContactResponse = dec.decode()?;

        Ok((res.identity().to_vec(), res.contact_id().to_string()))
    }

    async fn create_proof(
        ctx: &mut Context,
        identity: &[u8],
        state: &[u8],
        service_address: &str,
    ) -> Result<Vec<u8>> {
        let body = CreateAuthProofRequest::new(identity, state);
        let req = Request::post("identities/create_auth_proof")
            .body(body)
            .to_vec()?;

        let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
        let mut dec = Decoder::new(&receiving_buf);

        let res: Response = dec.decode()?;

        if let Some(Status::Ok) = res.status() {
        } else {
            return Err(IdentityError::ConsistencyError.into());
        }

        let res: CreateAuthProofResponse = dec.decode()?;

        Ok(res.proof().to_vec())
    }

    async fn verify_proof(
        ctx: &mut Context,
        identity: &[u8],
        peer_identity_id: &str,
        state: &[u8],
        proof: &[u8],
        service_address: &str,
    ) -> Result<bool> {
        let body = VerifyAuthProofRequest::new(identity, peer_identity_id, state, proof);
        let req = Request::post("identities/verify_auth_proof")
            .body(body)
            .to_vec()?;

        let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
        let mut dec = Decoder::new(&receiving_buf);

        let res: Response = dec.decode()?;

        if let Some(Status::Ok) = res.status() {
        } else {
            return Err(IdentityError::ConsistencyError.into());
        }

        let res: VerifyAuthProofResponse = dec.decode()?;

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

        let contact1 = export_as_contact(ctx, &identity1, "1").await?;
        let contact2 = export_as_contact(ctx, &identity2, "2").await?;

        // Identity is updated here
        let (identity1, identity_id2) =
            verify_and_add_contact(ctx, &identity1, contact2, "1").await?;
        let (identity2, identity_id1) =
            verify_and_add_contact(ctx, &identity2, contact1, "2").await?;

        let state: [u8; 32] = random();

        let proof1 = create_proof(ctx, &identity1, &state, "1").await?;
        let proof2 = create_proof(ctx, &identity2, &state, "2").await?;

        let verified1 = verify_proof(ctx, &identity1, &identity_id2, &state, &proof2, "1").await?;
        let verified2 = verify_proof(ctx, &identity2, &identity_id1, &state, &proof1, "2").await?;

        assert!(verified1);
        assert!(verified2);

        ctx.stop().await?;

        Ok(())
    }
}
