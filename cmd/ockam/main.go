package main

import (
	"fmt"
	"os"

	_ "github.com/ockam-network/ockam"
	_ "github.com/pkg/errors"
)

func main() {
	conf, err := newConfig()
	ifErrorThenExit(err)

	fmt.Printf("%+v\n", conf)
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
