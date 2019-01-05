package entity

import (
	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
)

// Entity is
type Entity struct {
	id      *did.DID
	signers []ockam.Signer
}

// Option is
type Option func(*Entity)

// New creates
func New(options ...Option) (*Entity, error) {
	e := &Entity{}

	for _, option := range options {
		option(e)
	}

	if e.id == nil {
		if len(e.signers) > 0 {
			e.id = e.signers[0].PublicKey().DID()
		}
	}

	return e, nil
}

// ID is
func ID(did *did.DID) Option {
	return func(e *Entity) {
		e.id = did
	}
}

// Signer is
func Signer(s ockam.Signer) Option {
	return func(e *Entity) {
		// s.PublicKey().SetOwner(e)
		e.signers = append(e.signers, s)
	}
}

// ID is
func (e *Entity) ID() *did.DID {
	return e.id
}

// Signers is
func (e *Entity) Signers() []ockam.Signer {
	return e.signers
}

// PublicKeys is
func (e *Entity) PublicKeys() []ockam.PublicKey {
	var pubKeys []ockam.PublicKey
	for _, signer := range e.signers {
		pubKeys = append(pubKeys, signer.PublicKey())
	}
	return pubKeys
}
