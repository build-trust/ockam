package main

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"time"

	"github.com/pkg/errors"
)

type newNodeOptions struct {
	address string
}

// Node is
type Node struct {
	address    string
	endpoint   string
	httpClient *http.Client
}

// NewNodeOption is
type NewNodeOption func(*newNodeOptions)

// NewNode returns
func NewNode(options ...NewNodeOption) (*Node, error) {
	o := &newNodeOptions{}
	for _, option := range options {
		option(o)
	}

	endpoint := "http://" + o.address

	return &Node{
		address:    o.address,
		endpoint:   endpoint,
		httpClient: &http.Client{Timeout: time.Second * 10},
	}, nil
}

// Address is
func Address(a string) NewNodeOption {
	return func(o *newNodeOptions) {
		o.address = a
	}
}

// Status is
func (n *Node) Status() (*Status, error) {
	r := new(StatusResponse)
	err := n.Call("/status", &r)
	if err != nil {
		return nil, errors.WithStack(err)
	}
	return &r.Result, nil
}

// Commit is
func (n *Node) Commit(height string) (*CommitResult, error) {
	r := new(CommitResponse)
	err := n.Call(fmt.Sprintf("/commit?height=%s", height), &r)
	if err != nil {
		return nil, errors.WithStack(err)
	}
	return &r.Result, nil
}

// BroadcastTxSync is
func (n *Node) BroadcastTxSync(tx string) ([]byte, error) {
	r := new(BroadcastTxSyncResponse)
	err := n.Call(fmt.Sprintf("/broadcast_tx_sync?tx=\"%s\"", tx), &r)
	if err != nil {
		return nil, errors.WithStack(err)
	}
	return []byte((&r.Result).Hash), nil
}

// Tx is
func (n *Node) Tx(hash []byte) (*Tx, error) {
	r := new(TxResponse)
	err := n.Call(fmt.Sprintf("/tx?hash=0x%s&prove=true", hash), &r)
	if err != nil {
		return nil, errors.WithStack(err)
	}
	return &r.Result, nil
}

// WaitForTxHeight is
func (n *Node) WaitForTxHeight(hash []byte) (*Tx, error) {
	for {
		tx, err := n.Tx(hash)
		if err != nil {
			return nil, err
		}
		if tx.Height != "" {
			return tx, err
		}
		time.Sleep(2 * time.Second)
	}
}

// Validators is
func (n *Node) Validators(height string) ([]*Validator, error) {
	r := new(ValidatorsResponse)
	err := n.Call(fmt.Sprintf("/validators?height=%s", height), &r)
	if err != nil {
		return nil, errors.WithStack(err)
	}
	return r.Result.Validators, nil
}

//GetTrustedCommit collects the info needed for a TrustedCommit struct
func (n *Node) GetTrustedCommit(height string) (*TrustedCommit, error) {
	commit, err := n.Commit(height)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	validators, err := n.Validators(height)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	tc := new(TrustedCommit)
	tc.Height = height
	tc.Validators = validators
	tc.NextValidatorsHash = commit.SignedHeader.Header.NextValidatorsHash

	return tc, nil
}

// Call is
func (n *Node) Call(q string, r interface{}) error {
	resp, err := n.httpClient.Get(n.endpoint + q)
	if err != nil {
		return errors.WithStack(err)
	}

	body, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		return errors.WithStack(err)
	}

	err = json.Unmarshal(body, r)
	if err != nil {
		return errors.WithStack(err)
	}

	return nil
}
