package entity

import (
	"testing"

	"github.com/ockam-network/did"
	"github.com/stretchr/testify/assert"
)

func TestOptionAttributes(t *testing.T) {
	e := &Entity{}
	o := OptionAttributes(Attributes{"foo": "bar"})
	err := o(e)

	assert.Nil(t, err)
	assert.Equal(t, "bar", e.attributes["foo"])
}

func TestOptionSignerNotNil(t *testing.T) {
	e := &Entity{}
	o := OptionSigner(nil)
	err := o(e)

	assert.NotNil(t, err)
}

func TestOptionID(t *testing.T) {
	e := &Entity{}
	o := OptionID(&did.DID{})
	err := o(e)

	assert.Nil(t, err)
}
