// Package node implements the ockam.Node interface
package node

import (
	"fmt"
	"math/rand"
	"time"

	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/chain"

	// "github.com/ockam-network/ockam/node/types"
	"github.com/ockam-network/ockam/random"
	"github.com/pkg/errors"
)

func init() {
	// seed the random number generator when the package is initialized
	rand.Seed(time.Now().UnixNano())
}

// Peer is the interface that this local node implementation
// expects other peers to implement
type Peer interface {
	ockam.Node
	LatestCommit() *Commit
	FullCommit(height string) (*FullCommit, error)
}

// Tx is
type Tx struct {
	Proof struct {
		Proof struct {
			Aunts []interface{} `json:"aunts"`
		} `json:"Proof"`
		Data     string `json:"Data"`
		RootHash string `json:"RootHash"`
		Total    string `json:"Total"`
		Index    string `json:"Index"`
	} `json:"proof"`
	Tx       string `json:"tx"`
	TxResult struct {
		Log  string `json:"log"`
		Data string `json:"data"`
		Code string `json:"code"`
	} `json:"tx_result"`
	Height string `json:"height"`
	Hash   string `json:"hash"`
}

// Node is
type Node struct {
	id              string
	chain           ockam.Chain
	peers           []Peer
	peerDiscoverers []ockam.NodeDiscoverer
	latestCommit    *Commit
	verifier        *Verifier
}

// Option is
type Option func(*Node)

// New returns a new Node
func New(options ...Option) (*Node, error) {
	n := &Node{}

	for _, option := range options {
		option(n)
	}

	s, err := random.GenerateAlphaNumericString(31)
	if err != nil {
		return nil, err
	}
	n.id = s

	// if n.verifier == nil {
	// 	n.verifier = NewVerifier(NewMemCommitStore())
	// }

	return n, nil
}

// PeerDiscoverer is
func PeerDiscoverer(d ockam.NodeDiscoverer) Option {
	return func(n *Node) {
		n.peerDiscoverers = append(n.peerDiscoverers, d)
	}
}

// Chain is
func (n *Node) Chain() ockam.Chain {
	return n.chain
}

// Peers returns the list of peer nodes that this node is aware of
func (n *Node) Peers() []ockam.Node {
	var peers []ockam.Node
	for _, peer := range n.peers {
		peers = append(peers, peer.(ockam.Node))
	}
	return peers
}

// ID returns the identifier of the chain
func (n *Node) ID() string {
	return n.id
}

// Sync returns
func (n *Node) Sync() error {
	// if we don't know of any peers, try to discover them
	if len(n.peers) == 0 {
		err := n.discoverPeers()
		if err != nil {
			return err
		}
	}
	if len(n.peers) == 0 || n.peers[0] == nil {
		return errors.New("could not find any peers")
	}

	// randomly select one of the peers
	selectedPeer := n.peers[rand.Intn(len(n.peers))]
	err := selectedPeer.Sync()
	if err != nil {
		return errors.WithStack(err)
	}

	// get the latest commit from the se
	n.latestCommit = selectedPeer.LatestCommit()

	if n.verifier != nil {
		err := Initialize(selectedPeer, n.verifier.store)
		if err != nil {
			return errors.WithStack(err)
		}

		_, err = n.verifier.GetLastTrusted()
		if err != nil {
			fmt.Println(err)
		}
		// fmt.Println("Last Trusted: ", last.Height)
		_, err = n.verifier.Verify(n.latestCommit, selectedPeer)
		if err != nil {
			fmt.Println(err)
		}
		// fmt.Println("Is Verified: ", isVerified)
		_, err = n.verifier.GetLastTrusted()
		if err != nil {
			fmt.Println(err)
		}
		// fmt.Println("Last Trusted: ", last.Height)

	}
	if n.chain == nil {
		c, err := chain.New(chain.TrustedNode(n), chain.ID(n.latestCommit.SignedHeader.Header.ChainID))
		if err != nil {
			return errors.WithStack(err)
		}
		n.chain = c
	}

	return nil
}

// LatestBlock returns
func (n *Node) LatestBlock() ockam.Block {
	return &block{
		height: n.latestCommit.SignedHeader.Header.Height,
		hash:   n.latestCommit.SignedHeader.Commit.BlockID.Hash.String(),
	}
}

// Register is
func (n *Node) Register(e ockam.Entity) (ockam.Claim, error) {
	// pick a random peer
	p := n.peers[rand.Intn(len(n.peers))]
	// register the entity to that peer
	return p.Register(e)
}

// Submit is
func (n *Node) Submit(c ockam.Claim) error {
	// pick a random peer
	p := n.peers[rand.Intn(len(n.peers))]
	// submit the claim to that peet
	return p.Submit(c)
}

func (n *Node) FetchClaim(key string) ([]byte, ockam.Claim, error) {
	p := n.peers[rand.Intn(len(n.peers))]

	return p.FetchClaim(key)
}

func (n *Node) FetchEntity(key string) ([]byte, ockam.Entity, error) {
	p := n.peers[rand.Intn(len(n.peers))]

	return p.FetchEntity(key)
}

func (n *Node) discoverPeers() error {
	peers := n.peers

	for _, discoverer := range n.peerDiscoverers {
		discoveredPeers, err := discoverer.Discover()
		if err != nil {
			return err
		}

		for _, discoveredPeer := range discoveredPeers {
			peers = append(peers, discoveredPeer.(Peer))
		}
	}

	n.peers = peers
	return nil
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
