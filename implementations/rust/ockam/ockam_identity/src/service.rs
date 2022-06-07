pub mod models;

mod identity_service;

pub use identity_service::*;

#[cfg(test)]
mod tests {
    use crate::service::models::*;
    use crate::service::IdentityService;
    use crate::{IdentityError, IdentityIdentifier};
    use minicbor::Decoder;
    use ockam_api::{Request, Response, Status};
    use ockam_core::{route, AsyncTryClone, Result};
    use ockam_node::Context;
    use ockam_vault::Vault;
    use rand::random;

    async fn create_identity(
        ctx: &mut Context,
        service_address: &str,
    ) -> Result<IdentityIdentifier> {
        let req = Request::post("identities").to_vec()?;

        let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
        let mut dec = Decoder::new(&receiving_buf);

        let res: Response = dec.decode()?;

        if let Some(Status::Ok) = res.status() {
        } else {
            return Err(IdentityError::ConsistencyError.into());
        }

        let res: CreateResponse = dec.decode()?;

        let identity_id = IdentityIdentifier::try_from(res.identity_id()).unwrap();

        Ok(identity_id)
    }

    async fn export_identity(
        ctx: &mut Context,
        identity_id: &str,
        service_address: &str,
    ) -> Result<Vec<u8>> {
        let req = Request::get(format!("identities/{}", identity_id)).to_vec()?;

        let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
        let mut dec = Decoder::new(&receiving_buf);

        let res: Response = dec.decode()?;

        if let Some(Status::Ok) = res.status() {
        } else {
            return Err(IdentityError::ConsistencyError.into());
        }

        let res: ExportResponse = dec.decode()?;

        Ok(res.identity().to_vec())
    }

    async fn export_as_contact(
        ctx: &mut Context,
        identity_id: &str,
        service_address: &str,
    ) -> Result<Vec<u8>> {
        let req = Request::get(format!("identities/{}/contact", identity_id)).to_vec()?;

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

    async fn import_identity(
        ctx: &mut Context,
        identity: Vec<u8>,
        service_address: &str,
    ) -> Result<()> {
        let body = ImportRequest::new(identity);
        let req = Request::post("identities/import").body(body).to_vec()?;

        let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
        let mut dec = Decoder::new(&receiving_buf);

        let res: Response = dec.decode()?;

        if let Some(Status::Ok) = res.status() {
        } else {
            return Err(IdentityError::ConsistencyError.into());
        }

        let _res: ImportResponse = dec.decode()?;

        Ok(())
    }

    async fn verify_and_add_contact(
        ctx: &mut Context,
        identity_id: &str,
        contact: Vec<u8>,
        service_address: &str,
    ) -> Result<()> {
        let body = VerifyAndAddContactRequest::new(contact);
        let req = Request::post(format!("identities/{}/verify_and_add_contact", identity_id))
            .body(body)
            .to_vec()?;

        let receiving_buf: Vec<u8> = ctx.send_and_receive(route![service_address], req).await?;
        let mut dec = Decoder::new(&receiving_buf);

        let res: Response = dec.decode()?;

        if let Some(Status::Ok) = res.status() {
        } else {
            return Err(IdentityError::ConsistencyError.into());
        }

        Ok(())
    }

    async fn create_proof(
        ctx: &mut Context,
        identity_id: &str,
        state: &[u8],
        service_address: &str,
    ) -> Result<Vec<u8>> {
        let body = CreateAuthProofRequest::new(state);
        let req = Request::post(format!("identities/{}/create_auth_proof", identity_id))
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
        identity_id: &str,
        peer_identity_id: &str,
        state: &[u8],
        proof: &[u8],
        service_address: &str,
    ) -> Result<bool> {
        let body = VerifyAuthProofRequest::new(peer_identity_id, state, proof);
        let req = Request::post(format!("identities/{}/verify_auth_proof", identity_id))
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
        IdentityService::create(ctx, "01", vault1.async_try_clone().await?).await?;
        IdentityService::create(ctx, "02", vault2.async_try_clone().await?).await?;
        IdentityService::create(ctx, "11", vault1.async_try_clone().await?).await?;
        IdentityService::create(ctx, "12", vault2.async_try_clone().await?).await?;

        let identity_id1 = create_identity(ctx, "01").await?.to_string();
        let identity_id2 = create_identity(ctx, "02").await?.to_string();

        let identity1 = export_identity(ctx, &identity_id1, "01").await?;
        let identity2 = export_identity(ctx, &identity_id2, "02").await?;

        import_identity(ctx, identity1, "11").await?;
        import_identity(ctx, identity2, "12").await?;

        let contact1 = export_as_contact(ctx, &identity_id1, "11").await?;
        let contact2 = export_as_contact(ctx, &identity_id2, "12").await?;

        verify_and_add_contact(ctx, &identity_id1, contact2, "11").await?;
        verify_and_add_contact(ctx, &identity_id2, contact1, "12").await?;

        let state: [u8; 32] = random();

        let proof1 = create_proof(ctx, &identity_id1, &state, "11").await?;
        let proof2 = create_proof(ctx, &identity_id2, &state, "12").await?;

        let verified1 =
            verify_proof(ctx, &identity_id1, &identity_id2, &state, &proof2, "11").await?;
        let verified2 =
            verify_proof(ctx, &identity_id2, &identity_id1, &state, &proof1, "12").await?;

        assert!(verified1);
        assert!(verified2);

        ctx.stop().await?;

        Ok(())
    }
}
