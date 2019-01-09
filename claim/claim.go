package claim

import (
	"encoding/json"
	"strings"
	"time"

	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/random"
)

// Data is
type Data map[string]interface{}

// Claim is
type Claim struct {
	nonce      string
	id         string
	data       Data
	_type      string
	issuer     ockam.Entity
	subject    ockam.Entity
	signatures []ockam.Signature
}

// Option is
type Option func(*Claim)

// New returns a new Claim
func New(data Data, options ...Option) (*Claim, error) {
	c := &Claim{data: data}

	for _, option := range options {
		option(c)
	}

	nonce, err := random.GenerateAlphaNumericString(10)
	if err != nil {
		return nil, err
	}
	c.nonce = nonce

	if c.issuer != nil {
		c.id = c.issuer.ID().String() + "/claim/" + nonce
	} else {
		c.id = nonce
	}

	return c, nil
}

// ID is
func ID(id string) Option {
	return func(c *Claim) {
		c.id = id
	}
}

// Type is
func Type(t string) Option {
	return func(c *Claim) {
		c._type = t
	}
}

// Issuer is
func Issuer(e ockam.Entity) Option {
	return func(c *Claim) {
		c.issuer = e
	}
}

// Subject is
func Subject(e ockam.Entity) Option {
	return func(c *Claim) {
		c.subject = e
	}
}

// ID returns the claim's ID
func (c *Claim) ID() string {
	return c.id
}

// Nonce returns the claim's Nonce
func (c *Claim) Nonce() string {
	return c.nonce
}

// Type returns the claim's Type
func (c *Claim) Type() string {
	return c._type
}

// SetType sets the claim's Type
func (c *Claim) SetType(t string) {
	c._type = t
}

// Issuer returns the claim's Issuer entity
func (c *Claim) Issuer() ockam.Entity {
	return c.issuer
}

// SetIssuer sets the claim's Issuer entity
func (c *Claim) SetIssuer(issuer ockam.Entity) {
	c.issuer = issuer

	parts := strings.Split(c.id, "/claim/")
	if len(parts) == 2 {
		c.id = c.issuer.ID().String() + "/claim/" + parts[1]
	} else {
		c.id = c.issuer.ID().String() + "/claim/" + c.id
	}
}

// Subject returns the claim's Subject entity
func (c *Claim) Subject() ockam.Entity {
	return c.subject
}

// SetSubject sets the claim's Subject entity
func (c *Claim) SetSubject(subject ockam.Entity) {
	c.subject = subject
}

// Data returns the claimed data
func (c *Claim) Data() map[string]interface{} {
	return c.data
}

// SetData sets the claimed data
func (c *Claim) SetData(data map[string]interface{}) {
	c.data = data
}

// Signatures is
func (c *Claim) Signatures() []ockam.Signature {
	return nil
}

// AddSignature is
func (c *Claim) AddSignature(s ockam.Signature) {
	c.signatures = append(c.signatures, s)
}

// MarshalJSON is
func (c *Claim) MarshalJSON() ([]byte, error) {
	type j struct {
		Context    []string                 `json:"@context"`
		ID         string                   `json:"id"`
		Type       []string                 `json:"type"`
		Issuer     string                   `json:"issuer"`
		Issued     string                   `json:"issued"`
		Claim      map[string]interface{}   `json:"claim"`
		Signatures []map[string]interface{} `json:"signatures,omitempty"`
	}

	vc := &j{
		Context: []string{"https://w3id.org/identity/v1", "https://w3id.org/security/v1"},
		ID:      c.ID(),
		Type:    []string{c.Type()},
		Issuer:  c.Issuer().ID().String(),
		Issued:  time.Now().UTC().Format("2006-01-02"),
	}

	vc.Claim = c.Data()
	vc.Claim["id"] = c.Subject().ID().String()

	for _, s := range c.signatures {
		sj := map[string]interface{}{
			"type":           s.Type(),
			"created":        s.Created(),
			"creator":        s.Creator(),
			"domain":         s.Domain(),
			"nonce":          s.Nonce(),
			"signatureValue": s.SignatureValue(),
		}
		vc.Signatures = append(vc.Signatures, sj)
	}

	return json.Marshal(vc)
}
