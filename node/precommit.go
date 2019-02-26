package node

import (
	"bytes"
	"time"

	"github.com/pkg/errors"
	amino "github.com/tendermint/go-amino"
)

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

type ValAddress = HexBytes

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

func (pc *Precommit) SignBytes(chainID string) ([]byte, error) {

	var cdc = amino.NewCodec()

	cv := CanonicalizeVote(chainID, pc)
	bz, err := cdc.MarshalBinaryLengthPrefixed(cv)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	return bz, nil
}

type BlockID struct {
	Hash        HexBytes      `json:"hash"`
	PartsHeader PartSetHeader `json:"parts"`
}

// Equals returns true if the BlockID matches the given BlockID
func (blockID BlockID) Equals(other BlockID) bool {
	return bytes.Equal(blockID.Hash, other.Hash) &&
		blockID.PartsHeader.Equals(other.PartsHeader)
}

type PartSetHeader struct {
	Total int      `json:"total,string"`
	Hash  HexBytes `json:"hash"`
}

func (psh PartSetHeader) Equals(other PartSetHeader) bool {
	return psh.Total == other.Total && bytes.Equal(psh.Hash, other.Hash)
}
