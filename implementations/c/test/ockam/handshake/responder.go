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




func xxRespond(transport prototype.Transport) (err error) {

	m1, err := transport.Receive()
	if err != nil {
		return err
	}
	fmt.Printf("-> M1 (Receive): %x\n", m1)
	reader := bytes.NewReader(m1)



	handshake := &Handshake{}

	// 1. Pick a static 25519 keypair for
	// this handshake and set it to s
	s, err := curve25519.GenerateKeyPair(hexReader("0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"))
	if err != nil {
		return err
	}
	handshake.s = s


	// 2. Generate an ephemeral 25519
	// keypair for this handshake and set
	// it to e
	e, err := curve25519.GenerateKeyPair(hexReader("4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60"))
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

	// 6. Read 32 bytes from the incoming
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

	// 7. read remaining message as payload
	// h = SHA256(h || payload),
	// payload should be empty
	payload := make([]byte, reader.Len())
	_, err = io.ReadFull(reader, payload)
	if  err != nil {
		return err
	}
	handshake.MixHash(payload)

	// ------------------------------------------------------------------------
		// M2 (Send)
		// <- e, ee, s, es

		writer := new(bytes.Buffer)

		// 1. h = SHA256(h || e.PublicKey),
		// Write e.PublicKey to outgoing message
		// buffer, BigEndian
		err = binary.Write(writer, binary.BigEndian, e.PublicKey)
		if err != nil {
			return err
		}
		handshake.MixHash(e.PublicKey)

		// 2. ck, k = HKDF(ck, DH(e, re), 2)
		// n = 0
		// handshake.MixKey(handshake.e.ComputeSharedSecret(handshake.re))
		k1, k2, err := Kdf(handshake.ck, handshake.e.ComputeSharedSecret(handshake.re))
		if err != nil {
			return err
		}
		handshake.ck = k1
		handshake.k = k2
		handshake.n = 0


		// 3. c = ENCRYPT(k, n++, h, s.PublicKey)
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

		// 4. ck, k = HKDF(ck, DH(s, re), 2)
		// n = 0
		k1, k2, err = Kdf(handshake.ck, handshake.s.ComputeSharedSecret(handshake.re))
		if err != nil {
			return err
		}
		handshake.ck = k1
		handshake.k = k2
		handshake.n = 0


		// 5. c = ENCRYPT(k, n++, h, payload)
		// h = SHA256(h || c),
		// payload is empty
		payload = []byte{}

		payloadEncrypted, err := Encrypt(handshake.k, handshake.n, handshake.h, payload)
		if err != nil {
			return err
		}
		handshake.n = handshake.n + 1
		handshake.MixHash(payloadEncrypted)

		err = binary.Write(writer, binary.BigEndian, payloadEncrypted)
		if err != nil {
			return err
		}


		m2 := writer.Bytes()

		err = transport.Send(m2)
		if err != nil {
			return err
		}




	// ------------------------------------------------------------------------
	// M3 (Receive)
	// -> s, se

	m3, err := transport.Receive()
	if err != nil {
		return err
	}
	fmt.Printf("-> M3 (Receive): %x\n", m3)

	reader = bytes.NewReader(m3)

	// 1. Read 48 bytes the incoming
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

	// 2. ck, k = HKDF(ck, DH(e, rs), 2)
	// n = 0
	k1, k2, err = Kdf(handshake.ck, handshake.e.ComputeSharedSecret(handshake.rs))
	if err != nil {
		return err
	}
	handshake.ck = k1
	handshake.k = k2
	handshake.n = 0

	// 3. Read remaining bytes of incoming
	// message buffer as c
	// p = DECRYPT(k, n++, h, c)
	// h = SHA256(h || c),
	// parse p as a payload,
	// payload should be empty
	payloadEncrypted = make([]byte, reader.Len())
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


	k1, k2, err = Kdf(handshake.ck, []byte{})
	if err != nil {
		return err
	}

	fmt.Printf("\n   k1: %x\n", k1)
	fmt.Printf("   k2: %x\n\n", k2)
	var n1, n2 uint64
	n1 = 0
	n2 = 0


	payload, err = hex.DecodeString("79656c6c6f777375626d6172696e65")
	if err != nil {
		return err
	}

	payloadEncrypted, err = Encrypt(k1, n1, []byte{}, payload)
	if err != nil {
		return err
	}
	n1 = n1 + 1

	m4 := payloadEncrypted

	err = transport.Send(m4)
	if err != nil {
		return err
	}

	fmt.Printf("<- M4 (Send): %x\n", m4)


	m5Encrypted, err := transport.Receive()
	if err != nil {
		return err
	}
	fmt.Printf("-> M5 (Receive): %x\n", m5Encrypted)

	m5, err := Decrypt(k2, n2, []byte{}, m5Encrypted)
	if err != nil {
		return err
	}

	fmt.Printf("-> M5 (Receive - Decrypted): %x\n", m5)





	return nil
}

func main() {
	err := socket.SetupServer("unix", "/tmp/xx.socket", xxRespond)
	if err != nil {
		log.Fatal(err)
	}
}
