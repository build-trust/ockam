package pkcs11

import (
	"crypto"
	"encoding/hex"

	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/entity"
)

// PKCS11PublicKey is an implementation of an ockam.PublicKey
// interface for the pkcs11 package
type PKCS11PublicKey struct {
	label     string
	owner     ockam.Entity
	publicKey crypto.PublicKey
}

// Label is a getting for the private lable attribute
func (p *PKCS11PublicKey) Label() string {
	return p.label
}

// SetLabel is a setter for the private label attribute
func (p *PKCS11PublicKey) SetLabel(l string) {
	p.label = l
}

// Owner is is a getter for the private owner attribute
func (p *PKCS11PublicKey) Owner() ockam.Entity {
	return p.owner
}

// SetOwner is a setter for the private owner attribute
func (p *PKCS11PublicKey) SetOwner(o ockam.Entity) {
	p.owner = o
}

// Type returns the public key type
func (p *PKCS11PublicKey) Type() string {
	return "RSA"
}

// Encoding returns the key encoding type
func (p *PKCS11PublicKey) Encoding() string {
	return "Hex"
}

// Value is is the hex value of the publicKey attribute
func (p *PKCS11PublicKey) Value() string {
	return hex.EncodeToString(p.publicKey.([]byte))
}

// DID returns a DID for the public key object
func (p PKCS11PublicKey) DID() (*did.DID, error) {
	return entity.NewDID([]byte(p.Value()))
}
