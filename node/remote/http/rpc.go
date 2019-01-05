package http

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"time"

	"github.com/ockam-network/ockam/node"
	"github.com/pkg/errors"
)

// CommitResponse is
type CommitResponse struct {
	Error  interface{} `json:"error"`
	Result node.Commit `json:"result"`
}

// TxResponse is
type TxResponse struct {
	Error  interface{} `json:"error"`
	Result node.Tx     `json:"result"`
}

// BroadcastTxSyncResponse is
type BroadcastTxSyncResponse struct {
	Result struct {
		Code int    `json:"code"`
		Data string `json:"data"`
		Log  string `json:"log"`
		Hash string `json:"hash"`
	} `json:"result"`
	Error interface{} `json:"error"`
}

// Commit fetches the the commit at the provided height
func (n *Node) Commit(height string) (*node.Commit, error) {
	r := new(CommitResponse)
	err := n.Call(fmt.Sprintf("/commit?height=%s", height), &r)
	if err != nil {
		return nil, errors.WithStack(err)
	}
	return &r.Result, nil
}

// BroadcastTxSync broadcasts a transaction
func (n *Node) BroadcastTxSync(tx string) ([]byte, error) {
	r := new(BroadcastTxSyncResponse)
	fmt.Println(fmt.Sprintf("/broadcast_tx_sync?tx=\"%s\"", tx))
	err := n.Call(fmt.Sprintf("/broadcast_tx_sync?tx=\"%s\"", tx), &r)
	if err != nil {
		return nil, errors.WithStack(err)
	}
	fmt.Println((&r.Result).Hash)
	return []byte((&r.Result).Hash), nil
}

// Tx fetched a transaction including a proof for the transaction
func (n *Node) Tx(hash []byte) (*node.Tx, error) {
	r := new(TxResponse)
	err := n.Call(fmt.Sprintf("/tx?hash=0x%s&prove=true", hash), &r)
	if err != nil {
		return nil, errors.WithStack(err)
	}
	return &r.Result, nil
}

// Call makes an RPC call
func (n *Node) Call(q string, r interface{}) error {
	httpClient := &http.Client{Timeout: time.Second * 10}
	endpoint := fmt.Sprintf("http://%s:%d", n.ip, n.port)

	resp, err := httpClient.Get(endpoint + q)
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
