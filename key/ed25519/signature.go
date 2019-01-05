package ed25519

import (
	"time"

	"github.com/ockam-network/ockam"
)

// Signature is
type Signature struct {
	t              string
	creator        ockam.PublicKey
	created        time.Time
	domain         string
	nonce          []byte
	signatureValue []byte
}

// Type is
// https://w3c-ccg.github.io/ld-cryptosuite-registry/#ed25519signature2018
func (s *Signature) Type() string {
	return s.t
}

// Creator is
func (s *Signature) Creator() ockam.PublicKey {
	return s.creator
}

// Created is
func (s *Signature) Created() time.Time {
	return s.created
}

// Domain is
func (s *Signature) Domain() string {
	return s.domain
}

// Nonce is
// https://web-payments.org/vocabs/security#nonce
func (s *Signature) Nonce() []byte {
	return s.nonce
}

// SignatureValue is
// https://web-payments.org/vocabs/security#signatureValue
func (s *Signature) SignatureValue() []byte {
	return s.signatureValue
}
