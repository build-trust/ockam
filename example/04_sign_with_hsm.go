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
	"github.com/ockam-network/ockam/key/pkcs11"
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

	// create a new pkcs11 signer
	// this step assumes you've installed the YubiHSM2 SDK and are running
	// a yubihsm-connector process
	// for more information see https://developers.yubico.com/YubiHSM2/Usage_Guides/YubiHSM_quick_start_tutorial.html
	signer, err := pkcs11.New(

		// install YubiHSM2 https://developers.yubico.com/YubiHSM2/Releases/
		pkcs11.ModulePath("/usr/lib/x86_64-linux-gnu/pkcs11/yubihsm_pkcs11.so"),
		pkcs11.PublicKey(pubKey),
		// returned from running `pkcs11-tool --list-slots --module=/usr/lib/x86_64-linux-gnu/pkcs11/yubihsm_pkcs11.so`
		pkcs11.TokenLabel("02"))
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

const pubKey = `-----BEGIN PGP PUBLIC KEY BLOCK-----

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
GdD+h9o+RgRpGSaT0dOeqRrn1yL1knBFfblek90=
=xHWB`
