// +build ignore

package main

import (
	"fmt"
	"os"

	"github.com/ockam-network/ockam/node"
	"github.com/ockam-network/ockam/node/remote/http"
)

// This example shows how a device can connect to the Ockam Testnet.

func main() {
	// create a local ockam node and give it a way to find peers on the ockam test network
	ockamNode, err := node.New(node.PeerDiscoverer(http.Discoverer("test.ockam.network", 26657)))
	exitOnError(err)

	// ask the local node to find peers and sync with network state
	err = ockamNode.Sync()
	exitOnError(err)

	// get a reference to the chain that is being managed by the distributed ockam network
	// that the local node is connected to
	ockamChain := ockamNode.Chain()

	// print some information about the chain
	fmt.Printf("Chain ID: %s\n", ockamChain.ID())
	fmt.Printf("Latest Block Height: %s\n", ockamChain.LatestBlock().Height())
	fmt.Printf("Latest Block Hash: %s\n", ockamChain.LatestBlock().Hash())
}

func exitOnError(err error) {
	if err != nil {
		fmt.Fprintf(os.Stderr, "%+v\n", err)
		os.Exit(1)
	}
}
