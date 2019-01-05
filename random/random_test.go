package random

import (
	"bytes"
	"fmt"
	"regexp"
	"testing"
)

func TestGenerateBytes(t *testing.T) {

	t.Run("consecutive runs shouldn't return same output", func(t *testing.T) {
		a, _ := GenerateBytes(10)
		b, _ := GenerateBytes(10)
		if bytes.Equal(a, b) {
			t.Errorf("output from two consecutive runs is equal %s:%s", a, b)
		}
	})

	t.Run("returned []byte's length should match argument", func(t *testing.T) {
		expected := 64
		a, _ := GenerateBytes(expected)
		actual := len(a)

		if expected != actual {
			t.Errorf("length %d is not as expected %d", actual, expected)
		}
	})

}

func TestGenerateString(t *testing.T) {

	t.Run("consecutive runs shouldn't return same output", func(t *testing.T) {
		a, _ := GenerateString(10, "123")
		b, _ := GenerateString(10, "123")
		if a == b {
			t.Errorf("output from two consecutive runs is equal %s:%s", a, b)
		}
	})

	t.Run("returned strings length should match argument", func(t *testing.T) {
		expected := 64
		a, _ := GenerateString(expected, "123")
		actual := len(a)

		if expected != actual {
			t.Errorf("length %d is not as expected %d", actual, expected)
		}
	})

	t.Run("output should only contain chars from charset", func(t *testing.T) {
		charset := "1234ABCD"
		a, _ := GenerateString(256, charset)
		expression := fmt.Sprintf("^[%s]+$", charset)
		onlyContains := regexp.MustCompile(expression).MatchString

		if !onlyContains(a) {
			t.Errorf("output %s contains chars outside the charset %s", a, charset)
		}
	})

}

func TestGenerateAlphaNumericString(t *testing.T) {

	t.Run("consecutive runs shouldn't return same output", func(t *testing.T) {
		a, _ := GenerateAlphaNumericString(256)
		b, _ := GenerateAlphaNumericString(256)
		if a == b {
			t.Errorf("output from two consecutive runs is equal %s:%s", a, b)
		}
	})

	t.Run("returned strings length should match argument", func(t *testing.T) {
		expected := 64
		a, _ := GenerateAlphaNumericString(expected)
		actual := len(a)

		if expected != actual {
			t.Errorf("length %d is not as expected %d", actual, expected)
		}
	})

	t.Run("should only return chars from fixed charset", func(t *testing.T) {
		charset := "0123456789abcdefghijklmnopqrstuvwxyz"
		a, _ := GenerateAlphaNumericString(256)
		expression := fmt.Sprintf("^[%s]+$", charset)
		onlyContains := regexp.MustCompile(expression).MatchString

		if !onlyContains(a) {
			t.Errorf("output %s contains chars outside the charset %s", a, charset)
		}
	})

}
