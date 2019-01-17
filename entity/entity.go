package entity

import (
	"strconv"

	"github.com/pkg/errors"

	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
)

// ErrorNilTypeNotAllowed is an error type to be returned
// for any option whose passed in value requires a non-nil value
var ErrorNilTypeNotAllowed = errors.New("can not pass nil value to option")

// Attributes is
type Attributes map[string]interface{}

// Entity is
type Entity struct {
	id         *did.DID
	signers    []ockam.Signer
	attributes Attributes
}

// Option is
type Option func(*Entity) error

// New accepts an Attribute{} type and any number of functional
// Option{} types and returns an initialized Entity{} and error
func New(attributes Attributes, options ...Option) (*Entity, error) {
	e := &Entity{attributes: attributes}

	for _, option := range options {
		if err := option(e); err != nil {
			return e, err
		}
	}

	if e.id == nil {
		if len(e.signers) > 0 {
			id, err := e.signers[0].PublicKey().DID()
			if err != nil {
				return e, err
			}

			e.id = id
		}
	}

	return e, nil
}

// ID is a functional option to configure a distributed ID
// attribute on a new Entity{} type
func ID(did *did.DID) Option {
	return func(e *Entity) error {
		if did == nil {
			return ErrorNilTypeNotAllowed
		}
		e.id = did
		return nil
	}
}

// Signer is a functional option to set a ockem.Signer{}
// attribute on a new Entity{} type
func Signer(s ockam.Signer) Option {
	return func(e *Entity) error {
		if s == nil {
			return ErrorNilTypeNotAllowed
		}
		// s.PublicKey().SetOwner(e)
		s.PublicKey().SetLabel("#key-" + strconv.Itoa(len(e.signers)+1))
		e.signers = append(e.signers, s)
		return nil
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
