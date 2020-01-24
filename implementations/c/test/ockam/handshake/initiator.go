package main

import (
	"bytes"
	"crypto/aes"
	"crypto/cipher"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"io"
	"log"
	"prototype"
	"prototype/curve25519"
	"prototype/transport/socket"

	"golang.org/x/crypto/hkdf"
)

func hexReader(s string) io.Reader {
	decoded, err := hex.DecodeString(s)
	if err != nil {
		panic(err)
	}

	return bytes.NewBuffer(decoded)
}

type Handshake struct {
	h  []byte
	ck []byte

	k  []byte
	n  uint64

	e  *curve25519.KeyPair
	s  *curve25519.KeyPair

	rs curve25519.PublicKey
	re curve25519.PublicKey
}

func (s *Handshake) MixHash(b []byte) {
	h := sha256.Sum256(append(s.h, b...))
	s.h = h[:]
}

func Kdf(salt, inputKeyMaterial []byte) ([]byte, []byte, error) {
	info := []byte{}
	f := hkdf.New(sha256.New, inputKeyMaterial, salt, info)

	k1 := make([]byte, 32)
	if _, err := io.ReadFull(f, k1); err != nil {
		return nil, nil, err
	}

	k2 := make([]byte, 32)
	if _, err := io.ReadFull(f, k2); err != nil {
		return nil, nil, err
	}

	return k1, k2, nil
}

func Encrypt(k []byte, n uint64, ad []byte, plaintext []byte) ([]byte, error) {
	c, err := aes.NewCipher(k[:])
	if err != nil {
		return nil, err
	}

	gcm, err := cipher.NewGCM(c)
	if err != nil {
		return nil, err
	}

	// The 96-bit nonce is formed by encoding 32 bits of zeros followed by big-endian encoding of n.
	var nonce [12]byte
	binary.BigEndian.PutUint64(nonce[4:], n)

	return gcm.Seal(plaintext[:0], nonce[:], plaintext, ad), nil
}

func Decrypt(k []byte, n uint64, ad[]byte, ciphertext []byte) ([]byte, error) {
	ct := make([]byte, len(ciphertext))
	copy(ct, ciphertext)

	c, err := aes.NewCipher(k[:])
	if err != nil {
		return nil, err
	}

	gcm, err := cipher.NewGCM(c)
	if err != nil {
		return nil, err
	}

	// The 96-bit nonce is formed by encoding 32 bits of zeros followed by big-endian encoding of n.
	var nonce [12]byte
	binary.BigEndian.PutUint64(nonce[4:], n)

	return gcm.Open(ct[:0], nonce[:], ct, ad)
}

func xxInitiate(transport prototype.Transport) (err error) {
	handshake := &Handshake{}


	writer := new(bytes.Buffer)

	// 1. Pick a static 25519 keypair for
	// this handshake and set it to s
	s, err := curve25519.GenerateKeyPair(hexReader("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"))
	if err != nil {
		return err
	}
	handshake.s = s

	// 2. Generate an ephemeral 25519
	// keypair for this handshake and set
	// it to e
	e, err := curve25519.GenerateKeyPair(hexReader("202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f"))
	if err != nil {
		return err
	}
	handshake.e = e

	// 3. Set k to empty, Set n to 0
	handshake.k = []byte{}
	handshake.n = 0

	// 4. Set h and ck to
	// 'Noise_XX_25519_AESGCM_SHA256'
	name := []byte("Noise_XX_25519_AESGCM_SHA256")

	handshake.h = make([]byte, sha256.Size)
	copy(handshake.h, name)

	handshake.ck = make([]byte, sha256.Size)
	copy(handshake.ck, name)

	// 5. h = SHA256(h || prologue),
	// prologue is empty
	prologue := []byte{}
	handshake.MixHash(prologue)

	// 6. h = SHA256(h || e.PublicKey),
	// Write e.PublicKey to outgoing message
	// buffer, BigEndian
	handshake.MixHash(e.PublicKey)
	err = binary.Write(writer, binary.BigEndian, e.PublicKey)
	if err != nil {
		return err
	}

	// 7. h = SHA256(h || payload),
	// payload is empty
	payload := []byte{}
	handshake.MixHash(payload)
	err = binary.Write(writer, binary.BigEndian, payload)
	if err != nil {
		return err
	}

	m1 := writer.Bytes()
	err = transport.Send(m1)
	if err != nil {
		return err
	}

	fmt.Printf("-> M1 (Send): %x\n", m1)

	// ------------------------------- m2


	m2, err := transport.Receive()
	if err != nil {
		return err
	}
	fmt.Printf("<- M2(Receive): %x\n", m2)

	reader := bytes.NewReader(m2)

	// 1. Read 32 bytes from the incoming
	// message buffer, parse it as a public
	// key, set it to re
	// h = SHA256(h || re)

	reBytes := make([]byte, 32) // DHLEN
	_, err = io.ReadFull(reader, reBytes)
	if  err != nil {
		return err
	}
	handshake.MixHash(reBytes)

	re, err := curve25519.UnmarshallPublicKey(reBytes)
	if  err != nil {
		return err
	}
	handshake.re = re

	// 2. ck, k = HKDF(ck, DH(e, re), 2)
	// n = 0
	k1, k2, err := Kdf(handshake.ck, handshake.e.ComputeSharedSecret(handshake.re))
	if err != nil {
		return err
	}
	handshake.ck = k1
	handshake.k = k2
	handshake.n = 0

	// 3. Read 48 bytes the incoming
	// message buffer as c
	// p = DECRYPT(k, n++, h, c)
	// h = SHA256(h || c),
	// parse p as a public key,
	// set it to rs

	rsBytesEncrypted := make([]byte, 32+16) // DHLEN+16
	_, err = io.ReadFull(reader, rsBytesEncrypted)
	if  err != nil {
		return err
	}

	rsBytes, err := Decrypt(handshake.k, handshake.n, handshake.h, rsBytesEncrypted)
	if err != nil {
		return err
	}
	handshake.n = handshake.n + 1
	handshake.MixHash(rsBytesEncrypted)

	rs, err := curve25519.UnmarshallPublicKey(rsBytes)
	if  err != nil {
		return err
	}
	handshake.rs = rs

	// 4. ck, k = HKDF(ck, DH(e, rs), 2)
	// n = 0
	k1, k2, err = Kdf(handshake.ck, handshake.e.ComputeSharedSecret(handshake.rs))
	if err != nil {
		return err
	}
	handshake.ck = k1
	handshake.k = k2
	handshake.n = 0

	// 5. Read remaining bytes of incoming
	// message buffer as c
	// p = DECRYPT(k, n++, h, c)
	// h = SHA256(h || c),
	// parse p as a payload,
	// payload should be empty
	payloadEncrypted := make([]byte, reader.Len())
	_, err = io.ReadFull(reader, payloadEncrypted)
	if  err != nil {
		return err
	}

	_, err = Decrypt(handshake.k, handshake.n, handshake.h, payloadEncrypted)
	if err != nil {
		return err
	}
	handshake.n = handshake.n + 1
	handshake.MixHash(payloadEncrypted)

	// ------------------------------- m3


	writer = new(bytes.Buffer)

	// 1. c = ENCRYPT(k, n++, h, s.PublicKey)
	// h =  SHA256(h || c),
	// Write c to outgoing message
	// buffer, BigEndian
	sBytesEncrypted, err := Encrypt(handshake.k, handshake.n, handshake.h, handshake.s.PublicKey)
	if err != nil {
		return err
	}
	handshake.n = handshake.n + 1
	handshake.MixHash(sBytesEncrypted)

	err = binary.Write(writer, binary.BigEndian, sBytesEncrypted)
	if err != nil {
		return err
	}

	// 2. ck, k = HKDF(ck, DH(s, re), 2)
	// n = 0
	k1, k2, err = Kdf(handshake.ck, handshake.s.ComputeSharedSecret(handshake.re))
	if err != nil {
		return err
	}
	handshake.ck = k1
	handshake.k = k2
	handshake.n = 0


	// 3. c = ENCRYPT(k, n++, h, payload)
	// h = SHA256(h || c),
	// payload is empty
	payload = []byte{}

	payloadEncrypted, err = Encrypt(handshake.k, handshake.n, handshake.h, payload)
	if err != nil {
		return err
	}
	handshake.n = handshake.n + 1
	handshake.MixHash(payloadEncrypted)

	err = binary.Write(writer, binary.BigEndian, payloadEncrypted)
	if err != nil {
		return err
	}

	m3 := writer.Bytes()

	err = transport.Send(m3)
	if err != nil {
		return err
	}

	fmt.Printf("-> M3 (Send): %x\n", m3)


	k1, k2, err = Kdf(handshake.ck, []byte{})
	if err != nil {
		return err
	}

	fmt.Printf("\n   k1: %x\n", k1)
	fmt.Printf("   k2: %x\n\n", k2)
	var n1, n2 uint64
	n1 = 0
	n2 = 0


	m4Encrypted, err := transport.Receive()
	if err != nil {
		return err
	}
	fmt.Printf("<- M4 (Receive): %x\n", m4Encrypted)

	m4, err := Decrypt(k1, n1, []byte{}, m4Encrypted)
	if err != nil {
		return err
	}

	fmt.Printf("<- M4 (Receive - Decrypted): %x\n", m4)

	payload, err = hex.DecodeString("7375626d6172696e6579656c6c6f77")
	if err != nil {
		return err
	}

	payloadEncrypted, err = Encrypt(k2, n2, []byte{}, payload)
	if err != nil {
		return err
	}
	n1 = n1 + 1

	m5 := payloadEncrypted

	err = transport.Send(m5)
	if err != nil {
		return err
	}

	fmt.Printf("-> M5 (Send): %x\n", m5)

	return nil
}

func main() {
	err := socket.SetupClient("unix", "/tmp/xx.socket", xxInitiate)
	if err != nil {
		log.Fatal(err)
	}
}
