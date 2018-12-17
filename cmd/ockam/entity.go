package main

import (
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
