package ockam

import (
	"encoding"
	"time"

	"github.com/ockam-network/did"
)

// Block represents a block in the Ockam Blockchain
type Block interface {
	Height() string
	Hash() string
}

// Chain represents a chain of blocks that is maintained by a network of nodes
type Chain interface {
	ID() string
	Sync() error
	LatestBlock() Block

	Register(Entity) (Claim, error)
	Submit(Claim) error
}

// Node represents a node connected to a network of other peer nodes
type Node interface {
	Sync() error
	Peers() []Node
	Chain() Chain
	LatestBlock() Block
	Submit(b []byte) ([]byte, error)
}

// NodeDiscoverer provides the means to discover other nodes on in a network
type NodeDiscoverer interface {
	Discover() ([]Node, error)
}

// PublicKey is
type PublicKey interface {
	Label() string
	SetLabel(string)

	Owner() Entity
	SetOwner(Entity)

	Type() string
	Encoding() string
	Value() string

	DID() *did.DID
}

// Signature is
// https://w3c-dvcg.github.io/ld-signatures/#linked-data-signature-overview
type Signature interface {
	// Type
	// https://w3c-ccg.github.io/ld-cryptosuite-registry/#ed25519signature2018
	Type() string
	Creator() PublicKey
	Created() time.Time
	Domain() string

	// Nonce
	// https://web-payments.org/vocabs/security#nonce
	Nonce() []byte

	// SignatureValue
	// https://web-payments.org/vocabs/security#signatureValue
	SignatureValue() []byte
}

// Claim is
type Claim interface {
	ID() string

	Issuer() Entity
	SetIssuer(Entity)

	Subject() Entity
	SetSubject(Entity)

	Data() map[string]interface{}
	SetData(map[string]interface{})

	Signatures() []Signature
	AddSignature(Signature)

	encoding.BinaryMarshaler
}

// Entity represents and Ockam entity
type Entity interface {
	ID() *did.DID
	PublicKeys() []PublicKey
	Signers() []Signer
}

// Signer is
type Signer interface {
	PublicKey() PublicKey
	Sign(Claim) error
	SignatureType() string
}

// Fields is
type Fields map[string]interface{}

// Logger is an interface for Logging.
type Logger interface {
	Error(format string, v ...interface{})
	Warn(format string, v ...interface{})
	Notice(format string, v ...interface{})
	Info(format string, v ...interface{})
	Debug(format string, v ...interface{})
	WithFields(fields Fields) Logger
}

// Version returns the current version of Ockam
func Version() string {
	version := "0.2.0-develop"
	return version
}
