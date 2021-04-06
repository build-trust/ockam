use crate::error::Error;
use crate::lib::*;

use bls12_381_plus::{G1Projective, Scalar};
use digest::Update;
use ff::Field;
use group::Curve;
use rand_core::RngCore;
use subtle::ConstantTimeEq;
use typenum::NonZero;

#[derive(Clone, Debug)]
struct ProofCommittedBuilderCache<P, S>
where
    P: ArrayLength<G1Projective> + NonZero + Clone,
    S: ArrayLength<Scalar> + NonZero + Clone,
{
    commitment: G1Projective,
    points: Vec<G1Projective, P>,
    scalars: Vec<Scalar, S>,
}

impl<P, S> Default for ProofCommittedBuilderCache<P, S>
where
    P: ArrayLength<G1Projective> + NonZero + Clone,
    S: ArrayLength<Scalar> + NonZero + Clone,
{
    fn default() -> Self {
        Self {
            commitment: G1Projective::identity(),
            points: Vec::new(),
            scalars: Vec::new(),
        }
    }
}

impl<P, S> PartialEq<ProofCommittedBuilder<P, S>> for ProofCommittedBuilderCache<P, S>
where
    P: ArrayLength<G1Projective> + NonZero + Clone,
    S: ArrayLength<Scalar> + NonZero + Clone,
{
    fn eq(&self, other: &ProofCommittedBuilder<P, S>) -> bool {
        if self.points.len() != other.points.len() {
            return false;
        }
        let mut res = 1u8;
        for i in 0..self.points.len() {
            res &= self.points[i].ct_eq(&other.points[i]).unwrap_u8();
        }
        if res == 1 {
            true
        } else {
            false
        }
    }
}

/// A builder struct for creating a proof of knowledge
/// of messages in a vector commitment
/// each message has a blinding factor
#[derive(Clone, Debug)]
pub struct ProofCommittedBuilder<P, S>
where
    P: ArrayLength<G1Projective> + NonZero,
    S: ArrayLength<Scalar> + NonZero,
{
    cache: ProofCommittedBuilderCache<P, S>,
    points: Vec<G1Projective, P>,
    scalars: Vec<Scalar, S>,
}

impl<P, S> Default for ProofCommittedBuilder<P, S>
where
    P: ArrayLength<G1Projective> + NonZero,
    S: ArrayLength<Scalar> + NonZero,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<P, S> ProofCommittedBuilder<P, S>
where
    P: ArrayLength<G1Projective> + NonZero,
    S: ArrayLength<Scalar> + NonZero,
{
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            cache: ProofCommittedBuilderCache::default(),
            points: Vec::new(),
            scalars: Vec::new(),
        }
    }

    /// Add a specified point and generate a random blinding factor
    pub fn commit_random(&mut self, point: G1Projective, rng: impl RngCore) {
        let r = Scalar::random(rng);
        self.points.push(point).unwrap();
        self.scalars.push(r).unwrap();
    }

    /// Commit a specified point with the specified scalar
    pub fn commit(&mut self, point: G1Projective, scalar: Scalar) {
        self.points.push(point).unwrap();
        self.scalars.push(scalar).unwrap();
    }

    /// Return the point and blinding factor at the specified index
    pub fn get(&self, index: usize) -> Option<(G1Projective, Scalar)> {
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
            let commitment =
                G1Projective::sum_of_products_in_place(self.points.as_ref(), scalars.as_mut());
            self.cache = ProofCommittedBuilderCache {
                points: self.points.clone(),
                scalars,
                commitment,
            }
        }

        hasher.update(self.cache.commitment.to_affine().to_uncompressed());
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
        for i in 0..self.cache.scalars.len() {
            self.cache.scalars[i] += secrets[i] * challenge;
        }
        Ok(self.cache.scalars)
    }
}
