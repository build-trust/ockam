package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/pkg/errors"
	"gopkg.in/alecthomas/kingpin.v2"
)

// claim command

func attachClaimCommand(app *kingpin.Application, conf *config) {
	claimCommand := app.Command("claim", "Manage Claims.")
	attachClaimAddCommand(claimCommand, conf)
	attachClaimShowCommand(claimCommand, conf)
}

// claim add command

type claimAddCommand struct {
	body   string
	config *config
}

func (c *claimAddCommand) runClaimAddCommand(_ *kingpin.ParseContext) error {
	body := c.body
	if body == "" {
		input, err := readStandardInput()
		if err != nil {
			return err
		}
		body = input
	}

	var b map[string]interface{}
	err := json.Unmarshal([]byte(body), &b)
	if err != nil {
		return errors.WithStack(err)
	}

	fmt.Printf("%+v", b)

	return nil
}

func readStandardInput() (string, error) {
	var lines []string

	info, err := os.Stdin.Stat()
	if err != nil {
		return "", errors.WithStack(err)
	}

	if info.Mode()&os.ModeCharDevice != 0 || info.Size() <= 0 {
		return "", errors.New("no body")
	}

	scanner := bufio.NewScanner(os.Stdin)
	scanner.Split(bufio.ScanLines)
	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}
	err = scanner.Err()
	if err != nil {
		return "", errors.WithStack(err)
	}

	return strings.Join(lines, "\n"), nil
}

// nolint: dupl
func attachClaimAddCommand(p *kingpin.CmdClause, conf *config) {
	c := &claimAddCommand{config: conf}
	cmd := p.Command("add", "Add Claim.").Action(c.runClaimAddCommand)
	cmd.Arg("body", "Body of the claim in JSON").StringVar(&c.body)
}

// claim show command

type claimShowCommand struct {
	config *config
}

func (c *claimShowCommand) runClaimShowCommand(_ *kingpin.ParseContext) error {
	return nil
}

func attachClaimShowCommand(parent *kingpin.CmdClause, conf *config) {
	c := &claimShowCommand{config: conf}
	parent.Command("show", "Show Claim.").Action(c.runClaimShowCommand)
}
