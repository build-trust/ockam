package main

import (
	"bytes"
	"time"

	cmn "github.com/ockam-network/ockam/dynamic/common"
)

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

// CommitResponse is
type CommitResponse struct {
	Jsonrpc string       `json:"jsonrpc"`
	ID      string       `json:"id"`
	Error   interface{}  `json:"error"`
	Result  CommitResult `json:"result"`
}

// Commit is
type CommitResult struct {
	SignedHeader struct {
		Header struct {
			Version struct {
				Block string `json:"block"`
				App   string `json:"app"`
			} `json:"version"`
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
		Commit Commit `json:"commit"`
	} `json:"signed_header"`
	Canonical bool `json:"canonical"`
}

type Commit struct {
	BlockID    BlockID      `json:"block_id"`
	Precommits []*Precommit `json:"precommits"`

	firstPrecommit *Precommit
}

func (commit *Commit) FirstPrecommit() *Precommit {
	if len(commit.Precommits) == 0 {
		return nil
	}
	if commit.firstPrecommit != nil {
		return commit.firstPrecommit
	}
	for _, precommit := range commit.Precommits {
		if precommit != nil {
			commit.firstPrecommit = precommit
			return precommit
		}
	}
	return &Precommit{}
}

// Height returns the height of the commit
func (commit *Commit) Height() (int64, error) {
	if len(commit.Precommits) == 0 {
		return 0, nil
	}

	return commit.FirstPrecommit().Height, nil
}

func (commit *Commit) Round() int {
	if len(commit.Precommits) == 0 {
		return 0
	}
	return commit.FirstPrecommit().Round
}

// SignedMsgType is a type of signed message in the consensus.
type SignedMsgType byte

const (
	// Votes
	PrevoteType   SignedMsgType = 0x01
	PrecommitType SignedMsgType = 0x02

	// Proposals
	ProposalType SignedMsgType = 0x20

	// Heartbeat
	HeartbeatType SignedMsgType = 0x30
)

type ValAddress = cmn.HexBytes

// Precommit is
type Precommit struct {
	Type             SignedMsgType `json:"type"`
	Height           int64         `json:"height,string"`
	Round            int           `json:"round,string"`
	Timestamp        time.Time     `json:"timestamp"`
	BlockID          BlockID       `json:"block_id"`
	ValidatorAddress ValAddress    `json:"validator_address"`
	ValidatorIndex   string        `json:"validator_index"`
	Signature        []byte        `json:"signature"`
}

type BlockID struct {
	Hash        cmn.HexBytes  `json:"hash"`
	PartsHeader PartSetHeader `json:"parts"`
}

// Equals returns true if the BlockID matches the given BlockID
func (blockID BlockID) Equals(other BlockID) bool {
	return bytes.Equal(blockID.Hash, other.Hash) &&
		blockID.PartsHeader.Equals(other.PartsHeader)
}

type PartSetHeader struct {
	Total int          `json:"total,string"`
	Hash  cmn.HexBytes `json:"hash"`
}

func (psh PartSetHeader) Equals(other PartSetHeader) bool {
	return psh.Total == other.Total && bytes.Equal(psh.Hash, other.Hash)
}

// TxResponse is
type TxResponse struct {
	Jsonrpc string      `json:"jsonrpc"`
	ID      string      `json:"id"`
	Error   interface{} `json:"error"`
	Result  Tx          `json:"result"`
}

// Tx is
type Tx struct {
	Proof    TxProof `json:"proof"`
	Tx       string  `json:"tx"`
	TxResult struct {
		Log  string `json:"log"`
		Data string `json:"data"`
		Code string `json:"code"`
	} `json:"tx_result"`
	Height string `json:"height"`
	Hash   string `json:"hash"`
}

type TxProof struct {
	RootHash cmn.HexBytes `json:"RootHash"`
	Data     TxData       `json:"Data"`
	Proof    SimpleProof  `json:"Proof"`
}

type TxData []byte

type SimpleProof struct {
	Total    int      `json:"total,string"`
	Index    int      `json:"index,string"`
	LeafHash []byte   `json:"leaf_hash"`
	Aunts    [][]byte `json:"aunts"`
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
	Address     cmn.HexBytes `json:"address"`
	PubKey      PubKey       `json:"pub_key"`
	VotingPower int64        `json:"voting_power,string"`
	Accum       string       `json:"accum"`
}

type PubKey struct {
	Type  string `json:"type"`
	Value []byte `json:"value"`
}

type CanonicalVote struct {
	Type      SignedMsgType // type alias for byte
	Height    int64         `binary:"fixed64"`
	Round     int64         `binary:"fixed64"`
	Timestamp time.Time
	BlockID   CanonicalBlockID
	ChainID   string
}

type CanonicalBlockID struct {
	Hash        cmn.HexBytes
	PartsHeader CanonicalPartSetHeader
}

type CanonicalPartSetHeader struct {
	Hash  cmn.HexBytes
	Total int
}
