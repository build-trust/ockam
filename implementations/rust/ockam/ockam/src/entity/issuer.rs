use crate::{
    Address, CredentialAttribute, CredentialFragment2, CredentialIssuer, CredentialOffer,
    CredentialRequest, CredentialSchema, OckamError, OfferIdBytes, ProfileTrait, ProofBytes,
    PublicKeyBytes, Result, Route,
};

pub struct LocalIssuingEntity<E: ProfileTrait> {
    address: Address,
    credential_issuer: CredentialIssuer,
    credential_schema: CredentialSchema,
    _entity: E,
    public_key: PublicKeyBytes,
    proof: ProofBytes,
}

impl<E: ProfileTrait> LocalIssuingEntity<E> {
    pub fn new<A: Into<Address>>(
        _entity: E,
        address: A,
        credential_schema: CredentialSchema,
    ) -> Self {
        let credential_issuer = CredentialIssuer::new(rand::thread_rng());

        let public_key: PublicKeyBytes = credential_issuer.get_public_key();
        let proof: ProofBytes = credential_issuer.create_proof_of_possession();

        Self {
            address: address.into(),
            credential_issuer,
            credential_schema,
            _entity,
            public_key,
            proof,
        }
    }

    pub fn create_offer(&self) -> CredentialOffer {
        self.credential_issuer
            .create_offer(&self.credential_schema, rand::thread_rng())
    }

    pub fn sign_credential_request(
        &self,
        credential_request: &CredentialRequest,
        attributes: &[(String, CredentialAttribute)],
        offer_id: OfferIdBytes,
    ) -> Result<CredentialFragment2> {
        // TODO better handling of conversion between CredentialError
        if let Ok(fragment) = self.credential_issuer.sign_credential_request(
            credential_request,
            &self.credential_schema,
            attributes,
            offer_id,
        ) {
            Ok(fragment)
        } else {
            Err(OckamError::InvalidParameter.into())
        }
    }
}

#[derive(Debug)]
pub struct RemoteIssuingEntity {
    route: Route,
    public_key: Option<PublicKeyBytes>,
    proof: Option<ProofBytes>,
}

impl<E: ProfileTrait> From<LocalIssuingEntity<E>> for RemoteIssuingEntity {
    fn from(local: LocalIssuingEntity<E>) -> Self {
        RemoteIssuingEntity {
            route: local.address.into(),
            public_key: Some(local.public_key),
            proof: Some(local.proof),
        }
    }
}

pub enum IssuingEntity<L: ProfileTrait> {
    Local(LocalIssuingEntity<L>),
    Remote(RemoteIssuingEntity),
}

#[cfg(test)]
mod tests {
    use crate::{
        Context, CredentialAttributeSchema, CredentialAttributeType, CredentialSchema, LocalEntity,
        LocalIssuingEntity, RemoteIssuingEntity, SECRET_ID,
    };

    pub fn example_schema() -> CredentialSchema {
        CredentialSchema {
            id: "file:///truck-schema-20210227-1_0_0".to_string(),
            label: "Truck Management".to_string(),
            description: "A Demoable schema".to_string(),
            attributes: vec![
                CredentialAttributeSchema {
                    label: SECRET_ID.to_string(),
                    description: "A unique identifier for maintenance worker. ".to_string(),
                    attribute_type: CredentialAttributeType::Blob,
                    unknown: true,
                },
                CredentialAttributeSchema {
                    label: "can_access".to_string(),
                    description: "Can worker access the truck maintenance codes?".to_string(),
                    attribute_type: CredentialAttributeType::Number,
                    unknown: false,
                },
            ],
        }
    }

    async fn local_to_remote(ctx: Context) -> ockam_core::Result<()> {
        let local = LocalEntity::create(&ctx, "issuer_entity").await?;

        let issuer = LocalIssuingEntity::new(local, "issuer", example_schema());

        let remote: RemoteIssuingEntity = issuer.into();
        println!("{:#?}", remote);

        Ok(())
    }
}
