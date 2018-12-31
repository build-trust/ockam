package types

import "golang.org/x/crypto/ed25519"

type PubKey struct {
	Type  string `json:"type"`
	Value []byte `json:"value"`
}

func (pc *PubKey) VerifyBytes(msg []byte, sig []byte) bool {
	isVerified := ed25519.Verify(pc.Value[:], msg, sig)

	return isVerified
}
