build: typescript_build rust_build elixir_build
build_release: typescript_build_release rust_build_release elixir_build_release
test: typescript_test rust_test elixir_test
lint: typescript_lint rust_lint elixir_lint
clean: typescript_clean rust_clean elixir_clean
very_clean: typescript_very_clean rust_very_clean elixir_very_clean

elixir_%:
	@cd implementations/elixir && $(MAKE) $(@:elixir_%=%)

rust_%:
	$(MAKE) -C implementations/rust $(@:rust_%=%)

swift_%:
	$(MAKE) -C implementations/swift $(@:swift_%=%)

typescript_%:
	@cd implementations/typescript && $(MAKE) $(@:typescript_%=%)

.PHONY: \
	build build_release test lint clean very_clean \
	elixir_% rust_% swift_% typescript_%
