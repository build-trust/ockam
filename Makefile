build: rust_build elixir_build
build_release: rust_build_release elixir_build_release
test: rust_test elixir_test
lint: rust_lint elixir_lint notice_file_lint
clean: rust_clean elixir_clean
very_clean: rust_very_clean elixir_very_clean

elixir_%:
	$(MAKE) -C implementations/elixir $(@:elixir_%=%)

# run an elixir command in a nix environment, so that all tools are installed
nix_elixir_%:
	nix develop ./tools/nix#elixir --command make elixir_$*

rust_%:
	$(MAKE) -f implementations/rust/Makefile $(@:rust_%=%)

# run a rust command in a nix environment, so that all tools are installed
nix_rust_%:
	nix develop ./tools/nix#rust --command make rust_$*

notice_file_update:
	cargo deny --all-features  list --config=tools/cargo-deny/deny.toml --layout crate --format json | ./tools/scripts/release/parseCrates.sh

.PHONY: \
	build build_release test lint clean very_clean \
	elixir_% rust_% nix_rust_%  nix_elixir_% \
