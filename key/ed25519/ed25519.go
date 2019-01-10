package ed25519

import (
	"crypto/rand"
	"crypto/sha512"
	"encoding/hex"
	"encoding/json"
	"hash"
	"time"

	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/entity"
	"github.com/piprate/json-gold/ld"
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

// PrivateKey is
func (k *Ed25519) PrivateKey() []byte {
	return k.private
}

// Sign is
func (k *Ed25519) Sign(c ockam.Claim) error {
	claimJSON, err := c.MarshalJSON()
	if err != nil {
		return err
	}

	var claimMap map[string]interface{}
	err = json.Unmarshal(claimJSON, &claimMap)
	if err != nil {
		return errors.WithStack(err)
	}

	delete(claimMap, "signatures")

	proc := ld.NewJsonLdProcessor()
	options := ld.NewJsonLdOptions("")
	options.Format = "application/nquads"
	options.Algorithm = "URDNA2015"

	canonicalized, err := proc.Normalize(claimMap, options)
	if err != nil {
		return errors.WithStack(err)
	}

	toSign := []byte(canonicalized.(string))

	k.hasher.Write(toSign)
	signature := ed25519.Sign(k.private, k.hasher.Sum(nil))

	s := &Signature{
		t:              "Ed25519Signature2018",
		creator:        c.Issuer().ID().String() + k.PublicKey().Label(),
		created:        time.Now().UTC().Format(time.RFC3339),
		nonce:          c.Nonce(),
		signatureValue: signature,
		signedValue:    toSign,
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
