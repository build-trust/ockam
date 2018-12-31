package main

import (
	"bytes"
	"crypto/sha256"
	"hash"
	"io"

	"github.com/pkg/errors"
	amino "github.com/tendermint/go-amino"
)

// Verify that the SimpleProof proves the root hash.
// Check sp.Index/sp.Total manually if needed
func (sp *SimpleProof) Verify(rootHash []byte, leafHash []byte) error {
	if sp.Total < 0 {
		return errors.New("Proof total must be positive")
	}
	if sp.Index < 0 {
		return errors.New("Proof index cannot be negative")
	}
	if !bytes.Equal(sp.LeafHash, leafHash) {
		return errors.Errorf("invalid leaf hash: wanted %X got %X", leafHash, sp.LeafHash)
	}
	computedHash := sp.ComputeRootHash()
	if !bytes.Equal(computedHash, rootHash) {
		return errors.Errorf("invalid root hash: wanted %X got %X", rootHash, computedHash)
	}
	return nil
}

// Compute the root hash given a leaf hash.  Does not verify the result.
func (sp *SimpleProof) ComputeRootHash() []byte {
	return computeHashFromAunts(
		sp.Index,
		sp.Total,
		sp.LeafHash,
		sp.Aunts,
	)
}

// Use the leafHash and innerHashes to get the root merkle hash.
// If the length of the innerHashes slice isn't exactly correct, the result is nil.
// Recursive impl.
func computeHashFromAunts(index int, total int, leafHash []byte, innerHashes [][]byte) []byte {
	if index >= total || index < 0 || total <= 0 {
		return nil
	}
	switch total {
	case 0:
		panic("Cannot call computeHashFromAunts() with 0 total")
	case 1:
		if len(innerHashes) != 0 {
			return nil
		}
		return leafHash
	default:
		if len(innerHashes) == 0 {
			return nil
		}
		numLeft := (total + 1) / 2
		if index < numLeft {
			leftHash := computeHashFromAunts(index, numLeft, leafHash, innerHashes[:len(innerHashes)-1])
			if leftHash == nil {
				return nil
			}
			return simpleHashFromTwoHashes(leftHash, innerHashes[len(innerHashes)-1])
		}
		rightHash := computeHashFromAunts(index-numLeft, total-numLeft, leafHash, innerHashes[:len(innerHashes)-1])
		if rightHash == nil {
			return nil
		}
		return simpleHashFromTwoHashes(innerHashes[len(innerHashes)-1], rightHash)
	}
}

// simpleHashFromTwoHashes is the basic operation of the Merkle tree: Hash(left | right).
func simpleHashFromTwoHashes(left, right []byte) []byte {
	var hasher = NewHasher()
	err := encodeByteSlice(hasher, left)
	if err != nil {
		panic(err)
	}
	err = encodeByteSlice(hasher, right)
	if err != nil {
		panic(err)
	}
	return hasher.Sum(nil)
}

// New returns a new hash.Hash.
func NewHasher() hash.Hash {
	return sha256.New()
}

// Uvarint length prefixed byteslice
func encodeByteSlice(w io.Writer, bz []byte) (err error) {
	return amino.EncodeByteSlice(w, bz)
}
