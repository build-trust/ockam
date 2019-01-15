package entity

import (
	"errors"
	"fmt"
	"strconv"

	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
)

// Option is a functional option that represents
// an arbitrary entity configuration
type Option func(*Entity) error

// OptionAttributes is a functional option for configuring
// entity attributes.
// TODO - validate attributes, should they be non-zero?
func OptionAttributes(attributes Attributes) Option {
	return func(e *Entity) error {
		if len(attributes) == 0 {
			fmt.Printf("warn: attributes are empty")
		}

		e.attributes = attributes
		return nil
	}
}

// OptionSigner is a functional option for setting the entity
// signer attribute.
func OptionSigner(s ockam.Signer) Option {
	return func(e *Entity) error {
		if s == nil {
			return errors.New("singer can not be nil in argument to OptionSigner()")
		}
		// s.PublicKey().SetOwner(e)
		s.PublicKey().SetLabel("#key-" + strconv.Itoa(len(e.signers)+1))
		e.signers = append(e.signers, s)
		return nil
	}
}

// OptionID is a functional argument for setting the entity
// ID attribute
func OptionID(did *did.DID) Option {
	return func(e *Entity) error {
		if did == nil {
			return errors.New("can not pass nil object to OptionID")
		}
		e.id = did
		return nil
	}
}
