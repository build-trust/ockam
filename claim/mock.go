package claim

type mockSignature struct{}

func (s mockSignature) Type() string           { return "" }
func (s mockSignature) Creator() string        { return "" }
func (s mockSignature) Created() string        { return "" }
func (s mockSignature) Domain() string         { return "" }
func (s mockSignature) Nonce() string          { return "" }
func (s mockSignature) SignatureValue() []byte { return []byte{} }
func (s mockSignature) SignedValue() []byte    { return []byte{} }
