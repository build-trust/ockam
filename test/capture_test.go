package test

import (
	"fmt"
	"os"
	"testing"
)

func TestCapture(t *testing.T) {
	t.Run("captures stdout", func(t *testing.T) {
		stdout, stderr := Capture(func() {
			fmt.Println("hello out")
		})

		Assert(t, "hello out\n", stdout)
		Assert(t, "", stderr)
	})

	t.Run("captures stderr", func(t *testing.T) {
		stdout, stderr := Capture(func() {
			os.Stderr.WriteString("hello error")
		})

		Assert(t, "", stdout)
		Assert(t, "hello error", stderr)
	})
}
