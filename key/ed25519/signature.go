package ed25519

// Signature is
type Signature struct {
	t              string
	creator        string
	created        string
	domain         string
	nonce          string
	signatureValue []byte
	signedValue    []byte
}

// Type is
// https://w3c-ccg.github.io/ld-cryptosuite-registry/#ed25519signature2018
func (s *Signature) Type() string {
	return s.t
}

// Creator is
func (s *Signature) Creator() string {
	return s.creator
}

// Created is
func (s *Signature) Created() string {
	return s.created
}

// Domain is
func (s *Signature) Domain() string {
	return "ockam"
}

// Nonce is
// https://web-payments.org/vocabs/security#nonce
func (s *Signature) Nonce() string {
	return s.nonce
}

// SignatureValue is
// https://web-payments.org/vocabs/security#signatureValue
func (s *Signature) SignatureValue() []byte {
	return s.signatureValue
}

func (s *Signature) SignedValue() []byte {
	return s.signedValue
}

//AssembleSignature is used for converting map[string]interface{} to Signature in FetchClaim()
func AssembleSignature(t, creator, created, domain, nonce string, signatureValue []byte) *Signature {
	sig := &Signature{
		t:              t,
		creator:        creator,
		created:        created,
		domain:         domain,
		nonce:          nonce,
		signatureValue: signatureValue,
	}

	return sig
}
