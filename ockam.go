package ockam

// Block represents a block in the chain
type Block interface {
	Height() string
	Hash() string
}

// Chain represents a chain of blocks that is maintained by a network of nodes
type Chain interface {
	ID() string
	Sync() error
	LatestBlock() Block
}

// Node represents a node connected to a network of other peer nodes
type Node interface {
	Sync() error
	Peers() []Node
	Chain() Chain
	LatestBlock() Block
}

// NodeDiscoverer provides the means to discover other nodes on in a network
type NodeDiscoverer interface {
	Discover() ([]Node, error)
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
