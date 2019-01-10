package main

import (
	kingpin "gopkg.in/alecthomas/kingpin.v2"
)

type registerCommand struct {
	config *config
	alias  string
}

func (c *registerCommand) run(ctx *kingpin.ParseContext) error {
	ec := &entityAddCommand{
		config: c.config,
		alias:  c.alias,
	}
	return ec.run(ctx)
}

func attachRegisterCommand(app *kingpin.Application, conf *config) {
	c := &registerCommand{config: conf}
	cmd := app.Command("register", "Register this device.").Action(c.run)
	cmd.Arg("alias", "Alias for the entity.").StringVar(&c.alias)
}
