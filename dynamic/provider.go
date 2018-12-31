package main

import (
	"encoding/json"
	"io/ioutil"
	"os"
	"os/user"
	"path/filepath"

	"github.com/pkg/errors"
)

//TrustedCommit is
type TrustedCommit struct {
	Height             string       `json:"height"`
	NextValidatorsHash string       `json:"next_validators_hash"`
	Validators         []*Validator `json:"validators"`
}

//provider interface stores and retrieves commits for verification
type Provider interface {
	LoadTrustedCommit() (*TrustedCommit, error)
	SaveTrustedCommit(tc *TrustedCommit) error
}

//implements provider
type DiskProvider struct {
	LastTrusted           *TrustedCommit
	TrustedCommitFilepath string
	//HasTrustedCommit      bool //when set to false, we know to look to block 1 to establish trust
}

func NewDiskProvider(n *Node) (*DiskProvider, error) {
	dp := DiskProvider{}

	dir, err := dp.CreateTrustDirectoryIfNotExist()
	if err != nil {
		return nil, err
	}

	dp.TrustedCommitFilepath = filepath.Join(dir, "last_trusted.json")

	//If last_trusted.json does not exist, initialize from block 1
	if _, err := os.Stat(dp.TrustedCommitFilepath); os.IsNotExist(err) {
		tc, err := n.GetTrustedCommit("1")
		if err != nil {
			return nil, errors.WithStack(err)
		}
		dp.SaveTrustedCommit(tc)
		dp.LastTrusted = tc
	} else { //else load from file
		tc, err := dp.LoadTrustedCommit()
		if err != nil {
			return nil, errors.WithStack(err)
		}
		dp.LastTrusted = tc
	}

	return &dp, nil
}

func (p *DiskProvider) LoadTrustedCommit() (*TrustedCommit, error) {
	if p.TrustedCommitFilepath == "" {
		return nil, errors.New("No filepath for trusted commit")
	}

	trustedFile, err := ioutil.ReadFile(p.TrustedCommitFilepath)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	var trusted TrustedCommit
	err = json.Unmarshal(trustedFile, &trusted)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	return &trusted, nil
}

func (p *DiskProvider) SaveTrustedCommit(tc *TrustedCommit) error {
	if p.TrustedCommitFilepath == "" {
		return errors.New("No filepath for trusted commit")
	}

	bytes, err := json.MarshalIndent(tc, "", "\t")
	if err != nil {
		return errors.WithStack(err)
	}

	err = ioutil.WriteFile(p.TrustedCommitFilepath, bytes, 0644)
	if err != nil {
		return errors.WithStack(err)
	}

	return nil
}

func (p *DiskProvider) CreateTrustDirectoryIfNotExist() (directory string, err error) {
	usr, err := user.Current()
	if err != nil {
		return directory, errors.WithStack(err)
	}

	directory = filepath.Join(usr.HomeDir, ".ockam", "trust")
	err = createDirIfNotExist(directory)
	if err != nil {
		return directory, err
	}

	//p.TrustedCommitFilepath = directory
	return directory, nil
}

func createDirIfNotExist(directory string) error {
	if _, err := os.Stat(directory); os.IsNotExist(err) {
		err = os.MkdirAll(directory, os.ModePerm)
		if err != nil {
			return errors.WithStack(err)
		}
	}
	return nil
}
