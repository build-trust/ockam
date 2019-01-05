// Package node implements the ockam.Node interface
package node

import (
	"fmt"
	"math/rand"
	"time"

	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/chain"
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
}

// Commit is
type Commit struct {
	SignedHeader struct {
		Header struct {
			ChainID     string    `json:"chain_id"`
			Height      string    `json:"height"`
			Time        time.Time `json:"time"`
			NumTxs      string    `json:"num_txs"`
			TotalTxs    string    `json:"total_txs"`
			LastBlockID struct {
				Hash  string `json:"hash"`
				Parts struct {
					Total string `json:"total"`
					Hash  string `json:"hash"`
				} `json:"parts"`
			} `json:"last_block_id"`
			LastCommitHash     string `json:"last_commit_hash"`
			DataHash           string `json:"data_hash"`
			ValidatorsHash     string `json:"validators_hash"`
			NextValidatorsHash string `json:"next_validators_hash"`
			ConsensusHash      string `json:"consensus_hash"`
			AppHash            string `json:"app_hash"`
			LastResultsHash    string `json:"last_results_hash"`
			EvidenceHash       string `json:"evidence_hash"`
			ProposerAddress    string `json:"proposer_address"`
		} `json:"header"`
		Commit struct {
			BlockID struct {
				Hash  string `json:"hash"`
				Parts struct {
					Total string `json:"total"`
					Hash  string `json:"hash"`
				} `json:"parts"`
			} `json:"block_id"`
			Precommits []Precommit `json:"precommits"`
		} `json:"commit"`
	} `json:"signed_header"`
	Canonical bool `json:"canonical"`
}

// Precommit is
type Precommit struct {
	Type      int       `json:"type"`
	Height    string    `json:"height"`
	Round     string    `json:"round"`
	Timestamp time.Time `json:"timestamp"`
	BlockID   struct {
		Hash  string `json:"hash"`
		Parts struct {
			Total string `json:"total"`
			Hash  string `json:"hash"`
		} `json:"parts"`
	} `json:"block_id"`
	ValidatorAddress string `json:"validator_address"`
	ValidatorIndex   string `json:"validator_index"`
	Signature        string `json:"signature"`
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
	chain           ockam.Chain
	peers           []Peer
	peerDiscoverers []ockam.NodeDiscoverer
	latestCommit    *Commit
}

// Option is
type Option func(*Node)

// New returns a new Node
func New(options ...Option) (*Node, error) {
	n := &Node{}

	for _, option := range options {
		option(n)
	}

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

type block struct {
	height string
	hash   string
}

// LatestBlock returns
func (n *Node) LatestBlock() ockam.Block {
	return &block{
		height: n.latestCommit.SignedHeader.Header.Height,
		hash:   n.latestCommit.SignedHeader.Commit.BlockID.Hash,
	}
}

// Height is
func (b *block) Height() string {
	return b.height
}

// Hash is
func (b *block) Hash() string {
	return b.hash
}

// Peers returns the list of peer nodes that this node is aware of
func (n *Node) Peers() []ockam.Node {
	var peers []ockam.Node
	for _, peer := range n.peers {
		peers = append(peers, peer.(ockam.Node))
	}
	return peers
}

// Submit is
func (n *Node) Submit(b []byte) ([]byte, error) {
	p := n.peers[rand.Intn(len(n.peers))]
	fmt.Println(p)
	return p.Submit(b)
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

	if n.chain == nil {
		c, err := chain.New(chain.TrustedNode(n), chain.ID(n.latestCommit.SignedHeader.Header.ChainID))
		if err != nil {
			return errors.WithStack(err)
		}
		n.chain = c
	}

	return nil
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
