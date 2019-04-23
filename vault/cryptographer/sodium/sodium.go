package sodium

// #cgo pkg-config: libsodium
// #include <stdlib.h>
// #include <sodium.h>
import "C"

import (
	"unsafe"

	"github.com/pkg/errors"
)

var (
	// ErrSodium is
	ErrSodium = errors.New("sodium")

	// ErrInvalidInput is
	ErrInvalidInput = errors.New("invalid input")

	cryptoSignBytes          = int(C.crypto_sign_bytes())
	cryptoSignPublicKeyBytes = int(C.crypto_sign_publickeybytes())
	cryptoSignSecretKeyBytes = int(C.crypto_sign_secretkeybytes())

	cryptoBoxPublicKeyBytes = int(C.crypto_box_publickeybytes())
	cryptoBoxSecretKeyBytes = int(C.crypto_box_secretkeybytes())
	cryptoBoxSealBytes      = int(C.crypto_box_sealbytes())
	cryptoBoxNonceBytes     = int(C.crypto_box_noncebytes())
	cryptoBoxMacBytes       = int(C.crypto_box_macbytes())
)

// Sodium is
type Sodium struct {
}

// NewSodium is
func NewSodium() (*Sodium, error) {
	return &Sodium{}, nil
}

// GenerateKeyPair is
func (s *Sodium) GenerateKeyPair() (publicKey []byte, secretKey []byte, err error) {
	publicKey = make([]byte, cryptoSignPublicKeyBytes)
	secretKey = make([]byte, cryptoSignSecretKeyBytes)
	if int(C.crypto_sign_keypair((*C.uchar)(&publicKey[0]), (*C.uchar)(&secretKey[0]))) != 0 {
		return nil, nil, ErrSodium
	}

	return publicKey, secretKey, nil
}

// AnonymousEncrypt is
func (s *Sodium) AnonymousEncrypt(unencrypted []byte, recipientPublicKey []byte) (encrypted []byte, err error) {
	publicKeyLength := len(recipientPublicKey)
	if !(publicKeyLength > 0 && publicKeyLength <= cryptoSignPublicKeyBytes) {
		return nil, ErrInvalidInput
	}
	rpk := make([]byte, cryptoBoxPublicKeyBytes)
	C.crypto_sign_ed25519_pk_to_curve25519((*C.uchar)(&rpk[0]), (*C.uchar)(&recipientPublicKey[0]))

	unencryptedLength := len(unencrypted)
	if !(unencryptedLength > 0) {
		return nil, ErrInvalidInput
	}

	encrypted = make([]byte, unencryptedLength+cryptoBoxSealBytes)
	if int(C.crypto_box_seal(
		(*C.uchar)(&encrypted[0]),
		(*C.uchar)(unsafe.Pointer(&unencrypted[0])),
		(C.ulonglong)(unencryptedLength),
		(*C.uchar)(&rpk[0]))) != 0 {
		return nil, ErrSodium
	}

	return encrypted, nil
}

// AnonymousDecrypt is
func (s *Sodium) AnonymousDecrypt(encrypted []byte, recipientSecretKey []byte) (unencrypted []byte, err error) {

	secretKeyLength := len(recipientSecretKey)
	if !(secretKeyLength > 0 && secretKeyLength <= cryptoSignSecretKeyBytes) {
		return nil, ErrInvalidInput
	}
	rsk := make([]byte, cryptoBoxSecretKeyBytes)
	C.crypto_sign_ed25519_sk_to_curve25519((*C.uchar)(&rsk[0]), (*C.uchar)(&recipientSecretKey[0]))

	recipientPublicKey := make([]byte, cryptoSignPublicKeyBytes)
	C.crypto_sign_ed25519_sk_to_pk((*C.uchar)(&recipientPublicKey[0]), (*C.uchar)(&recipientSecretKey[0]))
	publicKeyLength := len(recipientPublicKey)
	if !(publicKeyLength > 0 && publicKeyLength <= cryptoSignPublicKeyBytes) {
		return nil, ErrInvalidInput
	}
	rpk := make([]byte, cryptoBoxPublicKeyBytes)
	C.crypto_sign_ed25519_pk_to_curve25519((*C.uchar)(&rpk[0]), (*C.uchar)(&recipientPublicKey[0]))

	encryptedLength := len(encrypted)
	if !(encryptedLength > 0) {
		return nil, ErrInvalidInput
	}

	unencrypted = make([]byte, encryptedLength-cryptoBoxSealBytes)
	if int(C.crypto_box_seal_open(
		(*C.uchar)(unsafe.Pointer(&unencrypted[0])),
		(*C.uchar)(unsafe.Pointer(&encrypted[0])),
		(C.ulonglong)(encryptedLength),
		(*C.uchar)(&rpk[0]),
		(*C.uchar)(&rsk[0]))) != 0 {
		return nil, ErrSodium
	}

	return unencrypted, nil
}

// AuthenticatedEncrypt is
func (s *Sodium) AuthenticatedEncrypt(unencrypted []byte, senderSecretKey []byte, recipientPublicKey []byte, nonce []byte) (encrypted []byte, err error) {

	secretKeyLength := len(senderSecretKey)
	if !(secretKeyLength > 0 && secretKeyLength <= cryptoSignSecretKeyBytes) {
		return nil, ErrInvalidInput
	}
	ssk := make([]byte, cryptoBoxSecretKeyBytes)
	C.crypto_sign_ed25519_sk_to_curve25519((*C.uchar)(&ssk[0]), (*C.uchar)(&senderSecretKey[0]))

	publicKeyLength := len(recipientPublicKey)
	if !(publicKeyLength > 0 && publicKeyLength <= cryptoSignPublicKeyBytes) {
		return nil, ErrInvalidInput
	}
	rpk := make([]byte, cryptoBoxPublicKeyBytes)
	C.crypto_sign_ed25519_pk_to_curve25519((*C.uchar)(&rpk[0]), (*C.uchar)(&recipientPublicKey[0]))

	unencryptedLength := len(unencrypted)
	if !(unencryptedLength > 0) {
		return nil, ErrInvalidInput
	}

	nonceLength := len(nonce)
	if !(nonceLength > 0 && nonceLength <= cryptoBoxNonceBytes) {
		return nil, ErrInvalidInput
	}

	encrypted = make([]byte, unencryptedLength+cryptoBoxMacBytes)
	if int(C.crypto_box_easy(
		(*C.uchar)(&encrypted[0]),
		(*C.uchar)(unsafe.Pointer(&unencrypted[0])),
		(C.ulonglong)(unencryptedLength),
		(*C.uchar)(&nonce[0]),
		(*C.uchar)(&rpk[0]),
		(*C.uchar)(&ssk[0]))) != 0 {
		return nil, ErrSodium
	}

	return encrypted, nil
}

// AuthenticatedDecrypt is
func (s *Sodium) AuthenticatedDecrypt(encrypted []byte, recipientSecretKey []byte, senderPublicKey []byte, nonce []byte) (unencrypted []byte, err error) {

	secretKeyLength := len(recipientSecretKey)
	if !(secretKeyLength > 0 && secretKeyLength <= cryptoSignSecretKeyBytes) {
		return nil, ErrInvalidInput
	}
	rsk := make([]byte, cryptoBoxSecretKeyBytes)
	C.crypto_sign_ed25519_sk_to_curve25519((*C.uchar)(&rsk[0]), (*C.uchar)(&recipientSecretKey[0]))

	publicKeyLength := len(senderPublicKey)
	if !(publicKeyLength > 0 && publicKeyLength <= cryptoSignPublicKeyBytes) {
		return nil, ErrInvalidInput
	}
	spk := make([]byte, cryptoBoxPublicKeyBytes)
	C.crypto_sign_ed25519_pk_to_curve25519((*C.uchar)(&spk[0]), (*C.uchar)(&senderPublicKey[0]))

	encryptedLength := len(encrypted)
	if !(encryptedLength > 0) {
		return nil, ErrInvalidInput
	}

	unencrypted = make([]byte, encryptedLength-cryptoBoxMacBytes)
	if int(C.crypto_box_open_easy(
		(*C.uchar)(unsafe.Pointer(&unencrypted[0])),
		(*C.uchar)(unsafe.Pointer(&encrypted[0])),
		(C.ulonglong)(encryptedLength),
		(*C.uchar)(&nonce[0]),
		(*C.uchar)(&spk[0]),
		(*C.uchar)(&rsk[0]))) != 0 {
		return nil, ErrSodium
	}

	return unencrypted, nil
}

// Sign is
func (s *Sodium) Sign(message []byte, signerSecretKey []byte) (signed []byte, err error) {
	secretKeyLength := len(signerSecretKey)
	if !(secretKeyLength > 0 && secretKeyLength <= cryptoSignSecretKeyBytes) {
		return nil, ErrInvalidInput
	}

	messageLength := len(message)
	if !(messageLength > 0) {
		return nil, ErrInvalidInput
	}

	signed = make([]byte, secretKeyLength+cryptoSignBytes)
	var signedLength C.ulonglong

	if int(C.crypto_sign(
		(*C.uchar)(&signed[0]),
		&signedLength,
		(*C.uchar)(unsafe.Pointer(&message[0])),
		(C.ulonglong)(messageLength),
		(*C.uchar)(&signerSecretKey[0]))) != 0 {
		return nil, ErrSodium
	}
	signed = signed[:signedLength]

	return signed, nil
}

// VerifySignature is
func (s *Sodium) VerifySignature(signed []byte, signerPublicKey []byte) (ok bool, message []byte, err error) {
	publicKeyLength := len(signerPublicKey)
	if !(publicKeyLength > 0 && publicKeyLength <= cryptoSignPublicKeyBytes) {
		return false, nil, ErrInvalidInput
	}

	signedLength := len(signed)
	if !(signedLength > 0) {
		return false, nil, ErrInvalidInput
	}

	message = make([]byte, signedLength-cryptoSignBytes)
	var messageLength C.ulonglong

	if int(C.crypto_sign_open(
		(*C.uchar)(unsafe.Pointer(&message[0])),
		&messageLength,
		(*C.uchar)(unsafe.Pointer(&signed[0])),
		(C.ulonglong)(signedLength),
		(*C.uchar)(&signerPublicKey[0]))) != 0 {
		return true, nil, ErrSodium
	}
	message = message[:messageLength]

	return true, message, nil
}
