package types

// ValidatorsResponse is
type ValidatorsResponse struct {
	Jsonrpc string `json:"jsonrpc"`
	ID      string `json:"id"`
	Result  struct {
		BlockHeight string `json:"block_height"`
		Validators  []*Validator
	} `json:"result"`
	Error interface{} `json:"error"`
}

// Validator is
type Validator struct {
	Address     HexBytes `json:"address"`
	PubKey      PubKey   `json:"pub_key"`
	VotingPower int64    `json:"voting_power,string"`
	Accum       string   `json:"accum"`
}
