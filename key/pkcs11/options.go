package pkcs11

import "crypto"

// Option is a functional option for setting attribtues
// on a PKCS11 type
type Option func(*PKCS11) error

func ModulePath(path string) Option {
	return func(p *PKCS11) error {
		// TODO(malnick) validate path is valid
		p.modulePath = path
		return nil
	}
}

func PIN(pin string) Option {
	return func(p *PKCS11) error {
		p.pin = pin
		return nil
	}
}

func TokenLabel(label string) Option {
	return func(p *PKCS11) error {
		p.tokenLabel = label
		return nil
	}
}

func PublicKey(key crypto.PublicKey) Option {
	return func(p *PKCS11) error {
		p.publicKey = key
		return nil
	}
}
