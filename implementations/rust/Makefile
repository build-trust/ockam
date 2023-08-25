build_docs:
	cargo doc --locked --no-deps
build_examples:
	cargo --locked build --examples
build_packages:
	cargo build --locked
build_release:
	cargo --locked build --release
build_release_%:
	cargo --locked build --release --package $*
build_%:
	cargo build --locked --package $*

check_no_std:
	cargo --version
	cargo check --locked --target thumbv7em-none-eabihf -p ockam --no-default-features --features 'no_std alloc software_vault'
	# no_std example project
	cd examples/rust/example_projects/no_std
	cargo check --example hello
	# RUSTFLAGS='-Dwarnings' cargo check --example hello --no-default-features --features 'alloc no_std'
	# RUSTFLAGS='-Dwarnings' cargo +nightly check --example hello --target thumbv7em-none-eabihf --no-default-features --features="qemu"
	# RUSTFLAGS='-Dwarnings' cargo +nightly check --example hello --target thumbv7em-none-eabihf --no-default-features --features="atsame54"
	# RUSTFLAGS='-Dwarnings' cargo +nightly check --example hello --target thumbv7em-none-eabihf --no-default-features --features="stm32f4"
check_tag:
	cargo check --locked --features tag
check_cargo_update:
	rm -rf Cargo.lock
	cargo update
	cargo check --locked

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
		cargo --locked test --package $*; \
	fi
test_nextest:
	cargo --locked nextest --config-file tools/nextest/.config/nextest.toml run
	cargo --locked test --doc
test_nextest_%:
	cargo --locked nextest --config-file tools/nextest/.config/nextest.toml -p $* run
	cargo --locked test --doc

lint: lint_cargo_fmt_check lint_cargo_deny lint_cargo_clippy
lint_cargo_fmt_check:
	cargo fmt --all -- --check
lint_cargo_deny:
	cargo deny --all-features \
		check licenses advisories\
		--config=tools/cargo-deny/deny.toml
lint_cargo_clippy:
	cargo clippy --no-deps --all-targets -- -D warnings
lint_cargo_toml_files:
	dprint check --config tools/dprint/dprint.json
lint_cargo_readme:
	@set -e;	\
	for dir in implementations/rust/ockam/*; do \
		if [ -d "$$dir" ]; then	\
			$(MAKE) -f $(MAKEFILE_LIST) lint_cargo_readme_$$(basename $$dir); \
		fi \
	done

lint_cargo_readme_%:
	@set -e; \
	crate=$*; \
	cargo readme --project-root implementations/rust/ockam/$$crate --template ../README.tpl -o README-updated.md; \
	cmp -s implementations/rust/ockam/$$crate/README.md implementations/rust/ockam/$$crate/README-updated.md; \
	retval=$$?; \
	rm -f implementations/rust/ockam/$$crate/README-updated.md; \
	if [ $$retval -ne 0 ] ; then \
		echo "Readme defined in $$crate directory is not up to update";	\
		exit 1;	\
	fi

clean:
	cargo clean
clean_%:
	cargo clean --package $*

very_clean:
	rm -rf ../../target

format:
	cargo fmt --all

.PHONY:
	build_docs build_examples build_packages build_release build_release_% build_% \
	test test_% test_nextest \
	check_no_std check_tag check_cargo_update \
	lint lint_cargo_fmt_check lint_cargo_deny lint_cargo_clippy lint_cargo_readme lint_cargo_readme_% lint_cargo_toml_files \
	clean clean_% very_clean format
