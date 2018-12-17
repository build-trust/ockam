package main

import (
	"io/ioutil"
	"os"
	"runtime"
	"testing"

	"github.com/ockam-network/ockam/test"
)

// nolint: dupl
func Test_configSave(t *testing.T) {
	ockamHomeBeforeTest := os.Getenv("OCKAM_HOME")

	t.Run("correctly saves config that can be read back", func(t *testing.T) {
		temp, err := ioutil.TempDir("", "ockam_home")
		if err != nil {
			t.Error(err)
		}
		defer os.RemoveAll(temp)
		os.Setenv("OCKAM_HOME", temp)

		c := &config{UserName: "test"}
		err = c.save()
		if err != nil {
			t.Error(err)
		}

		output, err := newConfig()
		if err != nil {
			t.Error(err)
		}

		test.Assert(t, &config{UserName: "test"}, output)
		test.Assert(t, "test", output.UserName)
	})

	os.Setenv("OCKAM_HOME", ockamHomeBeforeTest)
}

// nolint: dupl
func Test_newConfig(t *testing.T) {
	ockamHomeBeforeTest := os.Getenv("OCKAM_HOME")

	t.Run("returns pointer to empty config if config file does not exist", func(t *testing.T) {
		temp, err := ioutil.TempDir("", "ockam_home")
		if err != nil {
			t.Error(err)
		}
		defer os.RemoveAll(temp)
		os.Setenv("OCKAM_HOME", temp)

		output, err := newConfig()
		if err != nil {
			t.Error(err)
		}

		test.Assert(t, &config{}, output)
		test.Assert(t, "", output.UserName)
	})

	t.Run("reads config from file if config file exists", func(t *testing.T) {
		temp, err := ioutil.TempDir("", "ockam_home")
		if err != nil {
			t.Error(err)
		}
		defer os.RemoveAll(temp)
		os.Setenv("OCKAM_HOME", temp)

		c := &config{UserName: "test"}
		c.save()

		output, err := newConfig()
		if err != nil {
			t.Error(err)
		}

		test.Assert(t, &config{UserName: "test"}, output)
		test.Assert(t, "test", output.UserName)
	})

	t.Run("fails if config file doesn't have valid config", func(t *testing.T) {
		temp, err := ioutil.TempDir("", "ockam_home")
		if err != nil {
			t.Error(err)
		}
		defer os.RemoveAll(temp)
		os.Setenv("OCKAM_HOME", temp)

		err = ensureConfigHomeExists()
		if err != nil {
			t.Error(err)
		}

		// write some invalid data to the config file
		err = ioutil.WriteFile(configFilePath(), []byte("hello"), 0644)
		if err != nil {
			t.Error(err)
		}

		_, err = newConfig()
		test.Assert(t, true, err != nil)
	})

	os.Setenv("OCKAM_HOME", ockamHomeBeforeTest)
}

func Test_configFileExists(t *testing.T) {
	ockamHomeBeforeTest := os.Getenv("OCKAM_HOME")

	t.Run("returns false if config file does not exist", func(t *testing.T) {
		temp, err := ioutil.TempDir("", "ockam_home")
		if err != nil {
			t.Error(err)
		}
		defer os.RemoveAll(temp)
		os.Setenv("OCKAM_HOME", temp)
		output := configFileExists()

		test.Assert(t, false, output)
	})

	t.Run("returns true if config file exists", func(t *testing.T) {
		temp, err := ioutil.TempDir("", "ockam_home")
		if err != nil {
			t.Error(err)
		}
		defer os.RemoveAll(temp)
		os.Setenv("OCKAM_HOME", temp)

		err = ensureConfigHomeExists()
		if err != nil {
			t.Error(err)
		}
		newFile, err := os.Create(configFilePath())
		if err != nil {
			t.Error(err)
		}
		defer newFile.Close()
		output := configFileExists()

		test.Assert(t, true, output)
	})

	os.Setenv("OCKAM_HOME", ockamHomeBeforeTest)
}

func Test_configFilePath(t *testing.T) {
	ockamHomeBeforeTest := os.Getenv("OCKAM_HOME")

	t.Run("returns configFilePath()/config.json", func(t *testing.T) {
		os.Setenv("OCKAM_HOME", "ockam_home")
		configFilePath := configFilePath()
		test.Assert(t, "ockam_home/config/config.json", configFilePath)
	})

	os.Setenv("OCKAM_HOME", ockamHomeBeforeTest)
}

func Test_configHome(t *testing.T) {
	ockamHomeBeforeTest := os.Getenv("OCKAM_HOME")
	xdgValueBeforeTest := os.Getenv("XDG_CONFIG_HOME")

	t.Run("returns \"$OCKAM_HOME/config\" if OCKAM_HOME is set", func(t *testing.T) {
		valueBeforeTest := os.Getenv("OCKAM_HOME")
		os.Setenv("OCKAM_HOME", "ockam_home")
		home := configHome()
		os.Setenv("OCKAM_HOME", valueBeforeTest)

		test.Assert(t, "ockam_home/config", home)
	})

	t.Run("returns \"$XDG_CONFIG_HOME/ockam\" if OCKAM_HOME is not set, XDG_CONFIG_HOME is set", func(t *testing.T) {
		os.Unsetenv("OCKAM_HOME")
		os.Setenv("XDG_CONFIG_HOME", "xdg_config_home")
		home := configHome()
		test.Assert(t, "xdg_config_home/ockam", home)
	})

	t.Run("returns defaultConfigHome for os if OCKAM_HOME and XDG_CONFIG_HOME are not set", func(t *testing.T) {
		os.Unsetenv("OCKAM_HOME")
		os.Unsetenv("XDG_CONFIG_HOME")
		home := configHome()
		test.Assert(t, defaultConfigHome(runtime.GOOS), home)
	})

	os.Setenv("OCKAM_HOME", ockamHomeBeforeTest)
	os.Setenv("XDG_CONFIG_HOME", xdgValueBeforeTest)
}

// nolint: dupl
func Test_defaultConfigHome(t *testing.T) {
	appDataBeforeTest := os.Getenv("APPDATA")
	homeBeforeTest := os.Getenv("HOME")

	t.Run("returns %APPDATA%\\Ockam for windows", func(t *testing.T) {
		os.Setenv("APPDATA", "test")
		home := defaultConfigHome("windows")
		test.Assert(t, "test\\Ockam", home)
	})

	t.Run("returns \"$HOME/Library/Application Support/Ockam\" for osx", func(t *testing.T) {
		os.Setenv("HOME", "test")
		home := defaultConfigHome("darwin")
		test.Assert(t, "test/Library/Application Support/Ockam", home)
	})

	t.Run("returns \"test/.config/ockam\" for linux", func(t *testing.T) {
		os.Setenv("HOME", "test")
		home := defaultConfigHome("linux")
		test.Assert(t, "test/.config/ockam", home)
	})

	os.Setenv("APPDATA", appDataBeforeTest)
	os.Setenv("HOME", homeBeforeTest)
}
