package types

import (
	"bytes"
	"math"
	"sort"

	"github.com/pkg/errors"
)

type ValidatorSet struct {
	Validators []*Validator
	Proposer   *Validator //Don't know if we need this for what we're doing here

	totalVotingPower int64
}

func (v *Validator) Copy() *Validator {
	vCopy := *v
	return &vCopy
}

func NewValidatorSet(valz []*Validator) (*ValidatorSet, error) {
	if valz != nil && len(valz) == 0 {
		return nil, errors.New("validator set initialization slice cannot be an empty slice (but it can be nil)")
	}
	validators := make([]*Validator, len(valz))
	for i, val := range valz {
		validators[i] = val.Copy()
	}
	sort.Sort(ValidatorsByAddress(validators))
	vals := &ValidatorSet{
		Validators: validators,
	}
	//establish total voting power on initialization of val set
	_ = vals.TotalVotingPower()
	return vals, nil
}

// Size returns the length of the validator set.
func (vals *ValidatorSet) Size() int {
	return len(vals.Validators)
}

// GetByIndex returns the validator's address and validator itself by index.
// It returns nil values if index is less than 0 or greater or equal to
// len(ValidatorSet.Validators).
func (vals *ValidatorSet) GetByIndex(index int) (address []byte, val *Validator) {
	if index < 0 || index >= len(vals.Validators) {
		return nil, nil
	}
	val = vals.Validators[index]
	return val.Address, val.Copy()
}

func (vals *ValidatorSet) GetByAddress(address []byte) (index int, val *Validator) {
	idx := sort.Search(len(vals.Validators), func(i int) bool {
		return bytes.Compare(address, vals.Validators[i].Address) <= 0
	})
	if idx < len(vals.Validators) && bytes.Equal(vals.Validators[idx].Address, address) {
		return idx, vals.Validators[idx].Copy()
	}
	return -1, nil
}

func (vals *ValidatorSet) TotalVotingPower() int64 {
	if vals.totalVotingPower == 0 {
		for _, val := range vals.Validators {
			//mind overflow
			vals.totalVotingPower = safeAddClip(vals.totalVotingPower, val.VotingPower)
		}
	}
	return vals.totalVotingPower
}

func (vals *ValidatorSet) VerifyCommit(chainID string, blockID BlockID, height int64, commit *NestedCommit) error {
	if vals.Size() != len(commit.Precommits) {
		return errors.Errorf("Invalid commit -- wrong set size: %v vs %v", vals.Size(), len(commit.Precommits))
	}
	commitHeight, err := commit.Height()
	if err != nil {
		return err
	}
	if height != commitHeight {
		return errors.Errorf("Invalid commit -- wrong height: %v vs %v", height, commitHeight)
	}

	talliedVotingPower := int64(0)
	round := commit.Round()

	for idx, precommit := range commit.Precommits {
		if precommit == nil {
			continue //OK, some precommits can be missing
		}
		if precommit.Height != height {
			return errors.Errorf("Invalid commit -- wrong height: want %v got %v", height, precommit.Height)
		}
		if precommit.Round != round {
			return errors.Errorf("Invalid commit -- wrong round: want %v got %v", round, precommit.Round)
		}
		_, val := vals.GetByIndex(idx)
		precommitSignBytes, err := precommit.SignBytes(chainID)
		if err != nil {
			return err
		}

		if !val.PubKey.VerifyBytes(precommitSignBytes, precommit.Signature) {
			return errors.Errorf("Invalid commit -- invalid signature: %v", precommit)
		}
		//Good precommit!
		if blockID.Equals(precommit.BlockID) {
			talliedVotingPower += val.VotingPower
		} else {
			// It's OK that the BlockID doesn't match.  We include stray
			// precommits to measure validator availability.
		}
	}

	if talliedVotingPower > vals.TotalVotingPower()*2/3 {
		return nil
	}
	return errors.Errorf("Invalid commit -- insufficient voting power: got %v, needed %v",
		talliedVotingPower, (vals.TotalVotingPower()*2/3 + 1))
}

func (vals *ValidatorSet) VerifyFutureCommit(newSet *ValidatorSet, chainID string, blockID BlockID, height int64, commit *NestedCommit) error {
	oldVals := vals
	//commit must be a valid commit for new set
	err := newSet.VerifyCommit(chainID, blockID, height, commit)
	if err != nil {
		return err
	}

	//check old voting power
	oldVotingPower := int64(0)
	seen := map[int]bool{}
	round := commit.Round()

	for idx, precommit := range commit.Precommits {
		if precommit == nil {
			continue
		}
		if precommit.Height != height {
			return errors.Errorf("Blocks don't match - %d vs %d", height, precommit.Height)
		}
		if precommit.Round != round {
			return errors.Errorf("Invalid commit -- wrong round: %v vs %v", round, precommit.Round)
		}
		if precommit.Type != PrecommitType {
			return errors.Errorf("Invalid commit -- not precommit @ index %v", idx)
		}
		//see if this validator is in oldVals
		idx, val := oldVals.GetByAddress(precommit.ValidatorAddress)
		if val == nil || seen[idx] {
			continue //missing or double vote
		}
		seen[idx] = true

		//Validate signature
		precommitSignBytes, err := precommit.SignBytes(chainID)
		if err != nil {
			return err
		}
		if !val.PubKey.VerifyBytes(precommitSignBytes, precommit.Signature) {
			return errors.Errorf("Invalid commit -- invalid signature: %v", precommit)
		}
		//Good Precommit!
		if blockID.Equals(precommit.BlockID) {
			oldVotingPower += val.VotingPower
		} else {
			// It's OK that the BlockID doesn't match.  We include stray
			// precommits to measure validator availability.
		}
	}

	if oldVotingPower <= oldVals.TotalVotingPower()*2/3 {
		return errors.Errorf("Invalid commit -- insufficient old voting power: got %v, needed %v",
			oldVotingPower, oldVals.TotalVotingPower()*2/3+1)
	}

	return nil
}

//-------------------------------------
// Implements sort for sorting validators by address.

// Sort validators by address
type ValidatorsByAddress []*Validator

func (valz ValidatorsByAddress) Len() int {
	return len(valz)
}

func (valz ValidatorsByAddress) Less(i, j int) bool {
	return bytes.Compare(valz[i].Address, valz[j].Address) == -1
}

func (valz ValidatorsByAddress) Swap(i, j int) {
	it := valz[i]
	valz[i] = valz[j]
	valz[j] = it
}

//helpers -- Safe Addition
func safeAdd(a, b int64) (int64, bool) {
	if b > 0 && a > math.MaxInt64-b {
		return -1, true
	} else if b < 0 && a < math.MinInt64-b {
		return -1, true
	}
	return a + b, false
}

func safeAddClip(a, b int64) int64 {
	c, overflow := safeAdd(a, b)
	if overflow {
		if b < 0 {
			return math.MinInt64
		}
		return math.MaxInt64
	}
	return c
}
