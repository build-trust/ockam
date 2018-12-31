package types

import "time"

// StatusResponse is
type StatusResponse struct {
	Jsonrpc string      `json:"jsonrpc"`
	ID      string      `json:"id"`
	Error   interface{} `json:"error"`
	Result  Status      `json:"result"`
}

// Status is
type Status struct {
	NodeInfo struct {
		ID      string `json:"id"`
		Network string `json:"network"`
	} `json:"node_info"`
	SyncInfo struct {
		LatestBlockHash   string    `json:"latest_block_hash"`
		LatestAppHash     string    `json:"latest_app_hash"`
		LatestBlockHeight string    `json:"latest_block_height"`
		LatestBlockTime   time.Time `json:"latest_block_time"`
		CatchingUp        bool      `json:"catching_up"`
	} `json:"sync_info"`
}
