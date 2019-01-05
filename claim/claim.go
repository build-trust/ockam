package claim

import (
	"encoding/hex"
	"encoding/json"
	"strings"

	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/random"
)

// Data is
type Data map[string]interface{}

// Claim is
type Claim struct {
	id      string
	data    Data
	issuer  ockam.Entity
	subject ockam.Entity
}

// Option is
type Option func(*Claim)

// New returns a new Claim
func New(data Data, options ...Option) (*Claim, error) {
	c := &Claim{data: data}

	for _, option := range options {
		option(c)
	}

	s, err := random.GenerateAlphaNumericString(31)
	if err != nil {
		return nil, err
	}

	if c.issuer != nil {
		c.id = c.issuer.ID().String() + "/claim/" + s
	} else {
		c.id = s
	}

	return c, nil
}

// ID is
func ID(id string) Option {
	return func(c *Claim) {
		c.id = id
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

// SetID sets the claim's ID
func (c *Claim) SetID(id string) {
	c.id = id
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
func (c *Claim) AddSignature(ockam.Signature) {
}

// MarshalBinary is
func (c Claim) MarshalBinary() ([]byte, error) {
	b, err := json.Marshal(c.data)
	if err != nil {
		return nil, err
	}
	s := hex.EncodeToString(b)
	return []byte(c.id + "=" + s), nil
}
