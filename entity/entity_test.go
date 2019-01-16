package entity

import (
	"testing"

	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
	"github.com/stretchr/testify/assert"
)

func TestNewValidEntity(t *testing.T) {
	// validate New() returns an initialized pointer to an
	// Entity{} type without throwing an error
	e, err := New(Attributes{})
	assert.Nil(t, err)
	assert.IsType(t, &Entity{}, e)

	// TODO should these be required?
	assert.Nil(t, e.ID())
	assert.Empty(t, e.Signers())
	assert.Empty(t, e.Attributes())
	assert.Empty(t, e.PublicKeys())
}

func TestNewEntityWithOptionSigner(t *testing.T) {
	// validate New() returns an initialized pointer to an
	// Entity{} type when called with Signer() option and
	// ensure functions on this type which depend on Singer
	// being present behave as expected.
	e, err := New(Attributes{}, Signer(mockSigner{}))
	assert.Nil(t, err)
	assert.NotEmpty(t, e.Signers())
	assert.Len(t, e.Signers(), 1)
	assert.NotEmpty(t, e.PublicKeys())
	assert.Len(t, e.PublicKeys(), 1)
}

func TestNewEntityWithOptionID(t *testing.T) {
	// validate New() returns an initialized pointer to a
	// Entity{} type with a non-nil ID attribute when setting
	// OptionID
	e, err := New(Attributes{}, ID(&did.DID{}))
	assert.Nil(t, err)
	assert.NotNil(t, e.ID())
}

func TestNewEntityWithOptionAttributes(t *testing.T) {
	// validate New() returns an initialized pointer to a
	// Entity{} type when called with OptionAttributes and
	// functions on this type which depend on that option behave
	// as expected.
	e, err := New(Attributes{"foo": "bar"})
	assert.Nil(t, err)
	assert.NotEmpty(t, e.Attributes())
	assert.Contains(t, e.Attributes(), "foo")
	assert.Len(t, e.Attributes(), 1)
}

func TestOptionSigner(t *testing.T) {
	e := &Entity{}
	o := Signer(mockSigner{})
	err := o(e)

	assert.Nil(t, err)
	assert.NotEmpty(t, e.signers)
	assert.Len(t, e.signers, 1)
}

func TestOptionNilSigner(t *testing.T) {
	e := &Entity{}
	o := Signer(nil)
	err := o(e)

	assert.NotNil(t, err)
}

func TestOptionID(t *testing.T) {
	e := &Entity{}
	o := ID(&did.DID{})
	err := o(e)

	assert.Nil(t, err)
	assert.NotNil(t, e.id)
}

func TestOptionNilID(t *testing.T) {
	e := &Entity{}
	o := ID(nil)
	err := o(e)

	assert.NotNil(t, err)
}

var te = Entity{
	attributes: map[string]interface{}{"foo": "bar"},
	id:         &did.DID{},
	signers:    []ockam.Signer{mockSigner{}},
}

func TestEntityID(t *testing.T) {
	id := te.ID()

	assert.NotNil(t, id)
	assert.IsType(t, &did.DID{}, id)
}

func TestEntityAttributes(t *testing.T) {
	a := te.Attributes()

	assert.NotEmpty(t, a)
	assert.Contains(t, a, "foo")
}

func TestEntitySigners(t *testing.T) {
	s := te.Signers()

	assert.NotEmpty(t, s)
}

func TestEntityPuplicKeys(t *testing.T) {
	p := te.PublicKeys()

	assert.NotEmpty(t, p)
}
