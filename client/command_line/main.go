package main

import (
	"fmt"

	"github.com/ockam-network/ockam"
	_ "github.com/pkg/errors"
)

func main() {
	fmt.Println(ockam.Version())
}
