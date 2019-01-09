// Package http implements ockam.Node interface to enable interaction
// with a remote node that can be commiunicated with over http.
package http

import (
	"encoding/hex"
	"encoding/json"

	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/chain"
	"github.com/ockam-network/ockam/claim"
	"github.com/ockam-network/ockam/node"
	"github.com/pkg/errors"
)

// Node represent a remote node
type Node struct {
	id           string
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

// ID is
func (n *Node) ID() string {
	return n.id
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

// Register is
func (n *Node) Register(e ockam.Entity) (ockam.Claim, error) {
	cl, err := claim.New(
		claim.Data{"id": e.ID().String()},
		claim.Issuer(e),
		claim.Subject(e),
	)
	if err != nil {
		return nil, err
	}

	err = n.Submit(cl)
	if err != nil {
		return nil, err
	}

	return cl, nil
}

// Submit is
func (n *Node) Submit(cl ockam.Claim) error {
	b, err := json.Marshal(cl.Data())
	if err != nil {
		return err
	}

	s := hex.EncodeToString(b) // base?
	_, err = n.BroadcastTxSync(cl.ID() + "=" + s)
	if err != nil {
		return err
	}

	return err
}
