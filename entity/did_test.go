package entity

import (
	"testing"

	"github.com/mr-tron/base58"
	"github.com/stretchr/testify/assert"
	"golang.org/x/crypto/sha3"
)

func TestNewDID(t *testing.T) {
	testBytes := []byte("foo")

	// make a SHA from the testBytes, grab the last 20 and prefix \x16
	testHash := sha3.Sum256(testBytes)
	testTruncHash := testHash[len(testHash)-20:]
	testTruncHash = append([]byte{0x16}, testTruncHash...)

	did, err := NewDID(testBytes)

	assert.Nil(t, err)
	assert.NotEmpty(t, did.ID)
	assert.Equal(t, did.Method, "ockam")

	decodedID, err := base58.Decode(did.ID)

	assert.Nil(t, err)
	assert.Equal(t, string(testTruncHash), string(decodedID))
}

func TestNewDIDNil(t *testing.T) {
	did, err := NewDID(nil)

	assert.NotNil(t, err)
	assert.Nil(t, did)
}
