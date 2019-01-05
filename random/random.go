// Package random is used to generate random bytes and strings.
package random

import (
	"crypto/rand"

	"github.com/pkg/errors"
)

// GenerateBytes returns a randomly generated slice of n bytes. It will return
// an error for any error returned by rand.Read
func GenerateBytes(n int) ([]byte, error) {
	// make a new slice of bytes of length n
	bytes := make([]byte, n)

	// read n cryptographically secure pseudorandom numbers from rand.Read
	// and write them to the given slice of bytes. In case rand.Read return an
	// error, return that error
	_, err := rand.Read(bytes)
	if err != nil {
		return nil, errors.WithStack(err)
	}

	return bytes, nil
}

// GenerateString returns a randomly generated string of the provided length.
// All the characters in the returned string are selected from the provided
// charset string.
func GenerateString(length int, charset string) (string, error) {

	// generate a slice of random bytes of input length
	bytes, err := GenerateBytes(length)
	if err != nil {
		return "", err
	}

	// get the length of the charset string, i.e the number of characters
	charsetLength := byte(len(charset))

	// replace each byte in the slice with a character that is in the charset
	for i, b := range bytes {
		bytes[i] = charset[b%charsetLength]
	}

	// return the slice as a string
	return string(bytes), nil
}

// GenerateAlphaNumericString returns a randomly generated string of the
// provided length selected from the characters in the charset
// "0123456789abcdefghijklmnopqrstuvwxyz"
func GenerateAlphaNumericString(length int) (string, error) {
	const chars = "0123456789abcdefghijklmnopqrstuvwxyz"
	return GenerateString(length, chars)
}
