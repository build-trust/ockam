package main

import (
	"fmt"
	"os"

	"github.com/ockam-network/ockam"
	_ "github.com/pkg/errors"
	"gopkg.in/alecthomas/kingpin.v2"
)

func main() {
	conf, err := newConfig()
	ifErrorThenExit(err)

	// if config file does not exist, assume this is the first invocation of this program
	// on the users machine
	if !configFileExists() {
		firstRun(conf)
	}

	app := kingpin.New("ockam", "A Command Line Interface to the Ockam.")
	app.Version(ockam.Version())
	app.HelpFlag.Short('h')

	attachClaimCommand(app, conf)
	attachEntityCommand(app, conf)

	kingpin.MustParse(app.Parse(os.Args[1:]))
}

// ifErrorThenExit checks if the provided error is nil
// if the error is not nil then it prints the error on stderr and calls
// os.Exit with exit status code 1 to exit the program.
func ifErrorThenExit(err error) {
	if err != nil {
		fmt.Fprintf(os.Stderr, "%+v\n", err) // nolint: errcheck
		os.Exit(1)
	}
}
