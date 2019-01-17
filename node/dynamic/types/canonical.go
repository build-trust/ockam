package types

import "time"

const TimeFormat = time.RFC3339Nano

type CanonicalJSONVote struct {
	ChainID   string               `json:"@chain_id"`
	Type      string               `json:"@type"`
	BlockID   CanonicalJSONBlockID `json:"block_id"`
	Height    int64                `json:"height"`
	Round     int                  `json:"round"`
	Timestamp string               `json:"timestamp"`
	VoteType  byte                 `json:"type"`
}

type CanonicalJSONBlockID struct {
	Hash        []byte                     `json:"hash,omitempty"`
	PartsHeader CanonicalJSONPartSetHeader `json:"parts,omitempty"`
}

type CanonicalJSONPartSetHeader struct {
	Hash  []byte `json:"hash,omitempty"`
	Total int    `json:"total,omitempty"`
}

func CanonicalVote(chainID string, vote *Precommit) CanonicalJSONVote {

	return CanonicalJSONVote{
		ChainID:   chainID,
		Type:      "vote",
		BlockID:   CanonicalBlockID(vote.BlockID),
		Height:    vote.Height,
		Round:     vote.Round,
		Timestamp: CanonicalTime(vote.Timestamp),
		VoteType:  vote.Type,
	}
}

func CanonicalBlockID(blockID BlockID) CanonicalJSONBlockID {
	return CanonicalJSONBlockID{
		Hash:        blockID.Hash,
		PartsHeader: CanonicalPartSetHeader(blockID.PartsHeader),
	}
}

func CanonicalPartSetHeader(psh PartSetHeader) CanonicalJSONPartSetHeader {

	// total, err := strconv.Atoi(psh.Total)
	// if err != nil {
	// 	fmt.Println(err)
	// }
	return CanonicalJSONPartSetHeader{
		psh.Hash,
		psh.Total,
	}
}

func CanonicalTime(t time.Time) string {
	return Canonical(t).Format(TimeFormat)
}

// Canonical returns UTC time with no monotonic component.
// Stripping the monotonic component is for time equality.
// See https://github.com/tendermint/tendermint/pull/2203#discussion_r215064334
func Canonical(t time.Time) time.Time {
	return t.Round(0).UTC()
}
