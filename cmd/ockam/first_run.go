package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"strings"
	"time"
)

const reqURL = "https://ockam.network/upload"

func firstRun(c *config) {
	fmt.Print(welcomeMessage)

	c.UserName = ask(c, "Name: ")
	c.UserEmail = ask(c, "Email: ")
	c.UserOrganization = ask(c, "Organization: ")
	c.UserReason = ask(c, "Your use case or interest in Ockam: ")

	fmt.Print("\nThank you.\n\n")

	err := c.save()
	ifErrorThenExit(c.Logger, err)

	uploadUserInfo(c)
}

func ask(c *config, prompt string) string {
	fmt.Print(prompt)

	input, err := bufio.NewReader(os.Stdin).ReadString('\n')
	ifErrorThenExit(c.Logger, err)

	return strings.TrimSpace(input)
}

// We don't want errors in this function to interrupt users, so ignore them
// nolint: errcheck, gosec
func uploadUserInfo(c *config) {
	marshalled, _ := json.Marshal(c)

	client := &http.Client{
		Timeout: 10 * time.Second,
	}
	_, _ = client.Post(reqURL, "application/json", bytes.NewBuffer(marshalled))
}

const welcomeMessage = `
Welcome to Ockam.

This message appears when you run the ockam command for the first time.

The ockam command connects to the Ockam Testnet, which is maintained by the
Ockam team. For current status of the Testnet see https://ockam.network

Please join The Ockam Community on Slack for questions and support
https://bit.ly/2CXw4PD

If your application requires a production network, please contact the
Ockam team at hello@ockam.io

For us to better support you with information and updates, please provide:
`
