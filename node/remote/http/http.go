// Package http implements ockam.Node interface to enable interaction
// with a remote node that can be commiunicated with over http.
package http

import (
	"bytes"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"strings"

	"github.com/ockam-network/ockam/entity"

	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/chain"
	"github.com/ockam-network/ockam/claim"
	"github.com/ockam-network/ockam/key/ed25519"
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

func (n *Node) FullCommit(height string) (*node.FullCommit, error) {
	validators, err := n.Validators(height)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	commit, err := n.Commit(height)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	fc, err := node.MakeFullCommit(validators, commit)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	return fc, nil

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
		hash:   n.latestCommit.SignedHeader.Commit.BlockID.Hash.String(),
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

func (n *Node) FetchEntity(key string) ([]byte, ockam.Entity, error) {
	value, err := n.ABCIQuery(key)
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	decoded, err := decodeValue(value)
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	//for printing/debugging
	var prettyJSON bytes.Buffer
	err = json.Indent(&prettyJSON, decoded, "", "\t")
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	//to construct an entity, we need a DID, a signer(s) and attributes
	//first unmarshal doc into map
	var m map[string]interface{}
	err = json.Unmarshal(decoded, &m)
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	//did is the id field
	id, err := did.Parse(m["id"].(string))
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	//loop through publicKeys, creating a signer for each
	//currently, ed25519 is only key supported
	//TODO add check for key type here
	var signers []ockam.Signer
	publicKeys := m["publicKey"].([]interface{})
	for _, k := range publicKeys {
		key := k.(map[string]interface{})
		signer, err := ed25519.NewWithExistingKey(key["publicKeyHex"].(string))
		if err != nil {
			return nil, nil, errors.WithStack(err)
		}
		signers = append(signers, signer)
	}

	//collect the attributes
	var attributes = make(map[string]interface{})
	for k, v := range m {
		if k != "publicKey" && k != "id" && k != "authentication" && k != "registrationClaim" {
			attributes[k] = v
		}
	}

	ent, err := entity.New(attributes, entity.ID(id), entity.SignerArray(signers))
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	return prettyJSON.Bytes(), ent, nil

}

func (n *Node) FetchClaim(key string) ([]byte, ockam.Claim, error) {
	value, err := n.ABCIQuery(key)
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	decoded, err := decodeValue(value)
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	var prettyJSON bytes.Buffer
	err = json.Indent(&prettyJSON, decoded, "", "\t")
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	//unmarshal into map so we can access fields to create claim
	var m map[string]interface{}
	err = json.Unmarshal(decoded, &m)
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	//get the type
	typeArray := m["type"].([]interface{})
	t := typeArray[0].(string)

	//get the subject from the claim field
	claimMap := m["claim"].(map[string]interface{})

	var empty map[string]interface{}
	subject, err := did.Parse(claimMap["id"].(string))
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}
	subjectEntity, err := entity.New(empty, entity.ID(subject))
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	//get issuer from issuer field
	issuer, err := did.Parse(m["issuer"].(string))
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}
	issuerEntity, err := entity.New(empty, entity.ID(issuer))
	if err != nil {
		return nil, nil, errors.WithStack(err)
	}

	cl, err := claim.New(
		claimMap,
		claim.Issuer(issuerEntity),
		claim.Subject(subjectEntity),
		claim.Type(t),
		claim.ID(m["id"].(string)),
		claim.Issued(m["issued"].(string)),
	)

	signatures := m["signatures"].([]interface{})
	for _, s := range signatures {
		sig := s.(map[string]interface{})
		if sig["type"].(string) == "Ed25519Signature2018" { //Todo: add support for other sig types, consider switch statement
			signature := ed25519.AssembleSignature(
				sig["type"].(string),
				sig["creator"].(string),
				sig["created"].(string),
				sig["domain"].(string),
				sig["nonce"].(string),
				[]byte(sig["signatureValue"].(string)))
			cl.AddSignature(signature)
		}
	}

	return prettyJSON.Bytes(), cl, err
}

//value is base64 and hex encoded
//decode base64, then hex
func decodeValue(value string) ([]byte, error) {
	hexValue, err := base64.StdEncoding.DecodeString(value)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	return hex.DecodeString(string(hexValue))
}
