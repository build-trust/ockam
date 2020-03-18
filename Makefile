.PHONY: help builder docker virtualbox

IMAGE_NAME ?= ockam-network/ockam
PUBKEY ?= `cat ~/.ssh/id_rsa.pub || echo ''`

help:
	@echo "$(IMAGE_NAME)"
	@perl -nle'print $& if m{^[a-zA-Z_-]+:.*?## .*$$}' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

docker-builder: ## Build the builder Docker image
	@cd tools/builder/debian && \
	 DOCKER_BUILDKIT=1 docker build \
		--build-arg public_key="$(PUBKEY)" \
		-t ockam-builder-debian-base:latest .

docker: ## Build inside Docker
	@VAGRANT_DEFAULT_PROVIDER=docker ./gradlew build

virtualbox: ## Build inside VirtualBox VM
	@VAGRANT_DEFAULT_PROVIDER=virtualbox ./gradlew build

virtualbox-local: ## Build using locally-built Debian VM
	@VAGRANT_DEFAULT_PROVIDER=virtualbox \
     OCKAM_DEBIAN_BUILDER_BOX=debian-9.9.0-amd64 \
     OCKAM_DEBIAN_BUILDER_BOX_URL="file://tools/builder/debian/_build/debian-9.9.0-amd64.virtualbox.box" \
     ./gradlew build

debian-builder: ## Build the Debian VirtualBox image
	@cd tools/builder/debian && \
        ./build

clean-debian-builder: ## Clean up artifacts produced when building VMs
	@cd tools/builder/debian && \
        ./clean
