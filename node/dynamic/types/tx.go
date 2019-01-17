package types

// TxResponse is
type TxResponse struct {
	Jsonrpc string      `json:"jsonrpc"`
	ID      string      `json:"id"`
	Error   interface{} `json:"error"`
	Result  Tx          `json:"result"`
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

// BroadcastTxSyncResponse is
type BroadcastTxSyncResponse struct {
	Jsonrpc string `json:"jsonrpc"`
	ID      string `json:"id"`
	Result  struct {
		Code int    `json:"code"`
		Data string `json:"data"`
		Log  string `json:"log"`
		Hash string `json:"hash"`
	} `json:"result"`
	Error interface{} `json:"error"`
}
