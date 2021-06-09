use crate::error::Error;
use crate::lib::*;

use bls12_381_plus::Scalar;
use digest::Update;
use ff::Field;
use group::{Curve, GroupEncoding};
use rand_core::RngCore;
use subtle::ConstantTimeEq;

struct ProofCommittedBuilderCache<B, C, const P: usize, const S: usize>
where
    B: Clone + Copy + Debug + Default + ConstantTimeEq + PartialEq + Eq + Curve<AffineRepr = C>,
    C: GroupEncoding + Debug,
{
    commitment: B,
    points: Vec<B, P>,
    scalars: Vec<Scalar, S>,
}

impl<B, C, const P: usize, const S: usize> Default for ProofCommittedBuilderCache<B, C, P, S>
where
    B: Clone + Copy + Debug + Default + ConstantTimeEq + PartialEq + Eq + Curve<AffineRepr = C>,
    C: GroupEncoding + Debug,
{
    fn default() -> Self {
        Self {
            commitment: B::default(),
            points: Vec::new(),
            scalars: Vec::new(),
        }
    }
}

impl<B, C, const P: usize, const S: usize> PartialEq<ProofCommittedBuilder<B, C, P, S>>
    for ProofCommittedBuilderCache<B, C, P, S>
where
    B: Clone + Copy + Debug + Default + ConstantTimeEq + PartialEq + Eq + Curve<AffineRepr = C>,
    C: GroupEncoding + Debug,
{
    fn eq(&self, other: &ProofCommittedBuilder<B, C, P, S>) -> bool {
        if self.points.len() != other.points.len() {
            return false;
        }
        let mut res = 1u8;
        for i in 0..self.points.len() {
            res &= self.points[i].ct_eq(&other.points[i]).unwrap_u8();
        }
        res == 1
    }
}

/// A builder struct for creating a proof of knowledge
/// of messages in a vector commitment
/// each message has a blinding factor
pub struct ProofCommittedBuilder<B, C, const P: usize, const S: usize>
where
    B: Clone + Copy + Debug + Default + ConstantTimeEq + PartialEq + Eq + Curve<AffineRepr = C>,
    C: GroupEncoding + Debug,
{
    cache: ProofCommittedBuilderCache<B, C, P, S>,
    points: Vec<B, P>,
    scalars: Vec<Scalar, S>,
    sum_of_products: fn(&[B], &mut [Scalar]) -> B,
}

impl<B, C, const P: usize, const S: usize> Default for ProofCommittedBuilder<B, C, P, S>
where
    B: Clone + Copy + Debug + Default + ConstantTimeEq + PartialEq + Eq + Curve<AffineRepr = C>,
    C: GroupEncoding + Debug,
{
    fn default() -> Self {
        Self::new(|_, _| B::default())
    }
}

impl<B, C, const P: usize, const S: usize> ProofCommittedBuilder<B, C, P, S>
where
    B: Clone + Copy + Debug + Default + ConstantTimeEq + PartialEq + Eq + Curve<AffineRepr = C>,
    C: GroupEncoding + Debug,
{
    /// Create a new builder
    pub fn new(sum_of_products: fn(&[B], &mut [Scalar]) -> B) -> Self {
        Self {
            cache: ProofCommittedBuilderCache::default(),
            points: Vec::new(),
            scalars: Vec::new(),
            sum_of_products,
        }
    }

    /// Add a specified point and generate a random blinding factor
    pub fn commit_random(&mut self, point: B, rng: impl RngCore) {
        let r = Scalar::random(rng);
        self.points.push(point).unwrap();
        self.scalars.push(r).unwrap();
    }

    /// Commit a specified point with the specified scalar
    pub fn commit(&mut self, point: B, scalar: Scalar) {
        self.points.push(point).unwrap();
        self.scalars.push(scalar).unwrap();
    }

    /// Return the point and blinding factor at the specified index
    pub fn get(&self, index: usize) -> Option<(B, Scalar)> {
        let p = self.points.get(index);
        let r = self.scalars.get(index);
        match (p, r) {
            (Some(point), Some(scalar)) => Some((*point, *scalar)),
            (_, _) => None,
        }
    }

    /// Convert the committed values to bytes for the fiat-shamir challenge
    pub fn add_challenge_contribution(&mut self, hasher: &mut impl Update) {
        if !self.cache.eq(self) {
            let mut scalars = self.scalars.clone();
            let commitment = (self.sum_of_products)(self.points.as_ref(), scalars.as_mut());
            self.cache = ProofCommittedBuilderCache {
                points: self.points.clone(),
                scalars,
                commitment,
            }
        }

        hasher.update(self.cache.commitment.to_affine().to_bytes());
    }

    /// Generate the Schnorr challenges given the specified secrets
    /// by computing p = r + c * s
    pub fn generate_proof(
        mut self,
        challenge: Scalar,
        secrets: &[Scalar],
    ) -> Result<Vec<Scalar, S>, Error> {
        if secrets.len() != self.cache.points.len() {
            return Err(Error::new(1, "secrets is not equal to blinding factors"));
        }

        #[allow(clippy::needless_range_loop)]
        for i in 0..self.cache.scalars.len() {
            self.cache.scalars[i] += secrets[i] * challenge;
        }
        Ok(self.cache.scalars)
    }
}

#[cfg(test)]
mod test {

    use crate::lib::ProofCommittedBuilder;
    use blake2::{Blake2b, Digest};
    use bls12_381_plus::{G1Affine, G1Projective, Scalar};

    #[test]
    fn test_proof_committed_builder() {
        let mut pb = ProofCommittedBuilder::<G1Projective, G1Affine, 2, 2>::new(
            G1Projective::sum_of_products_in_place,
        );

        let challenge = Scalar::from(3);

        let mut hasher = Blake2b::new();

        pb.commit(G1Projective::identity(), Scalar::from(2));

        pb.add_challenge_contribution(&mut hasher);
        let proof = pb.generate_proof(challenge, &[Scalar::from(1337)]).unwrap();
        assert!(!proof.is_empty());
    }
}
