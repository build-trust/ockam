# Ockam Implementations

Ockam cryptographic and messaging protocols can be implemented in various languages.
Our current focus is [Rust](rust) and [Elixir](elixir) but over time we will support many languages.


## Running Ockam Tests With Nix

We run ockam tests internally using [Nix](https://nixos.org/) environment, our Nix tooling can be found in tools/nix from the root path directory, we also have a [Makefile](https://www.gnu.org/software/make/) that consist of all Rust tests that are run during development, Rust test consists of

- Cargo test: make -f implementations/rust/Makefile test
- Cargo build: make -f implementations/rust/Makefile build
- Cargo build examples: make -f implementations/rust/Makefile build_examples
- Cargo deny: make -f implementations/rust/Makefile lint_cargo_deny
- Lint crates readme file: make -f implementations/rust/Makefile lint_cargo_readme
- Lint cargo toml files: make -f implementations/rust/Makefile lint_cargo_toml_files
- Rust code format check: make -f implementations/rust/Makefile lint_cargo_fmt_check
- Cargo clippy lint: make -f implementations/rust/Makefile lint_cargo_clippy
- No std compatibility check: make -f implementations/rust/Makefile check_no_std

...and more

Running Makefile with the nix environment requires using the commands above with the `nix develop` command, e.g. to run cargo test on stable with Nix and Make, we can run
```bash
nix develop ./tools/nix#rust --command make -f implementations/rust/Makefile test
```
