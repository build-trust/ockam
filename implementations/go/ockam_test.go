package ockam

import (
	"fmt"
	"path/filepath"
	"reflect"
	"runtime"
	"testing"
)

func TestGenerateKeyPair(t *testing.T) {
	t.Run("generates", func(t *testing.T) {
		assert(t, true, GenerateKeyPair() == "a key")
	})
}

func assert(t *testing.T, expected interface{}, actual interface{}, args ...interface{}) {
	if !reflect.DeepEqual(expected, actual) {
		argsLength := len(args)
		var message string

		// if only one arg is present, treat it as the message
		if argsLength == 1 {
			message = args[0].(string)
		}

		// if more than one arg is present, treat it as format, args (like Printf)
		if argsLength > 1 {
			message = fmt.Sprintf(args[0].(string), args[1:]...)
		}

		// is message is not empty add some spacing
		if message != "" {
			message = "\t" + message + "\n\n"
		}

		_, file, line, _ := runtime.Caller(1)
		fmt.Printf("%s:%d:\n\tExpected: %#v\n\tActual: %#v\n%s", filepath.Base(file), line, expected, actual, message)
		t.FailNow()
	}
}
