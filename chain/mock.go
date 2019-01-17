package chain

import "github.com/ockam-network/ockam"

type mockNode struct{}

func (n mockNode) Peers() []ockam.Node                        { return nil }
func (n mockNode) Chain() ockam.Chain                         { return nil }
func (n mockNode) ID() string                                 { return "" }
func (n mockNode) Sync() error                                { return nil }
func (n mockNode) LatestBlock() ockam.Block                   { return nil }
func (n mockNode) Register(ockam.Entity) (ockam.Claim, error) { return nil, nil }
func (n mockNode) Submit(ockam.Claim) error                   { return nil }
