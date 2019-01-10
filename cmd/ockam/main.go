package main

import (
	"os"

	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/log"
	_ "github.com/pkg/errors"
	"gopkg.in/alecthomas/kingpin.v2"
)

func main() {
	logger := log.New(
		log.Level(log.Notice),
		log.Formatter(&log.ConsoleFormatter{Colored: true}),
	)

	conf, err := newConfig()
	ifErrorThenExit(logger, err)

	conf.Logger = logger

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
	attachRegisterCommand(app, conf)

	kingpin.MustParse(app.Parse(os.Args[1:]))
}

// ifErrorThenExit checks if the provided error is nil
// if the error is not nil then it prints the error on stderr and calls
// os.Exit with exit status code 1 to exit the program.
func ifErrorThenExit(logger ockam.Logger, err error) {
	if err != nil {
		logger.Error("%+v", err)
		os.Exit(1)
	}
}
