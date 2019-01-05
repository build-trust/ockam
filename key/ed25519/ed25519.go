package ed25519

import (
	"crypto/rand"
	"crypto/sha512"
	"encoding/hex"
	"hash"
	"time"

	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/entity"
	"github.com/ockam-network/ockam/random"
	"github.com/pkg/errors"
	"golang.org/x/crypto/ed25519"
)

// Ed25519 is
type Ed25519 struct {
	public  *PublicKey
	private ed25519.PrivateKey
	hasher  hash.Hash
}

// New returns
func New() (*Ed25519, error) {
	ed25519Public, private, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	public := &PublicKey{ed25519Public: ed25519Public}

	return &Ed25519{public: public, private: private, hasher: sha512.New()}, nil
}

// PublicKey is
func (k *Ed25519) PublicKey() ockam.PublicKey {
	return k.public
}

// Sign is
func (k *Ed25519) Sign(c ockam.Claim) error {
	binary, err := c.MarshalBinary()
	if err != nil {
		return err
	}

	k.hasher.Write(binary)
	signature := ed25519.Sign(k.private, k.hasher.Sum(nil))

	nonce, err := random.GenerateBytes(20)
	if err != nil {
		return err
	}

	s := &Signature{
		t:              "Ed25519Signature2018",
		creator:        k.PublicKey(),
		created:        time.Now(),
		nonce:          nonce,
		signatureValue: signature,
	}

	c.AddSignature(s)
	return nil
}

// SignatureType is
func (k *Ed25519) SignatureType() string {
	return ""
}

// PublicKey is
type PublicKey struct {
	label string
	owner ockam.Entity

	ed25519Public ed25519.PublicKey
}

// Label is
func (p *PublicKey) Label() string {
	return p.label
}

// SetLabel is
func (p *PublicKey) SetLabel(l string) {
	p.label = l
}

// Owner is
func (p *PublicKey) Owner() ockam.Entity {
	return p.owner
}

// SetOwner is
func (p *PublicKey) SetOwner(o ockam.Entity) {
	p.owner = o
}

// Type is
func (p *PublicKey) Type() string {
	return "Ed25519VerificationKey2018"
}

// Encoding is
func (p *PublicKey) Encoding() string {
	return "Hex"
}

// Value is
func (p *PublicKey) Value() string {
	return hex.EncodeToString([]byte(p.ed25519Public))
}

// DID is
func (p *PublicKey) DID() *did.DID {
	return entity.NewDID([]byte(p.Value()))
}
