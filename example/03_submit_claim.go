// +build ignore

package main

import (
	"fmt"
	"os"

	"github.com/ockam-network/ockam/claim"
	"github.com/ockam-network/ockam/entity"
	"github.com/ockam-network/ockam/key/ed25519"
	"github.com/ockam-network/ockam/node"
	"github.com/ockam-network/ockam/node/remote/http"
)

func main() {

	// create a local ockam node and give it a way to find peers on the ockam network
	ockamNode, err := node.New(node.PeerDiscoverer(http.Discoverer("seeds.ockam.network", 26657)))
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
	temperatureSensor, err := entity.New(entity.Signer(signer))
	exitOnError(err)

	// create a temperature claim with this new sensor entity as both the issuer and the subject of the claim
	temperatureClaim, err := claim.New(
		claim.Data{"temperature": 100},
		claim.Issuer(temperatureSensor),
		claim.Subject(temperatureSensor),
	)
	exitOnError(err)

	// submit the claim to be
	err = ockamChain.Submit(temperatureClaim)
	exitOnError(err)

	fmt.Printf("Submitted - " + temperatureClaim.ID())
}

func exitOnError(err error) {
	if err != nil {
		fmt.Fprintf(os.Stderr, "%+v\n", err)
		os.Exit(1)
	}
}
