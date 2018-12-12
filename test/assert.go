package test

import (
	"fmt"
	"path/filepath"
	"reflect"
	"runtime"
)

// FailNower interface is implemented by any value that has a FailNow() method.
// This is typically values of type testing.T, testing.B etc.
type FailNower interface {
	FailNow()
}

// The below Assert function is inspired from https://github.com/benbjohnson/testing
// which is licenced under MIT License, see NOTICE file for attribution

// Assert is a test helper that checks if the actual value is equal to the expected value.
// The test fails if the two values are not deeply equal.
//
// The first argument is a value that implementes the FailNower interface, this is typically testing.T
// The second argument is the expected value as expected by the test
// The third argument is the actual value observed by the test
// The fourth argument is the message string to display if the assertion fails, this may be a format string
// The remaining arguments are treated as arguments to fmt.Printf, assuming the message is a format string
func Assert(t FailNower, expected interface{}, actual interface{}, args ...interface{}) {
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

		// if message is not empty add some spacing
		if message != "" {
			message = "\t" + message + "\n\n"
		}

		_, file, line, _ := runtime.Caller(1)
		fmt.Printf("%s:%d:\n\tExpected: %#v\n\tActual: %#v\n%s", filepath.Base(file), line, expected, actual, message)
		t.FailNow()
	}
}
