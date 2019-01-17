// Package http implements ockam.Node interface to enable interaction
// with a remote node that can be commiunicated with over http.
package http

import (
	"encoding/hex"
	"strings"

	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/chain"
	"github.com/ockam-network/ockam/claim"
	"github.com/ockam-network/ockam/node/types"
	"github.com/pkg/errors"
)

// Node represent a remote node
type Node struct {
	id           string
	ip           string
	port         int
	chain        ockam.Chain
	peers        []ockam.Node
	latestCommit *types.Commit
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
func (n *Node) LatestCommit() *types.Commit {
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
		hash:   n.latestCommit.SignedHeader.Commit.BlockID.Hash.String(),
	}
}

// Peers is
func (n *Node) Peers() []ockam.Node {
	return n.peers
}

func (n *Node) getLatestCommit() (*types.Commit, error) {
	r := new(CommitResponse)
	err := n.Call("/commit", &r)
	if err != nil {
		return nil, errors.WithStack(err)
	}
	return &r.Result, nil
}

// Register is
func (n *Node) Register(e ockam.Entity) (ockam.Claim, error) {
	doc := entityToDIDDocument(e)
	cl, err := claim.New(
		doc,
		claim.Type("EntityRegistrationClaim"),
		claim.Issuer(e),
		claim.Subject(e),
	)
	if err != nil {
		return nil, err
	}

	d := cl.Data()
	d["registrationClaim"] = cl.ID()
	cl.SetData(d)

	err = n.Submit(cl)
	if err != nil {
		return nil, err
	}

	return cl, nil
}

func entityToDIDDocument(e ockam.Entity) map[string]interface{} {
	doc := map[string]interface{}{
		"id":             e.ID().String(),
		"publicKey":      []map[string]interface{}{},
		"authentication": []map[string]interface{}{},
	}

	for _, k := range e.PublicKeys() {
		pk := map[string]interface{}{
			"id":   e.ID().String() + k.Label(),
			"type": k.Type(),
		}

		auth := map[string]interface{}{
			"type":      strings.Replace(k.Type(), "VerificationKey", "SignatureAuthentication", 1),
			"publicKey": e.ID().String() + k.Label(),
		}

		switch k.Encoding() {
		case "Pem":
			pk["publicKeyPem"] = k.Value()
		case "Hex":
			pk["publicKeyHex"] = k.Value()
		case "Base58":
			pk["publicKeyBase58"] = k.Value()
		}

		doc["publicKey"] = append(doc["publicKey"].([]map[string]interface{}), pk)
		doc["authentication"] = append(doc["authentication"].([]map[string]interface{}), auth)
	}

	for ak, av := range e.Attributes() {
		doc[ak] = av
	}

	return doc
}

// Submit is
func (n *Node) Submit(cl ockam.Claim) error {

	cl.Issuer().Signers()[0].Sign(cl)

	claimJSON, err := cl.MarshalJSON()
	if err != nil {
		return err
	}

	// var prettyJSON bytes.Buffer
	// json.Indent(&prettyJSON, claimJSON, "", "\t")
	// if err != nil {
	// 	return err
	// }
	// fmt.Printf("%s", string(prettyJSON.Bytes()))

	s := hex.EncodeToString(claimJSON) // base?
	_, err = n.BroadcastTxSync(cl.ID() + "=" + s)
	if err != nil {
		return err
	}

	return err
}
