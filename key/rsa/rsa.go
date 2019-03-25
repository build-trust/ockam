package rsa

import (
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"crypto/sha512"
	"crypto/x509"
	"encoding/hex"
	"encoding/json"
	"hash"
	"time"

	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/entity"
	"github.com/piprate/json-gold/ld"
	"github.com/pkg/errors"
)

// RSA is
type RSA struct {
	public  *publicKey
	private *rsa.PrivateKey
	hasher  hash.Hash
	bits    int
}

// Option is
type Option func(*RSA)

// New returns
func New(options ...Option) (*RSA, error) {
	s := &RSA{bits: 2048}

	for _, option := range options {
		option(s)
	}

	if s.public == nil && s.private == nil {
		private, err := rsa.GenerateKey(rand.Reader, s.bits)
		if err != nil {
			return nil, errors.WithStack(err)
		}

		s.public = &publicKey{rsaPublic: &private.PublicKey}
		s.private = private
	}

	if s.hasher == nil {
		s.hasher = sha512.New()
	}

	return s, nil
}

// PublicKey is
func PublicKey(public []byte) Option {
	return func(s *RSA) {
		public, _ := x509.ParsePKCS1PublicKey(public)
		s.public = &publicKey{rsaPublic: public}
	}
}

// PrivateKey is
func PrivateKey(private []byte) Option {
	return func(s *RSA) {
		p, _ := x509.ParsePKCS1PrivateKey(private)
		s.private = p
	}
}

// PublicKey is
func (k *RSA) PublicKey() ockam.PublicKey {
	return k.public
}

// PrivateKey is
func (k *RSA) PrivateKey() []byte {
	return x509.MarshalPKCS1PrivateKey(k.private)
}

// Sign is
func (k *RSA) Sign(c ockam.Claim) error {
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

	hashed := sha256.Sum256(toSign)
	signature, err := rsa.SignPKCS1v15(rand.Reader, k.private, crypto.SHA256, hashed[:])
	if err != nil {
		return errors.WithStack(err)
	}

	s := &Signature{
		t:              "RSASignature2018",
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
func (k *RSA) SignatureType() string {
	return ""
}

type publicKey struct {
	label string
	owner ockam.Entity

	rsaPublic *rsa.PublicKey
}

// Label is
func (p *publicKey) Label() string {
	return p.label
}

// SetLabel is
func (p *publicKey) SetLabel(l string) {
	p.label = l
}

// Owner is
func (p *publicKey) Owner() ockam.Entity {
	return p.owner
}

// SetOwner is
func (p *publicKey) SetOwner(o ockam.Entity) {
	p.owner = o
}

// Type is
func (p *publicKey) Type() string {
	return "RSAVerificationKey2018"
}

// Encoding is
func (p *publicKey) Encoding() string {
	return "Hex"
}

// Value is
func (p *publicKey) Value() string {
	return hex.EncodeToString(x509.MarshalPKCS1PublicKey(p.rsaPublic))
}

// DID is
func (p *publicKey) DID() (*did.DID, error) {
	return entity.NewDID([]byte(p.Value()))
}
