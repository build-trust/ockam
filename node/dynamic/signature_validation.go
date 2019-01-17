package main

import (
	"time"

	"github.com/pkg/errors"
	// tmAmino "github.com/tendermint/tendermint/crypto/encoding/amino"
	amino "github.com/tendermint/go-amino"
	// "github.com/tendermint/tendermint/types"
	"golang.org/x/crypto/ed25519"
)

// TimeFormat is used for generating the sigs
const TimeFormat = time.RFC3339Nano

func (pc *Precommit) SignBytes(chainID string) ([]byte, error) {

	var cdc = amino.NewCodec()

	cv := CanonicalizeVote(chainID, pc)
	bz, err := cdc.MarshalBinaryLengthPrefixed(cv)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	return bz, nil
}

func (pc *PubKey) VerifyBytes(msg []byte, sig []byte) bool {
	isVerified := ed25519.Verify(pc.Value[:], msg, sig)

	return isVerified
}

func CanonicalizeVote(chainID string, vote *Precommit) CanonicalVote {

	return CanonicalVote{
		Type:      vote.Type,
		Height:    vote.Height,
		Round:     int64(vote.Round),
		Timestamp: vote.Timestamp,
		BlockID:   CanonicalizeBlockID(vote.BlockID),
		ChainID:   chainID,
	}
}

func CanonicalizeBlockID(blockID BlockID) CanonicalBlockID {
	return CanonicalBlockID{
		Hash:        blockID.Hash,
		PartsHeader: CanonicalizePartSetHeader(blockID.PartsHeader),
	}
}

func CanonicalizePartSetHeader(psh PartSetHeader) CanonicalPartSetHeader {
	return CanonicalPartSetHeader{
		psh.Hash,
		psh.Total,
	}
}

//No longer used
func CanonicalTime(t time.Time) string {
	return Canonical(t).Format(TimeFormat)
}

//no longer used
// Canonical returns UTC time with no monotonic component.
// Stripping the monotonic component is for time equality.
// See https://github.com/tendermint/tendermint/pull/2203#discussion_r215064334
func Canonical(t time.Time) time.Time {
	return t.Round(0).UTC()
}
