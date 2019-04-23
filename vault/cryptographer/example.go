package main

import (
	"fmt"
	"log"

	"github.com/ockam-network/ockam/vault/cryptographer/sodium"
)

func main() {
	cryptographer, err := sodium.NewSodium()
	if err != nil {
		log.Fatal(err)
	}

	senderPublicKey, senderSecretKey, err := cryptographer.GenerateKeyPair()
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("senderPublicKey: %+v\n", senderPublicKey)
	fmt.Printf("senderSecretKey: %+v\n", senderSecretKey)

	recipientPublicKey, recipientSecretKey, err := cryptographer.GenerateKeyPair()
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("recipientPublicKey: %+v\n", recipientPublicKey)
	fmt.Printf("recipientSecretKey: %+v\n", recipientSecretKey)

	message := []byte("hello world")
	fmt.Printf("message: %+v\n", message)

	encryptedMessage, err := cryptographer.AnonymousEncrypt(message, recipientPublicKey)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("encryptedMessage: %+v\n", encryptedMessage)

	decryptedMessage, err := cryptographer.AnonymousDecrypt(encryptedMessage, recipientSecretKey)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("decryptedMessage: %+v\n", decryptedMessage)

	encryptedMessage, err = cryptographer.AuthenticatedEncrypt(message, senderSecretKey, recipientPublicKey, []byte("1"))
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("encryptedMessage: %+v\n", encryptedMessage)

	decryptedMessage, err = cryptographer.AuthenticatedDecrypt(encryptedMessage, recipientSecretKey, senderPublicKey, []byte("1"))
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("decryptedMessage: %+v\n", decryptedMessage)

	signedMessage, err := cryptographer.Sign(message, senderSecretKey)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("signedMessage: %+v\n", signedMessage)

	ok, message, err := cryptographer.VerifySignature(signedMessage, senderPublicKey)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("ok: %+v\n", ok)
	fmt.Printf("message: %+v\n", message)

}
