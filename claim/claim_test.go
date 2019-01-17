package claim

import (
	"testing"

	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/entity"
	"github.com/stretchr/testify/assert"
)

func TestNewDefaultClaim(t *testing.T) {
	// validate that New() returns an initialized Claim{} with a pre-generated
	// Claim.nonce with a length of 10, a Claim.id equal to the Claim.nonce when called with an
	// empty Data{} and no functional options
	c, err := New(Data{})

	assert.Nil(t, err)
	assert.NotNil(t, c)
	assert.NotEmpty(t, c.nonce)
	assert.NotEmpty(t, c.id)
	assert.Equal(t, c.id, c.nonce)
	assert.Len(t, c.nonce, 10)
	assert.Len(t, c.id, 10)
}

func TestNewClaimWithNilData(t *testing.T) {
	// validate that New() returns an initialized Claim{} with a pre-generated
	// Claim.nonce with a length of 10, and a Claim.id equal to the issuer ID
	// when called with an nil Data{} and no options
	_, err := New(nil)

	assert.NotNil(t, err)
}

func TestNewClaimWithIssuer(t *testing.T) {
	// validate that New() returns an initialized Claim{} with a pre-generated
	// Claim.nonce with a length of 10, and a Claim.id equal to the issuer ID
	// when called with an empty Data{} and the Issuer() option
	e, _ := defaultEntity(t)
	c, err := New(Data{}, Issuer(e))

	assert.Nil(t, err)
	assert.NotNil(t, c)
	assert.NotEmpty(t, c.nonce)
	assert.Len(t, c.nonce, 10)
	assert.NotEmpty(t, c.id)
	assert.Contains(t, c.id, "/claim/")
	assert.Contains(t, c.id, c.nonce)
	assert.Contains(t, c.id, "did:ockam:")
}

func TestOptionID(t *testing.T) {
	c := &Claim{}
	o := ID("foo")
	o(c)

	assert.NotEmpty(t, c.id)
	assert.Equal(t, c.id, "foo")
}

func TestOptionType(t *testing.T) {
	c := &Claim{}
	o := Type("foo")
	o(c)

	assert.NotEmpty(t, c._type)
	assert.Equal(t, c._type, "foo")
}

func TestOptionIssuer(t *testing.T) {
	e, did := defaultEntity(t)
	c := &Claim{}
	o := Issuer(e)
	o(c)

	assert.NotNil(t, c.issuer)
	assert.Equal(t, did.ID, c.issuer.ID().ID)
}

func TestOptionSubject(t *testing.T) {
	e, did := defaultEntity(t)
	c := &Claim{}
	o := Subject(e)
	o(c)

	assert.NotNil(t, c.subject)
	assert.Equal(t, did.ID, c.subject.ID().ID)
}

func TestClaimID(t *testing.T) {
	c := &Claim{id: "foo"}

	assert.Equal(t, "foo", c.ID())
}

func TestClaimNonce(t *testing.T) {
	c := &Claim{nonce: "foo"}

	assert.Equal(t, "foo", c.Nonce())
}

func TestClaimType(t *testing.T) {
	c := &Claim{_type: "foo"}

	assert.Equal(t, "foo", c.Type())
}

func TestClaimSetType(t *testing.T) {
	c := &Claim{}
	c.SetType("foo")

	assert.Equal(t, "foo", c.Type())
}

func TestClaimIssuer(t *testing.T) {
	e, did := defaultEntity(t)
	c := &Claim{issuer: e}

	assert.NotNil(t, c.issuer)
	assert.Equal(t, did.ID, c.issuer.ID().ID)
}

func TestClaimSetIssuer(t *testing.T) {
	e, did := defaultEntity(t)
	c := &Claim{}
	c.SetIssuer(e)

	assert.NotNil(t, c.issuer)
	assert.NotEmpty(t, c.id)
	assert.Contains(t, c.id, "/claim/")
	assert.Equal(t, did.ID, c.issuer.ID().ID)
}

func TestClaimSubject(t *testing.T) {
	e, did := defaultEntity(t)
	c := &Claim{subject: e}
	s := c.Subject()

	assert.NotNil(t, s)
	assert.Equal(t, did.ID, s.ID().ID)
}

func TestClaimSetSubject(t *testing.T) {
	e, did := defaultEntity(t)
	c := &Claim{}
	c.SetSubject(e)

	assert.NotNil(t, c.subject)
	assert.Equal(t, did.ID, c.subject.ID().ID)
}

func TestClaimData(t *testing.T) {
	c := Claim{data: map[string]interface{}{"foo": "bar"}}
	d := c.Data()

	assert.NotEmpty(t, d)
	assert.Len(t, d, 1)
	assert.Contains(t, d, "foo")
}

func TestClaimSetData(t *testing.T) {
	c := Claim{}
	c.SetData(map[string]interface{}{"foo": "bar"})

	assert.NotEmpty(t, c.data)
	assert.Len(t, c.data, 1)
	assert.Contains(t, c.data, "foo")
}

func TestClaimSignatures(t *testing.T) {
	// Claim.Signatures() is not yet implemented
	c := Claim{}
	s := c.Signatures()

	assert.Nil(t, s)
}

func TestClaimAddSignature(t *testing.T) {
	c := Claim{signatures: []ockam.Signature{}}
	c.AddSignature(mockSignature{})

	assert.Len(t, c.signatures, 1)
}

func TestClaimMarshalJSON(t *testing.T) {
	e, _ := defaultEntity(t)
	c := Claim{
		id:         "foo",
		data:       map[string]interface{}{},
		nonce:      "foo",
		subject:    e,
		issuer:     e,
		signatures: []ockam.Signature{mockSignature{}},
	}

	bytes, err := c.MarshalJSON()

	assert.Nil(t, err)
	assert.NotEmpty(t, bytes)
}

func defaultEntity(t *testing.T) (ockam.Entity, *did.DID) {
	testBytes := []byte("foo")

	did, err := entity.NewDID(testBytes)
	assert.Nil(t, err)

	e, err := entity.New(entity.Attributes{}, entity.ID(did))
	assert.Nil(t, err)

	return e, did
}
