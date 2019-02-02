// +build ignore

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"

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
		entity.Attributes{
			"name":         "Temperature Sensor",
			"manufacturer": "Element 14",
			"model":        "Raspberry Pi 3 Model B+",
		},
		entity.Signer(signer),
	)
	exitOnError(err)

	// register the ockam
	registrationClaim, err := ockamChain.Register(temperatureSensor)
	exitOnError(err)

	// turn the claim into json so we can print it
	claimJson, err := registrationClaim.MarshalJSON()
	exitOnError(err)

	// print the claim
	err = printJson(claimJson)
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
