build: format
	uv run maturin develop

build_release:
	uv run maturin build --release --target aarch64-apple-darwin

test: build
	uv run pytest -vv

lint:
	uv run pylint python
	uv run ruff check examples python --fix

clean:
	rm -rf python/ockam/*.abi3.so

very_clean: clean

format:
	cargo fmt
	uv run ruff format examples python

release-linux:
	docker run --rm -v "$(shell pwd)/..":/io --platform linux/amd64 ghcr.io/pyo3/maturin build --release --target x86_64-unknown-linux-gnu --manifest-path /io/python/Cargo.toml
	docker run --rm -v "$(shell pwd)/..":/io --platform linux/arm64 ghcr.io/pyo3/maturin build --release --target aarch64-unknown-linux-gnu --manifest-path /io/python/Cargo.toml

build-image-amd64:
	docker run --rm -v "$(shell pwd)/..":/io --platform linux/amd64 ghcr.io/pyo3/maturin build --release --target x86_64-unknown-linux-gnu --manifest-path /io/python/Cargo.toml
	$(eval SHORT_COMMIT_HASH := $(shell git rev-parse --short HEAD))

	docker pull cgr.dev/chainguard/python:latest-dev
	docker pull cgr.dev/chainguard/python:latest

	docker build --platform linux/amd64 \
		-t ghcr.io/build-trust/ockam-python-dev:latest \
		-t ghcr.io/build-trust/ockam-python-dev:$(SHORT_COMMIT_HASH) --file Dockerfile.dev .

	docker build --platform linux/amd64 \
		-t ghcr.io/build-trust/ockam-python:latest \
		-t ghcr.io/build-trust/ockam-python:$(SHORT_COMMIT_HASH) .

release-image-amd64-hash:
	$(eval SHORT_COMMIT_HASH := $(shell git rev-parse --short HEAD))
	docker push ghcr.io/build-trust/ockam-python-dev:$(SHORT_COMMIT_HASH)
	docker push ghcr.io/build-trust/ockam-python:$(SHORT_COMMIT_HASH)

release-image-amd64-latest:
	$(eval SHORT_COMMIT_HASH := $(shell git rev-parse --short HEAD))
	docker tag ghcr.io/ockam-network/ockam-python:$(SHORT_COMMIT_HASH) ghcr.io/ockam-network/ockam-python:latest
	docker tag ghcr.io/ockam-network/ockam-python-dev:$(SHORT_COMMIT_HASH) ghcr.io/ockam-network/ockam-python-dev:latest

	docker push ghcr.io/build-trust/ockam-python-dev:latest
	docker push ghcr.io/build-trust/ockam-python:latest

build-logs-server-image-amd64:
	$(eval SHORT_COMMIT_HASH := $(shell git rev-parse --short HEAD))
	docker build -t ockam-python-logs-server \
		--file python/kubernetes/logs_server/Dockerfile python/kubernetes/logs_server

	docker build --platform linux/amd64 \
		-t ghcr.io/build-trust/ockam-python-logs-server:latest \
		-t ghcr.io/build-trust/ockam-python-logs-server:$(SHORT_COMMIT_HASH) --file python/kubernetes/logs_server/Dockerfile python/kubernetes/logs_server

release-logs-server-image-amd64:
	make build-logs-server-image-amd64
	$(eval SHORT_COMMIT_HASH := $(shell git rev-parse --short HEAD))
	docker push ghcr.io/build-trust/ockam-python-logs-server:$(SHORT_COMMIT_HASH)

.PHONY: \
	build build_release test lint clean very_clean format \
	release-linux build-image-amd64 release-image-amd64-hash \
	release-image-amd64-latest build-logs-server-image-amd64 \
	release-logs-server-image-amd64
