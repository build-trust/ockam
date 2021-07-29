use crate::SoftwareVault;
use ockam_core::Result;
use vsss_rs::{Shamir, Share};

use crate::VaultError;
use rand_core::{CryptoRng, RngCore};
use ockam_vault_core::{SecretKeyShareVault, SecretKey as OckamVaultSecretKey, PublicKey as OckamVaultPublicKey};
use signature_bls::SecretKey as BlsSecretKey;
use bls12_381_plus::{multi_miller_loop, Scalar, ExpandMsgXmd, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective};
use array_macro::array;
use core::{
    ops::{BitOr, Not, Neg}
};
use group::{Group, Curve};

const BYTES: usize = 32;
const PARTIAL_SIGNATURE_BYTES: usize = 49;
const SECRET_KEY_SHARE_BYTES: usize = 33;
const DST: &'static [u8] = b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_POP_";

impl SecretKeyShareVault for SoftwareVault {

    fn split_secret<R: CryptoRng + RngCore, const T: usize, const N: usize>(
        &self,
        secret_key: &OckamVaultSecretKey,
        rng: &mut R
    ) -> Result<[OckamVaultSecretKey; N]>{

        let sk_bytes = secret_key.as_ref();
        let mut t = [0u8; BYTES];
        t.copy_from_slice(sk_bytes);

        let sk = match Option::<signature_bls::SecretKey>::from(BlsSecretKey::from_bytes(&t)){
            Some(sk) => sk,
            None => return Err(Into::<ockam_core::Error>::into(VaultError::SplitSecretError))
        };
        let shares = Shamir::<T, N>::split_secret::<Scalar, R, SECRET_KEY_SHARE_BYTES>(sk.0, rng)
            .map_err(|_| Into::<ockam_core::Error>::into(VaultError::SplitSecretError))?;

        let mut secret_shares: [OckamVaultSecretKey; N] = array![OckamVaultSecretKey::new(Vec::new()); N];

        for (i, s) in shares.iter().enumerate() {
            secret_shares[i] = OckamVaultSecretKey::new(s.as_ref().to_vec());
        }
        Ok(secret_shares)

    }

    fn combine_shares<const T: usize, const N: usize>(
        &self, 
        secret_shares: &[OckamVaultSecretKey]) -> Result<OckamVaultSecretKey>{
        if T > secret_shares.len() {
            return Err(Into::<ockam_core::Error>::into(VaultError::CombineSharesError));
        }
        let mut ss = [Share::<SECRET_KEY_SHARE_BYTES>::default(); T];
        for i in 0..T {
            let mut bytes = [0u8; SECRET_KEY_SHARE_BYTES];
            bytes.copy_from_slice(secret_shares[i].as_ref());
            ss[i] = Share(bytes);
        }
        let scalar = Shamir::<T, N>::combine_shares::<Scalar, SECRET_KEY_SHARE_BYTES>(&ss)
            .map_err(|_| Into::<ockam_core::Error>::into(VaultError::CombineSharesError))?;

        Ok(OckamVaultSecretKey::new(BlsSecretKey(scalar).to_bytes().to_vec()))
    }

    fn partial_sign<B: AsRef<[u8]>>(
        &self, 
        secret_share: &OckamVaultSecretKey, 
        msg: &B) -> Result<[u8; PARTIAL_SIGNATURE_BYTES]> {

        let mut bytes_identifier = [0u8; SECRET_KEY_SHARE_BYTES];
        bytes_identifier.copy_from_slice(&secret_share.as_ref());
        let share = Share(bytes_identifier);
        if share.is_zero() {
            return Err(Into::<ockam_core::Error>::into(VaultError::PartialSignError));
        }
        
        let mut bytes_no_identifier = [0u8; SECRET_KEY_SHARE_BYTES -1 ];
        bytes_no_identifier.copy_from_slice(&secret_share.as_ref()[1..]);

        let sk = match Option::<Scalar>::from(Scalar::from_bytes(&bytes_no_identifier)){
            Some(sk) => sk,
            None => return Err(Into::<ockam_core::Error>::into(VaultError::PartialSignError))
        };
        
        let h = G1Projective::hash::<ExpandMsgXmd<sha2::Sha256>>(msg.as_ref(), DST);
        let signature = h * sk;

        let mut bytes = [0u8; PARTIAL_SIGNATURE_BYTES];
        bytes[1..].copy_from_slice(&signature.to_affine().to_compressed());
        bytes[0] = secret_share.as_ref()[0];

        Ok(bytes)
    }

    fn verify_signatures<B: AsRef<[u8]>>(
        &self, 
        signature: &[u8; PARTIAL_SIGNATURE_BYTES -1], 
        public_key: &OckamVaultPublicKey, 
        msg: &B) -> Result<bool>{

        let pk_bytes = public_key.as_ref();
        let mut t = [0u8; 96];
        t.copy_from_slice(pk_bytes);
        
        let pk = match Option::<G2Projective>::from(G2Affine::from_compressed(&t).map(|p| G2Projective::from(&p))){
            Some(pk) => pk,
            None => return Err(Into::<ockam_core::Error>::into(VaultError::VerifySignaturesError))
        };
        let is_pk_on_curve = pk.is_on_curve().not();
        let is_pk_invalid = pk.is_identity().bitor(is_pk_on_curve);
        if pk.is_identity().bitor(is_pk_invalid).unwrap_u8() == 1 {
            return Err(Into::<ockam_core::Error>::into(VaultError::VerifySignaturesError));
        }
        let a = G1Projective::hash::<ExpandMsgXmd<sha2::Sha256>>(msg.as_ref(), DST);
        let g2 = G2Affine::generator().neg();

        let g1 = match Option::<G1Affine>::from(G1Affine::from_compressed(signature)){
            Some(g1) => g1,
            None => return Err(Into::<ockam_core::Error>::into(VaultError::VerifySignaturesError))
        };

        let result = multi_miller_loop(&[
            (&a.to_affine(), &G2Prepared::from(pk.to_affine())),
            (&g1, &G2Prepared::from(g2)),
            ]).final_exponentiation()
            .is_identity().unwrap_u8() == 1;

        Ok(result)
    }

    fn combine_signatures<const T: usize, const N: usize>(
        &self, 
        signatures: &[[u8; PARTIAL_SIGNATURE_BYTES]],) -> Result<[u8; PARTIAL_SIGNATURE_BYTES -1 ]> {

        if T > signatures.len() {
            return Err(Into::<ockam_core::Error>::into(VaultError::CombineSignaturesError));
        }
        let mut pp = [Share::<PARTIAL_SIGNATURE_BYTES>::default(); T];
        for i in 0..T {
            let mut bytes = [0u8; PARTIAL_SIGNATURE_BYTES];
            bytes.copy_from_slice(&signatures[i]);
        
            pp[i] = Share(bytes);
        }
        let point =
            Shamir::<T, N>::combine_shares_group::<Scalar, G1Projective, PARTIAL_SIGNATURE_BYTES>(&pp)
            .map_err(|_| Into::<ockam_core::Error>::into(VaultError::CombineSignaturesError))?;

        let mut bytes = [0u8; PARTIAL_SIGNATURE_BYTES-1];
        bytes.copy_from_slice(&point.to_affine().to_compressed());
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use crate::SoftwareVault;
    use crate::MockRng;
    use rand_core::SeedableRng;
    use ockam_vault_core::{SecretKeyShareVault, SecretAttributes, SecretPersistence, SecretType, SecretVault};
    use array_macro::array;
    use crate::Secret;

    #[test]
    fn mpc_threashold_signing() {
        const THREASHOLD_NUMBER: usize = 2;
        const TOTAL_NUMBER: usize = 3;  
        let mut vault = SoftwareVault::new();

        let attributes = SecretAttributes::new(
            SecretType::Bls,
            SecretPersistence::Ephemeral,
            super::BYTES,
        );

        //Generate keys
        let sk_ctx_1 = vault.secret_generate(attributes).unwrap();
        let pk = vault.secret_public_key_get(&sk_ctx_1).unwrap();
        let sk = vault.secret_export(&sk_ctx_1).unwrap();

        //Generate shares from random key
        let seed = [1u8; 16];
        let mut rng = MockRng::from_seed(seed);
        let shares = vault.split_secret::<MockRng, THREASHOLD_NUMBER, TOTAL_NUMBER>(&sk, &mut rng).unwrap();
   
        
        let mut secrets: Vec<Secret> = Vec::new();
        let mut vaults: Vec<SoftwareVault> = Vec::new();
        let attributes_share = SecretAttributes::new(
            SecretType::BlsShare,
            SecretPersistence::Ephemeral,
            super::BYTES
        );
        //Create 3 Vaults this simulates the where 3 nodes receive a secret share and store in his vault
        for i in 0..TOTAL_NUMBER{ 
            vaults.push(SoftwareVault::new());     
            let secret = vaults[i].secret_import(&shares[i].as_ref(), attributes_share).unwrap();
            secrets.push(secret);    
        }
        //message to be signed
        let msg = [0u8; 12];
       
        //sign the message using with secret shares (extracting from the respective vault)
        let mut partial_sigs = array![[0; super::PARTIAL_SIGNATURE_BYTES]; TOTAL_NUMBER];
        for i in 0..TOTAL_NUMBER{     
            let secret = &secrets[i];
            let sk = vaults[i].secret_export(secret).unwrap();
            partial_sigs[i] = vaults[i].partial_sign(&sk, &msg).unwrap();
        }

        //Generate keys to fail the validation
        let msg_fail = [1u8; 12];
        let sk_ctx_fail = vault.secret_generate(attributes).unwrap();
        let pk_fail = vault.secret_public_key_get(&sk_ctx_fail).unwrap();

        for i in 0..3 {
            for j in 0..3 {
                if i == j {
                    continue;
                }
                //Partial Signatures are sent to another node that will combine everything
                let signature = vault.combine_signatures::<THREASHOLD_NUMBER, TOTAL_NUMBER>(&[partial_sigs[i], partial_sigs[j]]).unwrap();

                //the resulting signature are compared with the final public key and the message.
                let result_valid = vault.verify_signatures(&signature, &pk, &msg).unwrap();
                let result_pk_fail = vault.verify_signatures(&signature, &pk_fail, &msg).unwrap();
                let result_msg_fail = vault.verify_signatures(&signature, &pk, &msg_fail).unwrap();
                let result_msg_pk_fail = vault.verify_signatures(&signature, &pk_fail, &msg_fail).unwrap();
                assert_eq!(result_valid, true);
                assert_eq!(result_pk_fail, false);
                assert_eq!(result_msg_fail, false);
                assert_eq!(result_msg_pk_fail, false);
            }
        }
    }
}