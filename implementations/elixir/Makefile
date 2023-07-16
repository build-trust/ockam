PACKAGES := \
	ockam_vault_software \
	ockam \
	ockam_abac \
	ockam_services \
	ockam_kafka \
	ockam_metrics \
	ockam_healthcheck \
	ockam_cloud_node \
	ockam_typed_cbor

build: $(addprefix build_,$(PACKAGES))
build_release: $(addprefix build_release_,$(PACKAGES))

build_release_%: deps_%
	cd ockam/$* && MIX_ENV=prod mix compile
build_release_ockam_vault_software: rust_build_release_ockam-ffi deps_ockam_vault_software
	cd ockam/ockam_vault_software && MIX_ENV=prod mix compile
rust_build_release_ockam-ffi:
	$(MAKE) -C ../rust build_release_ockam-ffi

build_%: deps_%
	cd ockam/$* && mix compile
build_ockam_vault_software: rust_build_release_ockam-ffi deps_ockam_vault_software
	cd ockam/ockam_vault_software && mix compile
rust_build_ockam-ffi:
	$(MAKE) -C ../rust build_ockam-ffi

test: $(addprefix test_,$(PACKAGES))
test_%: build_% deps_%
	cd ockam/$* && mix test

lint: $(addprefix lint_,$(PACKAGES))
lint_%: deps_%
	cd ockam/$* && mix lint

clean: $(addprefix clean_,$(PACKAGES))
clean_%:
	cd ockam/$* && mix clean

very_clean: $(addprefix very_clean_,$(PACKAGES))
	$(MAKE) -C ../rust very_clean
very_clean_%:
	cd ockam/$* && rm -rf deps && rm -rf _build

deps: $(addprefix deps_,$(PACKAGES))
deps_%: hex rebar
	cd ockam/$* && mix deps.get

hex:
	mix local.hex --force --if-missing

rebar:
	mix local.rebar --force --if-missing

.PHONY: \
	build build_% build_release build_release_% \
	test test_% lint lint_% clean clean_% deps deps_% hex rebar
