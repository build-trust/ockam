// +build softhsm

package pkcs11

import (
	"crypto"
	"crypto/rsa"
	"crypto/sha512"
	"crypto/x509"
	"encoding/pem"
	"fmt"
	"io/ioutil"
	"os/exec"
	"testing"

	"github.com/ockam-network/did"
	"github.com/ockam-network/ockam"
	"github.com/ockam-network/ockam/claim"
	"github.com/ockam-network/ockam/entity"
	"github.com/stretchr/testify/assert"
)

const (
	softHSMPath = "/usr/local/lib/softhsm/libsofthsm2.so"
	softPIN     = "\"mypin\""
	softToken   = "\"mytoken\""
	softKeyPath = "fixtures/pkcs8.key"
)

// TestMain does setup and teardown of the SoftHSM utility
// so test can run idempotently
func TestMain(m *testing.M) {
	if err := initializeSoftHSM(); err != nil {
		panic(err)
	}
	defer cleanSoftHSM()
	m.Run()

}

func TestNew(t *testing.T) {
	p := testFixture(t)
	defer p.signer.Destroy()

	t.Run("modulePath should be set to passed in value", func(t *testing.T) {
		assert.Equal(t, p.modulePath, softHSMPath, "module paths are not identical")
	})

	t.Run("pin should be set to passed in value", func(t *testing.T) {
		assert.Equal(t, p.pin, softPIN)
	})

	t.Run("token label should be set to passed in value", func(t *testing.T) {
		assert.Equal(t, p.tokenLabel, softToken)
	})

	t.Run("signer should not be nil", func(t *testing.T) {
		assert.NotNil(t, p.signer)
	})

	t.Run("public key should be not nil and loaded from disk", func(t *testing.T) {
		assert.NotNil(t, p.publicKey, "public key value is nil")

		key, err := readPKCS8PrivateKeyFixture()
		assert.Nil(t, err, "error reading test fixture private key")

		assert.Equal(t, p.publicKey, key.Public(), "the public key set is not the test fixture key")
	})
}

func TestSign(t *testing.T) {
	p := testFixture(t)
	defer p.signer.Destroy()

	e, _ := defaultEntity(t)

	c, err := claim.New(
		claim.Data{"foo": "bar"},
		claim.Issuer(e),
		claim.ID("test-id"),
		claim.Type("test-type"),
		claim.Subject(e))
	assert.Nil(t, err, "claim.New() returned an error")

	if err := p.Sign(c); err != nil {
		assert.Nil(t, err, "Sign() returned an error")
	}

	t.Run("should create 1 valid signature", func(t *testing.T) {
		assert.Len(t, c.Signatures(), 1, fmt.Sprintf("expected 1 signature in claim, got %d", len(c.Signatures())))
	})

	t.Run("signature should verify agianst key", func(t *testing.T) {
		key, err := readPKCS8PrivateKeyFixture()
		assert.Nil(t, err)

		claimBytes, err := claimLDJSONBytes(c)
		assert.Nil(t, err)

		hashed := sha512.Sum512(claimBytes)
		signature := c.Signatures()[0].SignatureValue()

		if err := rsa.VerifyPKCS1v15(&key.PublicKey, crypto.SHA512, hashed[:], signature); err != nil {
			t.Error(err)
		}
	})
}

/*** the following are helper methods for testing ***/

// testFixture returns an intialized instance of a PKCS11 object
// configured for use with SoftHSM
func testFixture(t *testing.T) *PKCS11 {
	key, err := readPKCS8PrivateKeyFixture()
	if err != nil {
		t.Fatal(err)
	}

	p, err := New(
		ModulePath(softHSMPath),
		TokenLabel(softToken),
		PIN(softPIN),
		PublicKey(key.Public()))

	if err != nil {
		fmt.Println("error inializing new pkcs11 object")
		t.Fatal("could not initialize new pkcs11 object: :" + err.Error())
	}

	if p == nil {
		t.Fatal("pkcs object is nil")
	}

	return p
}

// defaultEntity returns an initialized ocakm.Entity and DID
// for use with ockam.Claim in testing
func defaultEntity(t *testing.T) (ockam.Entity, *did.DID) {
	testBytes := []byte("foo")

	did, err := entity.NewDID(testBytes)
	assert.Nil(t, err)

	e, err := entity.New(entity.Attributes{}, entity.ID(did))
	assert.Nil(t, err)

	return e, did
}

// cleanSoftHSM deletes dangling tokens from the soft HSM utility after
// testing is completed
func cleanSoftHSM() {
	fmt.Println("cleaning up tokens")
	clean := exec.Command("softhsm2-util", "--delete-token", "--token", softToken, "--force")
	out, _ := clean.CombinedOutput()
	fmt.Println(string(out))
}

func initializeSoftHSM() error {
	/* initializes the softHSM (https://github.com/opendnssec/SoftHSMv2) utility with the pre-genreated pkcs8 private key fixutre.
	verifyication of these commands is avialable through the pkcs11-tool:
	pkcs11-tool --list-slots --module /usr/local/lib/softhsm/libsofthsm2.so
		Slot 1 (0x67e0a7e9): SoftHSM slot ID 0x67e0a7e9
			token label        : mytoken
			token manufacturer : SoftHSM project
			token model        : SoftHSM v2
			token flags        : login required, rng, token initialized, PIN initialized, other flags=0x20
			hardware version   : 2.5
			firmware version   : 2.5
			serial num         : b351cad167e0a7e9
			pin min/max        : 4/255
	*/
	initTokenCmd := exec.Command(
		"softhsm2-util",
		"--init-token",
		"--free",
		"--label", softToken,
		"--so-pin", softPIN,
		"--pin", softPIN,
		"--force")

	importKeyCmd := exec.Command(
		"softhsm2-util",
		"--import", softKeyPath,
		"--token", softToken,
		"--label", softToken,
		"--id", "A1B2",
		"--pin", softPIN,
		"--force")

	printout := func(cmd *exec.Cmd) error {
		stdoutStderr, err := cmd.CombinedOutput()
		fmt.Printf(string(stdoutStderr))
		return err
	}

	if err := printout(initTokenCmd); err != nil {
		return err
	}

	return printout(importKeyCmd)
}

func readPKCS8PrivateKeyFixture() (*rsa.PrivateKey, error) {
	bytes, err := ioutil.ReadFile(softKeyPath)
	if err != nil {
		return nil, err
	}

	block, _ := pem.Decode(bytes)

	parseResult, err := x509.ParsePKCS8PrivateKey(block.Bytes)
	if err != nil {
		return nil, err
	}

	key := parseResult.(*rsa.PrivateKey)

	return key, nil
}
