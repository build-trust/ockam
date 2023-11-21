
This folder contains yaml files to build a distroless wolfi image, to build a distroless image, you need to create a secret key which will be used to sign packages, to create a secret key, in the wolfi directory, call the below command to create a keypair
```bash
docker run --rm -v "${PWD}":/work cgr.dev/chainguard/melange keygen
```

Note, it is crucial we call the above command before building our packages and images.

After generating a keypair, we can now build our elixir and erlang packages which will be used in our docker wolfi images, elixir package has the erlang package as a dependency, so we need to build the erlang package first, to build the erlang package, from the wolfi directory, call
```bash
docker run --rm --privileged -v "${PWD}":/work cgr.dev/chainguard/melange build erlang_package.yaml --arch amd64 -k melange.rsa.pub --signing-key melange.rsa
```

To build the elixir package
```bash
docker run --rm --privileged -v "${PWD}":/work cgr.dev/chainguard/melange build elixir_package.yaml --arch amd64 -k melange.rsa.pub --signing-key melange.rsa
```

After building the packages, we can now build our builder and base image, to build the builder image
```bash
docker run --rm -v ${PWD}:/work -w /work cgr.dev/chainguard/apko build builder_image.yaml -k melange.rsa.pub ghcr.io/build-trust/ockam-elixir-builder:latest builder_image.tar
```

the command above builds the builder image and sets the image name as `ghcr.io/build-trust/ockam-elixir-builder:latest` and creates a `.tar` file which can be loaded as a docker image with the below command
```bash
docker load < builder_image.tar
```

To build the base image
```bash
docker run --rm -v ${PWD}:/work -w /work cgr.dev/chainguard/apko build base_image.yaml -k melange.rsa.pub ghcr.io/build-trust/ockam-elixir-base:latest base_image.tar
```
to load the base image
```bash
docker load < base_image.tar
```
