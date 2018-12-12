package test

import (
	"bytes"
	"io"
	"os"
)

// The below Capture function is inspired from https://github.com/kami-zh/go-capturer
// which is licenced under MIT License, see NOTICE file for attribution

// Capture returns the stdout and stderr output from invoking the passed in function
// This is useful when testing what a function outputs to stdout/stderr
//
// Since this function is designed for use by tests, it ignores all internal errors
// if they happen and proceeds to return stdout/stderr as strings. The assumption here
// is that the calling tests would fail if the desired output is not present in the
// returned strings
func Capture(f func()) (string, string) {
	rOut, wOut, _ := os.Pipe()
	rErr, wErr, _ := os.Pipe()

	stdout := os.Stdout
	os.Stdout = wOut
	defer func() { os.Stdout = stdout }()

	stderr := os.Stderr
	os.Stderr = wErr
	defer func() { os.Stderr = stderr }()

	f()

	_ = wOut.Close()
	_ = wErr.Close()

	var outBuf bytes.Buffer
	var errBuf bytes.Buffer

	_, _ = io.Copy(&outBuf, rOut)
	_, _ = io.Copy(&errBuf, rErr)

	return outBuf.String(), errBuf.String()
}
