#[derive(Clone, Debug)]
pub struct Ed25519ProfileVerificationMethod {
    public_key: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct EcdsaP256ProfileVerificationMethod {
    public_key: Vec<u8>,
}

#[derive(Clone, Debug)]
pub enum ProfileVerificationMethod {
    Ed25519(Ed25519ProfileVerificationMethod),
    EcdsaP256(EcdsaP256ProfileVerificationMethod),
}

#[derive(Clone, Debug)]
pub enum ProfileVerificationPurpose {
    // To present a ProfileChangeProof
    ProfileUpdate,
}

#[derive(Clone, Debug)]
pub struct ProfileVerificationPolicy {
    verification_method: ProfileVerificationMethod,
    can_be_used_for: Vec<ProfileVerificationPurpose>,
}
