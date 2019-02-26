package node

//FullCommit is
type FullCommit struct {
	Height             string       `json:"height"`
	NextValidatorsHash string       `json:"next_validators_hash"`
	Validators         []*Validator `json:"validators"`
	Commit             NestedCommit `json:"commit"`
}

func MakeFullCommit(vals []*Validator, commit *Commit) (fc *FullCommit, err error) {
	fc = new(FullCommit)
	fc.Height = commit.SignedHeader.Header.Height
	fc.Validators = vals
	fc.NextValidatorsHash = commit.SignedHeader.Header.NextValidatorsHash
	fc.Commit = commit.SignedHeader.Commit

	return fc, nil
}
