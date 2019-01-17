package chain

import (
	"testing"

	"github.com/ockam-network/ockam/claim"
	"github.com/ockam-network/ockam/entity"
	"github.com/stretchr/testify/assert"
)

func TestNewDefaultChain(t *testing.T) {
	c, err := New()

	assert.Nil(t, err)
	assert.NotNil(t, c)
}

func TestNewChainWithIDOption(t *testing.T) {
	c, err := New(ID("foo"))

	assert.Nil(t, err)
	assert.NotNil(t, c)
	assert.Equal(t, "foo", c.id)
}

func TestNewChainWithTrustedNodeOption(t *testing.T) {
	c, err := New(TrustedNode(mockNode{}))

	assert.Nil(t, err)
	assert.NotNil(t, c)
	assert.NotNil(t, c.trustedNode)
}

func TestIDOption(t *testing.T) {
	c := &Chain{}
	o := ID("foo")
	o(c)

	assert.Equal(t, "foo", c.id)
}

func TestTrustedNodeOption(t *testing.T) {
	c := &Chain{}
	o := TrustedNode(mockNode{})
	o(c)

	assert.NotNil(t, c.trustedNode)
}

func TestChainID(t *testing.T) {
	c := &Chain{id: "foo"}

	assert.Equal(t, c.ID(), "foo")
}

// the following are limited tests due to the limited
// mocking done on mockNode{}
func TestChainSync(t *testing.T) {
	c := &Chain{trustedNode: mockNode{}}
	err := c.Sync()
	assert.Nil(t, err)
}

func TestChainLatestBlock(t *testing.T) {
	c := &Chain{trustedNode: mockNode{}}
	err := c.LatestBlock()
	assert.Nil(t, err)
}

func TestChainRegister(t *testing.T) {
	c := &Chain{trustedNode: mockNode{}}
	claim, err := c.Register(&entity.Entity{})
	assert.Nil(t, err)
	assert.Nil(t, claim)
}

func TestChainSubmit(t *testing.T) {
	c := &Chain{trustedNode: mockNode{}}
	err := c.Submit(&claim.Claim{})
	assert.Nil(t, err)
}
