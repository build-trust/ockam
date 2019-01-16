package entity

import (
	"github.com/mr-tron/base58"
	"github.com/ockam-network/did"
	"golang.org/x/crypto/sha3"
)

// NewDID creates a new DID from a public key, the DID method is set to ockam
// and returns an error if a nil type is passed as publicKey
func NewDID(publicKey []byte) (*did.DID, error) {
	if publicKey == nil {
		return nil, ErrorNilTypeNotAllowed
	}

	//hash the public key
	hash := sha3.Sum256(publicKey)
	buf := hash[:]

	//truncate the hash to the lower 20 bytes
	buf = hash[len(buf)-20:]

	//prepend the multihash label for the hash algorithm
	//skip the varint length of the multihash, since that is fixed to 20
	// multihash.SHA3_256
	buf = append([]byte{0x16}, buf...)

	//base58 encode the above value
	id := base58.Encode(buf)

	// return a DID with Method: "ockam"
	return &did.DID{Method: "ockam", ID: id}, nil
}
