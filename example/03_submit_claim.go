// +build ignore

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"time"

	"github.com/ockam-network/ockam/claim"
	"github.com/ockam-network/ockam/entity"
	"github.com/ockam-network/ockam/key/ed25519"
	"github.com/ockam-network/ockam/node"
	"github.com/ockam-network/ockam/node/remote/http"
)

func main() {
	// create a local ockam node and give it a way to find peers on the ockam test network
	ockamNode, err := node.New(node.PeerDiscoverer(http.Discoverer("test.ockam.network", 26657)))
	exitOnError(err)

	// ask the node to find peers and sync with network state
	err = ockamNode.Sync()
	exitOnError(err)

	// get a reference to the chain the node is synced to
	ockamChain := ockamNode.Chain()
	fmt.Printf("Chain ID: %s\n", ockamChain.ID())

	// create a new ed25519 signer
	signer, err := ed25519.New()
	exitOnError(err)

	// create a new ockam entity to represent a temperature sensor
	temperatureSensor, err := entity.New(
		entity.Attributes{"name": "Temperature Sensor"},
		entity.Signer(signer),
	)
	exitOnError(err)

	// create a temperature claim with this new sensor entity as both the issuer and the subject of the claim
	temperatureClaim, err := claim.New(
		claim.Data{"temperature": 100},
		claim.Issuer(temperatureSensor),
		claim.Subject(temperatureSensor),
	)
	exitOnError(err)

	// submit the claim
	err = ockamChain.Submit(temperatureClaim)
	exitOnError(err)

	fmt.Printf("Submitted - %s\n", temperatureClaim.ID())
	time.Sleep(5 * time.Second)

	bytes, _, err := ockamChain.FetchClaim(temperatureClaim.ID())
	exitOnError(err)

	fmt.Println("Fetched claim:")
	err = printJson(bytes)
	exitOnError(err)
}

func exitOnError(err error) {
	if err != nil {
		fmt.Fprintf(os.Stderr, "%+v\n", err)
		os.Exit(1)
	}
}

func printJson(j []byte) error {
	var prettyJSON bytes.Buffer
	err := json.Indent(&prettyJSON, j, "", "\t")
	if err != nil {
		return err
	}
	fmt.Printf("%s", string(prettyJSON.Bytes()))
	return nil
}
