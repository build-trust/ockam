package main

import (
	"fmt"
	"strconv"

	"github.com/pkg/errors"
)

type Verifier struct {
	provider Provider
}

func NewVerifier(p Provider) *Verifier {
	v := Verifier{
		provider: p,
	}

	return &v
}

func (v *Verifier) Verify(c *CommitResult, n *Node) (bool, error) {
	lastTrusted, err := v.provider.LoadTrustedCommit()
	if err != nil {
		return false, errors.WithStack(err)
	}

	//if current validator hash == last trusted, check that current.height - 1 's next val set == current's val set
	if c.SignedHeader.Header.ValidatorsHash == lastTrusted.NextValidatorsHash {
		h, err := strconv.ParseInt(c.SignedHeader.Header.Height, 10, 64)
		if err != nil {
			return false, errors.WithStack(err)
		}

		heightMinusOne, err := n.GetTrustedCommit(strconv.FormatInt(h-1, 10))
		if err != nil {
			return false, errors.WithStack(err)
		}

		if heightMinusOne.NextValidatorsHash == c.SignedHeader.Header.ValidatorsHash {
			//we are good to go, save new last trusted
			newTrusted, err := n.GetTrustedCommit(strconv.FormatInt(h, 10))
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
				err = v.provider.SaveTrustedCommit(newTrusted)
				if err != nil {
					return true, errors.WithStack(err)
				}

				return true, nil
			} else {
				fmt.Println(err)
				return false, nil
			}

		} else {
			_, err = v.updateToHeight(c.SignedHeader.Header.Height, n, c)
			if err != nil {
				return false, err
			}

			//last trusted height should equal c.SignedHeader.Header.Height so retry
			return v.Verify(c, n)
		}

	} else {
		//We need to get a trusted commit for the height in question
		_, err = v.updateToHeight(c.SignedHeader.Header.Height, n, c)
		if err != nil {
			return false, err
		}

		//last trusted height should equal c.SignedHeader.Header.Height so retry
		return v.Verify(c, n)
	}

}

func (v *Verifier) updateToHeight(height string, n *Node, c *CommitResult) (*TrustedCommit, error) {
	//get full commit for height in question
	commitFromChain, err := n.GetTrustedCommit(height)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	if commitFromChain.Height != height {
		return nil, errors.New("Commit not found")
	}

FOR_LOOP:
	for {
		//Get lastest trusted full commit from provider
		trusted, err := v.provider.LoadTrustedCommit()
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
			_, err = v.updateToHeight(strconv.FormatInt(midInt, 10), n, c)
			if err != nil {
				return nil, err
			}

			//if we made it to mid, retry
			continue FOR_LOOP
		}
		return nil, err
	}
}

func (v *Verifier) verifyAndSave(trusted, source *TrustedCommit, c *CommitResult) error {

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

	err = oldValSet.VerifyFutureCommit(newValSet, c.SignedHeader.Header.ChainID, c.SignedHeader.Commit.BlockID, height, &c.SignedHeader.Commit)
	if err != nil {
		fmt.Println("Too much change!")
		return errors.New("Too much change")
	}

	return v.provider.SaveTrustedCommit(source)
}
