package entity

import (
	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
)

// Attributes is
type Attributes map[string]interface{}

// Entity is
type Entity struct {
	id         *did.DID
	signers    []ockam.Signer
	attributes Attributes
}

// New creates an initialized Entity{} object with a valid id attribute
// and returns the Entity{} pointer and an error if present
func New(options ...Option) (*Entity, error) {
	e := &Entity{}

	for _, option := range options {
		if err := option(e); err != nil {
			return e, err
		}
	}

	if e.id == nil {
		if len(e.signers) > 0 {
			e.id = e.signers[0].PublicKey().DID()
		}
	}

	return e, nil
}

// ID is
func (e *Entity) ID() *did.DID {
	return e.id
}

// Attributes is
func (e *Entity) Attributes() map[string]interface{} {
	return e.attributes
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
