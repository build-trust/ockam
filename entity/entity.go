package entity

import (
	"strconv"

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

// Option is
type Option func(*Entity)

// New creates
func New(attributes Attributes, options ...Option) (*Entity, error) {
	e := &Entity{attributes: attributes}

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
		s.PublicKey().SetLabel("#key-" + strconv.Itoa(len(e.signers)+1))
		e.signers = append(e.signers, s)
	}
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
