package node

import (
	"encoding/json"
	"fmt"
	"strconv"

	"github.com/ockam-network/ockam"
	"github.com/pkg/errors"
)

type Verifier struct {
	store ockam.CommitStore
}

func NewVerifier(store ockam.CommitStore) *Verifier {
	v := Verifier{
		store: store,
	}

	return &v
}

func (v *Verifier) GetLastTrusted() (*FullCommit, error) {
	last, err := v.store.GetLastTrusted()
	if err != nil {
		fmt.Println(err)
	}

	bytes, err := json.Marshal(last)
	if err != nil {
		fmt.Println(err)
	}

	var lastTrustedFullCommit FullCommit
	err = json.Unmarshal(bytes, &lastTrustedFullCommit)
	if err != nil {
		fmt.Println(err)
	}

	return &lastTrustedFullCommit, nil
}

func (v *Verifier) SaveFullCommit(fc *FullCommit) error {
	return v.store.StoreLastTrusted(fc)
}

func (v *Verifier) Verify(c *Commit, p Peer) (bool, error) {
	lastTrusted, err := v.GetLastTrusted()
	if err != nil {
		return false, errors.WithStack(err)
	}

	//if current validator hash == last trusted, check that current.height - 1 's next val set == current's val set
	if c.SignedHeader.Header.ValidatorsHash == lastTrusted.NextValidatorsHash {
		h, err := strconv.ParseInt(c.SignedHeader.Header.Height, 10, 64)
		if err != nil {
			return false, errors.WithStack(err)
		}

		heightMinusOne, err := p.FullCommit(strconv.FormatInt(h-1, 10))
		if err != nil {
			return false, errors.WithStack(err)
		}

		if heightMinusOne.NextValidatorsHash == c.SignedHeader.Header.ValidatorsHash {
			//we are good to go, save new last trusted
			newTrusted, err := p.FullCommit(strconv.FormatInt(h, 10))
			if err != nil {
				return false, errors.WithStack(err)
			}
			//but first, validate signatures
			valSet, err := NewValidatorSet(newTrusted.Validators)
			if err != nil {
				return false, errors.WithStack(err)
			}

			err = valSet.VerifyCommit(c.SignedHeader.Header.ChainID, c.SignedHeader.Commit.BlockID, h, &c.SignedHeader.Commit)
			if err == nil {
				//sigs are verified
				err = v.SaveFullCommit(newTrusted)
				if err != nil {
					return true, errors.WithStack(err)
				}

				return true, nil
			} else {
				fmt.Println(err)
				return false, nil
			}

		} else {
			_, err = v.updateToHeight(c.SignedHeader.Header.Height, p, c)
			if err != nil {
				return false, err
			}

			//last trusted height should equal c.SignedHeader.Header.Height so retry
			return v.Verify(c, p)
		}

	} else {
		//We need to get a trusted commit for the height in question
		_, err = v.updateToHeight(c.SignedHeader.Header.Height, p, c)
		if err != nil {
			return false, err
		}

		//last trusted height should equal c.SignedHeader.Header.Height so retry
		return v.Verify(c, p)
	}

}

func (v *Verifier) updateToHeight(height string, p Peer, c *Commit) (*FullCommit, error) {
	//get full commit for height in question
	commitFromChain, err := p.FullCommit(height)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	if commitFromChain.Height != height {
		return nil, errors.New("Commit not found")
	}

FOR_LOOP:
	for {
		//Get lastest trusted full commit from store
		trusted, err := v.GetLastTrusted()
		if err != nil {
			return nil, errors.WithStack(err)
		}

		//we have nothing to do
		if trusted.Height == height {
			return trusted, nil
		}

		//try to update
		err = v.verifyAndSave(trusted, commitFromChain, c)
		if err == nil {
			//we were able to update
			return commitFromChain, nil
		}

		if err.Error() == "Too much change" {
			//divide and conquer
			start, end := trusted.Height, commitFromChain.Height
			if !(start < end) {
				return nil, errors.New("Should not happen")
			}
			startInt, err := strconv.ParseInt(start, 10, 64)
			if err != nil {
				return nil, errors.WithStack(err)
			}

			endInt, err := strconv.ParseInt(end, 10, 64)
			if err != nil {
				return nil, errors.WithStack(err)
			}

			midInt := (startInt + endInt) / 2
			_, err = v.updateToHeight(strconv.FormatInt(midInt, 10), p, c)
			if err != nil {
				return nil, err
			}

			//if we made it to mid, retry
			continue FOR_LOOP
		}
		return nil, err
	}
}

func (v *Verifier) verifyAndSave(trusted, source *FullCommit, c *Commit) error {

	if trusted.Height >= source.Height {
		return errors.New("Should not happen")
	}

	oldValSet, err := NewValidatorSet(trusted.Validators)
	if err != nil {
		return errors.New("cannot make val set")
	}

	newValSet, err := NewValidatorSet(source.Validators)
	if err != nil {
		return errors.New("cannot make val set")
	}

	height, err := strconv.ParseInt(c.SignedHeader.Header.Height, 10, 64)
	if err != nil {
		return errors.WithStack(err)
	}

	chainID := c.SignedHeader.Header.ChainID
	err = oldValSet.VerifyFutureCommit(newValSet, chainID, c.SignedHeader.Commit.BlockID, height, &c.SignedHeader.Commit)
	if err != nil {
		fmt.Println("Too much change!")
		return errors.WithStack(err)
	}

	return v.SaveFullCommit(source)
}
