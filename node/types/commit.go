package types

import (
	"time"
)

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
		Commit NestedCommit `json:"commit"`
	} `json:"signed_header"`
	Canonical bool `json:"canonical"`
}

//
type NestedCommit struct {
	BlockID    BlockID      `json:"block_id"`
	Precommits []*Precommit `json:"precommits"`

	firstPrecommit *Precommit
}

func (commit *NestedCommit) FirstPrecommit() *Precommit {
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
func (commit *NestedCommit) Height() (int64, error) {
	if len(commit.Precommits) == 0 {
		return 0, nil
	}

	return commit.FirstPrecommit().Height, nil
}

func (commit *NestedCommit) Round() int {
	if len(commit.Precommits) == 0 {
		return 0
	}
	return commit.FirstPrecommit().Round
}
