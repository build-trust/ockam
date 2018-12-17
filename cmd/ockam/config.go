package main

import (
	"encoding/json"
	"io/ioutil"
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

type config struct {
	UserName         string `json:"userName"`
	UserEmail        string `json:"userEmail"`
	UserOrganization string `json:"userOrganization"`
	UserReason       string `json:"userReason"`
}

// save turns the config to json and stores it to disk at configFilePath() with 0644 permissions.
// if configHome() does not exist it would be created
func (c *config) save() error {
	// check if configHome() exists, create if it doesn't
	err := ensureConfigHomeExists()
	if err != nil {
		return err
	}

	// turn the config into json with tab indents
	marshalled, err := json.MarshalIndent(c, "", "\t")
	if err != nil {
		return err
	}

	// write the marshalled json to configFilePath() with permission bits set to 0644
	return ioutil.WriteFile(configFilePath(), marshalled, 0644)
}

// New returns a new Config
func newConfig() (*config, error) {
	if !configFileExists() {
		return &config{}, nil
	}

	var c config
	content, err := ioutil.ReadFile(configFilePath())
	if err != nil {
		return nil, err
	}

	err = json.Unmarshal(content, &c)
	if err != nil {
		return nil, err
	}

	return &c, nil
}

// configFileExists returns true if config file at configFilePath() exists, false if it does not.
func configFileExists() bool {
	_, err := os.Stat(configFilePath())
	return !os.IsNotExist(err)
}

// configFilePath returns the path of the main configuration file. This file is called config.json
// and lives the configuration home directory as returned by configHome()
func configFilePath() string {
	return filepath.Join(configHome(), "config.json")
}

// ensureConfigHome creates the configHome() path along with any necessary parents,
// and returns nil, or else returns an error.
//
// If configHome() is already a directory, ensureConfigHomeExists does nothing and returns nil.
//
// The permission bits 0700 are used for all directories that ensureConfigHomeExists may create.
func ensureConfigHomeExists() error {
	return os.MkdirAll(configHome(), 0700)
}

// configHome returns the path of the configuration home directory.
func configHome() string {
	// If OCKAM_HOME is set in the environment then return
	// ${OCKAM_HOME}/config with the appropriate os specific path separator.
	ockamHome := os.Getenv("OCKAM_HOME")
	if ockamHome != "" {
		return filepath.Join(ockamHome, "config")
	}

	// If OCKAM_HOME is not set but XDG_CONFIG_HOME is set in the environment
	// then return ${XDG_CONFIG_HOME}/ockam with the appropriate os
	// specific path separator.
	xdgConfigHome := os.Getenv("XDG_CONFIG_HOME")
	if xdgConfigHome != "" {
		return filepath.Join(xdgConfigHome, "ockam")
	}

	// If neither OCKAM_HOME or XDG_CONFIG_HOME are set then return the response
	// of defaultConfigHome(runtime.GOOS)
	return defaultConfigHome(runtime.GOOS)
}

// defaultConfigHome returns the default configuration home directory for a given operating system
// It is meant to be called as follows
//   defaultConfigHome(runtime.GOOS)
func defaultConfigHome(operatingSystem string) string {
	switch operatingSystem {
	case "windows":
		// On Windows return %APPDATA%\Ockam
		// https://bit.ly/2rtEK9V
		path := []string{os.Getenv("APPDATA"), "Ockam"}
		return strings.Join(path, "\\")
	case "darwin":
		// On MacOS return ${HOME}/Library/Application Support/Ockam
		// https://apple.co/2BeZjf5
		path := []string{os.Getenv("HOME"), "Library", "Application Support", "Ockam"}
		return strings.Join(path, "/")
	default:
		// Assuming all other os values are unix like, return ${HOME}/.config/ockam
		// based on recommendations in XDG Base Directory Specification
		// https://standards.freedesktop.org/basedir-spec/basedir-spec-latest.html
		path := []string{os.Getenv("HOME"), ".config", "ockam"}
		return strings.Join(path, "/")
	}
}
