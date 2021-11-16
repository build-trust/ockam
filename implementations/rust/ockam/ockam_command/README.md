# Ockam Command (WIP)

## Running

`cargo build`

Help:

`target/debug/ockam -h` or `--help` or `help`


Node command mock up:

`target/debug/ockam node create`

## Status

---

### Configuration

- Command line args
- Optional `ockam.toml` (override name with `-c`)
- Optional `ockam_secrets.toml` for distinct secret management (override name with `-s`)
- Environment variables
  - Support for `.env` file
  - Automatically pulls in all vars with `OCKAM_` prefix

---

### Subcommands

- Support for subcommands (which can have subcommands, etc)
- Easily modifiable `cli.yml` driven command and arg configuration
- Modular, loosely coupled command pattern

---

### Help

- Very verbose usage help text for commands and subcommands
- Auto generated from `cli.yml`

---

### Process control

- Ctrl+C handling
- Human readable panics

---

## UI

- Rich colorized text support
- Progress bars / spinners
- Tables

---

### Logging

- Rich, colorized logging
- Debug and trace support via environment variables
