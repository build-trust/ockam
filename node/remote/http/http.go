// Package http implements ockam.Node interface to enable interaction
// with a remote node that can be commiunicated with over http.
package http

import (
	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/chain"
	"github.com/ockam-network/ockam/node"
	"github.com/pkg/errors"
)

// Node represent a remote node
type Node struct {
	ip           string
	port         int
	chain        ockam.Chain
	peers        []ockam.Node
	latestCommit *node.Commit
}

// Option is
type Option func(*Node)

// NewNode returns
func NewNode(options ...Option) (*Node, error) {
	n := &Node{}

	for _, option := range options {
		option(n)
	}

	return n, nil
}

// IP is
func IP(ip string) Option {
	return func(n *Node) {
		n.ip = ip
	}
}

// Port is
func Port(port int) Option {
	return func(n *Node) {
		n.port = port
	}
}

// Sync returns
func (n *Node) Sync() error {

	latest, err := n.getLatestCommit()
	if err != nil {
		return errors.WithStack(err)
	}

	n.latestCommit = latest

	if n.chain == nil {
		c, err := chain.New(chain.TrustedNode(n))
		if err != nil {
			return errors.WithStack(err)
		}
		n.chain = c
	}

	return nil
}

// Chain is
func (n *Node) Chain() ockam.Chain {
	return n.chain
}

// LatestCommit returns
func (n *Node) LatestCommit() *node.Commit {
	return n.latestCommit
}

// Submit is
func (n *Node) Submit(b []byte) ([]byte, error) {
	return n.BroadcastTxSync(string(b[:]))
}

type block struct {
	height string
	hash   string
}

// Height is
func (b *block) Height() string {
	return b.height
}

// Hash is
func (b *block) Hash() string {
	return b.hash
}

// LatestBlock returns
func (n *Node) LatestBlock() ockam.Block {
	return &block{
		height: n.latestCommit.SignedHeader.Header.Height,
		hash:   n.latestCommit.SignedHeader.Commit.BlockID.Hash,
	}
}

// Peers is
func (n *Node) Peers() []ockam.Node {
	return n.peers
}

func (n *Node) getLatestCommit() (*node.Commit, error) {
	r := new(CommitResponse)
	err := n.Call("/commit", &r)
	if err != nil {
		return nil, errors.WithStack(err)
	}
	return &r.Result, nil
}
