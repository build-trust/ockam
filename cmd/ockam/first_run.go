package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"
)

func firstRun(c *config) {
	fmt.Print(welcomeMessage)

	c.UserName = ask("\nWhat is your full name?\n> ")
	c.UserEmail = ask("\nWhat is your email address?\n> ")
	c.UserOrganization = ask("\nWhere do you work?\n> ")
	c.UserReason = ask("\nWhy are you interested in Ockam?\n> ")

	fmt.Print("\nThank you.\n\n")

	err := c.save()
	ifErrorThenExit(err)
}

func ask(prompt string) string {
	fmt.Print(prompt)

	input, err := bufio.NewReader(os.Stdin).ReadString('\n')
	ifErrorThenExit(err)

	return strings.TrimSpace(input)
}

const welcomeMessage = `
Welcome to Ockam!

It looks like this is the first time you’ve run the ockam command.

Ockam is in early development, please help us improve by
answering a few quick questions.
`
