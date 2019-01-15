package entity

import (
	"testing"

	"github.com/ockam-network/did"
	"github.com/stretchr/testify/assert"
)

func TestNewValidEntity(t *testing.T) {
	// validate New() returns an initialized pointer to an
	// Entity{} type without throwing an error
	e, err := New()
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
	// Entity{} type when called with OptionSigner and
	// ensure functions on this type which depend on Singer
	// being present behave as expected.
	e, err := New(OptionSigner(mockSigner{}))
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
	e, err := New(OptionID(&did.DID{}))
	assert.Nil(t, err)
	assert.NotNil(t, e.ID())
}

func TestNewEntityWithOptionAttributes(t *testing.T) {
	// validate New() returns an initialized pointer to a
	// Entity{} type when called with OptionAttributes and
	// functions on this type which depend on that option behave
	// as expected.
	e, err := New(OptionAttributes(map[string]interface{}{"foo": "bar"}))
	assert.Nil(t, err)
	assert.NotEmpty(t, e.Attributes())
	assert.Contains(t, e.Attributes(), "foo")
	assert.Len(t, e.Attributes(), 1)
}
