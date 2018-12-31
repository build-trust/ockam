package main

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"log"
	"os"
	"strconv"
	"strings"
)

// TxHash is
func TxHash(tx string) string {
	h := sha256.Sum256([]byte(tx))
	return strings.ToUpper(hex.EncodeToString(h[:]))
}

func exitOnError(err error) {
	if err != nil {
		log.Fatal(err)
	}
}

func main() {
	input := os.Args[1]
	fmt.Printf("Calculated TxHash: %s\n", TxHash(input))

	n, err := NewNode(Address("40.87.50.217:26657"))
	exitOnError(err)

	dp, err := NewDiskProvider(n)
	exitOnError(err)

	verifier := NewVerifier(dp)

	txHash, err := n.BroadcastTxSync(input)
	exitOnError(err)
	fmt.Printf("Returned TxHash: %s\n", txHash)

	tx, err := n.WaitForTxHeight(txHash)
	exitOnError(err)
	fmt.Printf("TxHeight: %s\n", tx.Height)

	merkleProof := tx.Proof.Proof

	err = merkleProof.Verify(tx.Proof.RootHash, tx.Proof.Proof.LeafHash)
	if err != nil {
		fmt.Println("proof not verified")
	} else {
		fmt.Println("proof verified")
	}

	commit, err := n.Commit(tx.Height)
	exitOnError(err)
	fmt.Printf("Commit At TxHeight ValidatorsHash %+v\n", commit.SignedHeader.Header.ValidatorsHash)

	//verifier.Verify checks to make sure the val set is trusted and that
	// 2/3 of the trusted set have signed the commit
	isVerified, err := verifier.Verify(commit, n)
	exitOnError(err)

	fmt.Println("Is Verified?  ", isVerified)

	h, err := strconv.ParseInt(tx.Height, 10, 64)
	exitOnError(err)

	commit1, err := n.Commit(strconv.FormatInt(h-1, 10))
	exitOnError(err)
	fmt.Printf("Commit At TxHeight-1 NextValidatorsHash %+v\n", commit1.SignedHeader.Header.NextValidatorsHash)

}
