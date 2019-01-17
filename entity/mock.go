package entity

import (
	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
)

// these mocks exist in order to build unit tests while not
// introducing import cycles between this package and others
// which use it

type mockPublicKey struct{}

func (k mockPublicKey) Label() string           { return "" }
func (k mockPublicKey) SetLabel(s string)       { return }
func (k mockPublicKey) Owner() ockam.Entity     { return nil }
func (k mockPublicKey) SetOwner(e ockam.Entity) { return }
func (k mockPublicKey) Type() string            { return "" }
func (k mockPublicKey) Encoding() string        { return "" }
func (k mockPublicKey) Value() string           { return "" }
func (k mockPublicKey) DID() (*did.DID, error)  { return &did.DID{}, nil }

type mockSigner struct{}

func (k mockSigner) PublicKey() ockam.PublicKey { return mockPublicKey{} }
func (k mockSigner) Sign(c ockam.Claim) error   { return nil }
func (k mockSigner) SignatureType() string      { return "" }
