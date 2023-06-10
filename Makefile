build: rust_build elixir_build typescript_build
build_release: rust_build_release elixir_build_release typescript_build_release
test: rust_test elixir_test typescript_test
lint: rust_lint elixir_lint typescript_lint
clean: rust_clean elixir_clean typescript_clean
very_clean: rust_very_clean elixir_very_clean typescript_very_clean

elixir_%:
	$(MAKE) -C implementations/elixir $(@:elixir_%=%)

rust_%:
	$(MAKE) -C implementations/rust $(@:rust_%=%)

typescript_%:
	$(MAKE) -C implementations/typescript $(@:typescript_%=%)

.PHONY: \
	build build_release test lint clean very_clean \
	elixir_% rust_% typescript_%
