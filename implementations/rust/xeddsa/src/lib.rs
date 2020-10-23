//! Performe XEdDSA according to
//! <https://signal.org/docs/specifications/xeddsa/#xeddsa>
#![no_std]

use curve25519_dalek::{
    constants::ED25519_BASEPOINT_POINT, montgomery::MontgomeryPoint, scalar::Scalar,
};
use ed25519_dalek::{PublicKey as EPublicKey, Signature, Verifier};
use sha2::digest::Digest;
use x25519_dalek::{PublicKey as XPublicKey, StaticSecret as XSecretKey};

pub trait XEddsaSigner {
    fn sign(&self, msg: &[u8], nonce: &[u8; 64]) -> [u8; 64];
}

pub trait XEddsaVerifier {
    fn verify(&self, msg: &[u8], sig: &[u8; 64]) -> bool;
}

impl XEddsaSigner for XSecretKey {
    fn sign(&self, msg: &[u8], nonce: &[u8; 64]) -> [u8; 64] {
        /*
         * PREPARATION OF THE KEY MATERIAL
         *
         * This algorithm to sign data using a Curve25519 key pair has to tackle two issues. The
         * first issue is that the conversion of a Curve25519 public key to an Ed25519 public key
         * is not unique when only having access to the u coordinate of the Curve25519 public
         * key, which is the case with the serialization format commonly used. In fact the
         * conversion is unique by the sign of the Ed25519 public key x coordinate. This signing
         * algorithm "solves" the problem by modifying the private key so that the sign
         * of the resulting Ed25519 public key is always zero.
         */

        // x25519-dalek private keys are already clamped, so just compute the Ed25510 public key
        // from the Curve25519 private key
        let scalar_k = Scalar::from_bits(self.to_bytes());
        let ep = ED25519_BASEPOINT_POINT * scalar_k;
        let mut ce = ep.compress();
        let sign = ce.0[31] >> 7;
        // Set the sign bit to zero after adjusting the private key accordingly
        ce.0[31] &= 0x7F; // A.s = 0

        // Compute the negative secret key

        // If the sign bit of the calculated Ed25519 public key is zero, the private key doesn't
        // have to be touched. If the sign bit is one, the private key has to be inverted
        // prior to using it.
        let k = if sign == 1 { -scalar_k } else { scalar_k };

        /*
         * SIGNING
         *
         * The second problem this algorithm has to tackle is that Ed25519 signature algorithms
         * don't use the private scalar directly but rather use a seed to derive other
         * data from. To create signatures compatible with Ed25519, a modified version of
         * the signing algorithm is required that does not depend on a seed.
         */
        //  r = hash1(a || M || Z) (mod q)
        let mut hash_padding = [0xff, 32];
        hash_padding[0] = 0xfe;
        let mut hasher = sha2::Sha512::new();
        hasher.update(hash_padding);
        hasher.update(k.as_bytes());
        hasher.update(msg);
        hasher.update(nonce.as_ref());
        let r = Scalar::from_hash(hasher);

        // R = rB
        let cap_r = (ED25519_BASEPOINT_POINT * r).compress();

        // h = hash(R || A || M) (mod q)
        hasher = sha2::Sha512::new();
        hasher.update(cap_r.as_bytes());
        hasher.update(ce.as_bytes());
        hasher.update(msg);
        let h = Scalar::from_hash(hasher);

        // s = r + ha (mod q)
        let s = r + h * k;

        // return R || s
        let mut sig = [0u8; 64];
        sig[..32].copy_from_slice(cap_r.as_bytes());
        sig[32..].copy_from_slice(s.as_bytes());
        sig
    }
}

impl XEddsaVerifier for XPublicKey {
    fn verify(&self, msg: &[u8], sig: &[u8; 64]) -> bool {
        let pt = MontgomeryPoint(self.to_bytes());
        let pk = EPublicKey::from_bytes(&pt.to_edwards(0).unwrap().compress().to_bytes()).unwrap();
        let sig = Signature::new(*sig);
        pk.verify(msg, &sig).is_ok()
    }
}

#[test]
fn convert_test() {
    let nonce = [0u8; 64];
    let msg = [0u8; 200];
    let mut privkey = [0u8; 32];
    privkey[8] = 189;
    let xsecret_key = XSecretKey::from(privkey);
    let xpublic_key = XPublicKey::from(&xsecret_key);

    let sig = xsecret_key.sign(&msg, &nonce);
    assert!(xpublic_key.verify(&msg, &sig));
}
