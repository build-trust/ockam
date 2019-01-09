package chain

import "github.com/ockam-network/ockam"

// Chain represents a local instace of the ockam blockchain that is maintained by
// a network of ockam nodes.
//
// It implements the ockam.Chain interface and communicates with the ockam network
// using a trusted local (typically) ockam.Node instance.
type Chain struct {
	id          string
	trustedNode ockam.Node
}

// Option is used to provide optional arguments to the New function which creates a Chain instace.
type Option func(*Chain)

// New returns a new Chain instace
func New(options ...Option) (*Chain, error) {
	// create the struct
	c := &Chain{}

	// apply options to the new struct
	for _, option := range options {
		option(c)
	}

	// return the new Chain struct
	return c, nil
}

// ID is used to optionally set the id of a new Chain struct that is created
func ID(id string) Option {
	return func(c *Chain) {
		c.id = id
	}
}

// TrustedNode is used to optionally set the trustedNode of a new Chain
func TrustedNode(n ockam.Node) Option {
	return func(c *Chain) {
		c.trustedNode = n
	}
}

// ID returns the identifier of the chain
func (c *Chain) ID() string {
	return c.id
}

// Sync causes the Chain's trusted node to synchronize its state with its network
// Which includes fetching the latest block.
func (c *Chain) Sync() error {
	return c.trustedNode.Sync()
}

// LatestBlock returns the latest block in the chain
func (c *Chain) LatestBlock() ockam.Block {
	return c.trustedNode.LatestBlock()
}

// Register is
func (c *Chain) Register(e ockam.Entity) (ockam.Claim, error) {
	return c.trustedNode.Register(e)
}

// Submit is
func (c *Chain) Submit(claim ockam.Claim) error {
	return c.trustedNode.Submit(claim)
}
