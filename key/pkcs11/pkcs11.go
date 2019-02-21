// +build cgo
// +build linux darwin

package pkcs11

import (
	"crypto"
	"crypto/rand"
	"crypto/sha512"
	"encoding/json"
	"time"

	"github.com/letsencrypt/pkcs11key"
	"github.com/ockam-network/ockam"
	"github.com/piprate/json-gold/ld"
	"github.com/pkg/errors"
)

// PKCS11 extends letsencrypt/pkcs11key.Key in order
// to implement a crypto.Signer interface using a key
// stored in a PKCS#11 hardware token (such as a yubikey)
type PKCS11 struct {
	signer     *pkcs11key.Key
	publicKey  crypto.PublicKey
	modulePath string
	pin        string
	tokenLabel string
}

// New returns an initialized PKCS11 type with a default module
// path pointed to Yubikey's PKCS11 so file
func New(options ...Option) (*PKCS11, error) {
	pkcs11 := &PKCS11{
		modulePath: "/usr/local/lib/yubihsm_pkcs11.so",
	}

	for _, o := range options {
		if err := o(pkcs11); err != nil {
			return nil, err
		}
	}

	signer, err := pkcs11key.New(pkcs11.modulePath, pkcs11.tokenLabel, pkcs11.pin, pkcs11.publicKey)
	if err != nil {
		return nil, err
	}

	pkcs11.signer = signer

	return pkcs11, nil
}

func (k *PKCS11) PublicKey() ockam.PublicKey {
	// TODO(malnick) need to determine values for label and owner
	return &PKCS11PublicKey{publicKey: k.publicKey}
}

// SignatureType is not implemented
func (k *PKCS11) SignatureType() string {
	return ""
}

// Sign generates a signature for a ockam.Claim{} using a pkcs11
// interface leveraging a private key on an arbitrary HSM
func (k *PKCS11) Sign(c ockam.Claim) error {
	toSign, err := claimLDJSONBytes(c)
	if err != nil {
		return err
	}

	hashed := sha512.Sum512(toSign)
	rand := rand.Reader

	signature, err := k.signer.Sign(rand, hashed[:], crypto.SHA512)
	if err != nil {
		return err
	}

	s := &Signature{
		t:              "PKCS11Signature2018",
		creator:        c.Issuer().ID().String() + k.tokenLabel,
		created:        time.Now().UTC().Format(time.RFC3339),
		nonce:          c.Nonce(),
		signatureValue: signature,
		signedValue:    toSign,
	}

	c.AddSignature(s)

	return nil
}

func claimLDJSONBytes(c ockam.Claim) ([]byte, error) {
	claimJSON, err := c.MarshalJSON()
	if err != nil {
		return nil, err
	}

	var claimMap map[string]interface{}
	err = json.Unmarshal(claimJSON, &claimMap)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	delete(claimMap, "signatures")

	proc := ld.NewJsonLdProcessor()
	options := ld.NewJsonLdOptions("")
	options.Format = "application/nquads"
	options.Algorithm = "URDNA2015"

	canonicalized, err := proc.Normalize(claimMap, options)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	return []byte(canonicalized.(string)), nil

}
