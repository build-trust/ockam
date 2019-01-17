package types

import (
	"bytes"
	"time"

	cmn "github.com/ockam-network/ockam/node/dynamic/common"
	"github.com/pkg/errors"
	amino "github.com/tendermint/go-amino"
)

// CommitResponse is
type CommitResponse struct {
	Jsonrpc string       `json:"jsonrpc"`
	ID      string       `json:"id"`
	Error   interface{}  `json:"error"`
	Result  CommitResult `json:"result"`
}

// CommitResult is
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
	Total int    `json:"total,string"`
	Hash  []byte `json:"hash"`
}

func (psh PartSetHeader) Equals(other PartSetHeader) bool {
	return psh.Total == other.Total && bytes.Equal(psh.Hash, other.Hash)
}

type ValAddress = cmn.HexBytes

// Precommit is
type Precommit struct {
	Type             byte       `json:"type"`
	Height           int64      `json:"height,string"`
	Round            int        `json:"round,string"`
	Timestamp        time.Time  `json:"timestamp"`
	BlockID          BlockID    `json:"block_id"`
	ValidatorAddress ValAddress `json:"validator_address"`
	ValidatorIndex   string     `json:"validator_index"`
	Signature        []byte     `json:"signature"`
}

func (pc *Precommit) SignBytes(chainID string) ([]byte, error) {

	var cdc = amino.NewCodec()

	bz, err := cdc.MarshalJSON(CanonicalVote(chainID, pc))
	if err != nil {
		return nil, errors.WithStack(err)
	}

	return bz, nil
}
