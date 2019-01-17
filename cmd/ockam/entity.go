package main

import (
	"encoding/hex"
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"

	"github.com/ockam-network/ockam/entity"
	"github.com/ockam-network/ockam/key/ed25519"
	"github.com/ockam-network/ockam/node"
	"github.com/ockam-network/ockam/node/remote/http"
	"github.com/pkg/errors"
	kingpin "gopkg.in/alecthomas/kingpin.v2"
)

// entity command

func attachEntityCommand(app *kingpin.Application, conf *config) {
	entityCommand := app.Command("entity", "Manage Entities.")
	attachEntityAddCommand(entityCommand, conf)
	attachEntityShowCommand(entityCommand, conf)
}

// entity add command

type entityAddCommand struct {
	alias  string
	config *config
}

func (c *entityAddCommand) run(_ *kingpin.ParseContext) error {
	alias := c.alias
	if alias == "" {
		alias = "host"
	}

	aliasDir := filepath.Join(dataHome(), "alias")
	aliasPath := filepath.Join(aliasDir, alias)
	_, err := os.Stat(aliasPath)
	if !os.IsNotExist(err) {
		fmt.Fprintf(os.Stderr, white("%s already exists\n"), aliasPath)
		os.Exit(1)
	}

	fmt.Fprintf(os.Stderr, white("Generating a new Ockam Entity for %s ...\n"), alias)

	fmt.Fprintf(os.Stderr, white("\nGenerating a new Ed25519 signer for %s entity ... "), alias)
	signer, err := ed25519.New()
	ifErrorThenExit(c.config.Logger, err)
	fmt.Fprintf(os.Stderr, grey("\n%s: %s\n"), signer.PublicKey().Encoding(), signer.PublicKey().Value())

	fmt.Fprintf(os.Stderr, white("\nGenerating a new Ockam ID for %s entity ... "), alias)
	// create a new ockam entity to represent a temperature sensor
	newEntity, err := entity.New(
		entity.Attributes{},
		entity.Signer(signer),
	)
	ifErrorThenExit(c.config.Logger, err)
	fmt.Fprintf(os.Stderr, grey("\n%s\n"), newEntity.ID())

	newEntityPath := filepath.Join(dataHome(), "entity", newEntity.ID().String())
	err = os.MkdirAll(newEntityPath, 0755)
	ifErrorThenExit(c.config.Logger, errors.WithStack(err))
	privateKey := []byte(hex.EncodeToString(signer.PrivateKey()))
	privateKeyPath := filepath.Join(newEntityPath, newEntity.Signers()[0].PublicKey().Label())
	err = ioutil.WriteFile(privateKeyPath, privateKey, 0644)
	ifErrorThenExit(c.config.Logger, errors.WithStack(err))

	err = os.MkdirAll(aliasDir, 0755)
	ifErrorThenExit(c.config.Logger, errors.WithStack(err))
	err = ioutil.WriteFile(aliasPath, []byte(newEntity.ID().String()), 0644)
	ifErrorThenExit(c.config.Logger, errors.WithStack(err))

	fmt.Fprintf(os.Stderr, white("\nNew Entity:\n"))

	fmt.Fprintf(os.Stderr, white("  ID: "))
	fmt.Fprintf(os.Stderr, grey("%s\n"), newEntity.ID())

	fmt.Fprintf(os.Stderr, white("  Aliases: "))
	fmt.Fprintf(os.Stderr, grey("['%s', '%s']\n"), newEntity.ID(), alias)

	fmt.Fprintf(os.Stderr, white("  Local Path: "))
	fmt.Fprintf(os.Stderr, grey("'%s'\n"), newEntityPath)

	fmt.Fprintf(os.Stderr, white("  Keys:\n"))

	fmt.Fprintf(os.Stderr, white("    - Public: "))
	fmt.Fprintf(os.Stderr, grey("(%s) %s\n"), signer.PublicKey().Encoding(), signer.PublicKey().Value())

	fmt.Fprintf(os.Stderr, white("      Private: "))
	fmt.Fprintf(os.Stderr, grey("'%s'\n"), privateKeyPath)

	fmt.Fprintf(os.Stderr, green("\nGenerated %s ✔\n"), newEntity.ID())

	fmt.Fprintf(os.Stderr, white("\nStarting local Ockam node ..."))
	// create a local ockam node and give it a way to find peers on the ockam test network
	ockamNode, err := node.New(node.PeerDiscoverer(http.Discoverer("test.ockam.network", 26657)))
	ifErrorThenExit(c.config.Logger, err)

	// ask the node to find peers and sync with network state
	err = ockamNode.Sync()
	ifErrorThenExit(c.config.Logger, err)
	fmt.Fprintf(os.Stderr, white("\nDiscovering peers and syncing with network ... "))

	// get a reference to the chain the node is synced to
	ockamChain := ockamNode.Chain()
	fmt.Fprintf(os.Stderr, grey("\nChain ID: %s\n"), ockamChain.ID())

	fmt.Fprintf(os.Stderr, white("\nCreating a new registration claim for %s ...\n"), newEntity.ID())

	fmt.Fprintf(os.Stderr, white("\nSigning the registration claim with key %s ...\n"), signer.PublicKey().Value())

	registerationClaim, err := ockamChain.Register(newEntity)
	ifErrorThenExit(c.config.Logger, err)
	fmt.Fprintf(os.Stderr, grey("Signed %s\n"), registerationClaim.ID())

	fmt.Fprintf(os.Stderr, white("\nSubmitting registration claim to %s \n"), ockamChain.ID())

	// todo: confirm commit

	fmt.Fprintf(os.Stderr, green("\nRegistered %s ✔\n"), newEntity.ID())

	return nil
}

// nolint: dupl
func attachEntityAddCommand(p *kingpin.CmdClause, conf *config) {
	c := &entityAddCommand{config: conf}
	cmd := p.Command("add", "Add Entity.").Action(c.run)
	cmd.Arg("alias", "Alias for the entity.").StringVar(&c.alias)
}

// entity show command

type entityShowCommand struct {
	idOrAlias string
	config    *config
}

func (c *entityShowCommand) run(_ *kingpin.ParseContext) error {
	println(c.idOrAlias)
	return nil
}

// nolint: dupl
func attachEntityShowCommand(parent *kingpin.CmdClause, conf *config) {
	c := &entityShowCommand{config: conf}
	cmd := parent.Command("show", "Show Entity.").Action(c.run)
	cmd.Arg("id | alias", "Id or alias of the entity that you wish to show.").StringVar(&c.idOrAlias)
}
