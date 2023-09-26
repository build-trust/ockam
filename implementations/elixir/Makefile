PACKAGES := \
	ockly \
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
	cd ockam/$* && MIX_ENV=prod MIX_BUILD_ROOT="../_build" mix compile

build_%: deps_%
	cd ockam/$* && MIX_BUILD_ROOT="../_build" mix compile

test: $(addprefix test_,$(PACKAGES))
test_%: deps_%
	cd ockam/$* &&  MIX_BUILD_ROOT="../_build" mix test

lint: $(addprefix lint_,$(PACKAGES))
lint_%: deps_%
	cd ockam/$* && MIX_ENV=test MIX_BUILD_ROOT="../_build" mix lint

clean: $(addprefix clean_,$(PACKAGES))
clean_%:
	cd ockam/$* && mix clean

very_clean: $(addprefix very_clean_,$(PACKAGES))
	$(MAKE) -C ../rust very_clean
	rm -rf ockam/_build
	rm -rf ockam/_deps

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
