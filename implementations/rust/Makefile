build:
	cargo build
build_release:
	cargo --locked build --release
build_release_%:
	cargo --locked build --release --package $*
build_%:
	cargo build --package $*

test:
	@if [ "$(CI)" = "true" ]; then \
		$(MAKE) test_nextest; \
	else \
		cargo --locked test; \
	fi
test_%:
	@if [ "$(CI)" = "true" ]; then \
		$(MAKE) test_nextest; \
	else \
		cargo --locked test; \
	fi
test_nextest:
	cargo --locked nextest --config-file tools/nextest/.config/nextest.toml run
	cargo --locked test --doc

lint: lint_format lint_cargo_deny lint_cargo_clippy
lint_format:
	cargo fmt --all -- --check
lint_cargo_deny:
	cargo deny --all-features \
		--manifest-path=../../Cargo.toml check \
		--config=tools/cargo-deny/deny.toml licenses advisories
lint_cargo_clippy:
	cargo clippy --no-deps --all-targets -- -D warnings

clean:
	cargo clean
clean_%:
	cargo clean --package $*

very_clean:
	rm -rf ../../target

docs:
	cargo doc --no-deps

format:
	cargo fmt --all

.PHONY: \
	build build_% build_release build_release_% \
	test test_% test_nextest \
	lint lint_format lint_cargo_deny lint_cargo_clippy \
	clean clean_% very_clean docs format
