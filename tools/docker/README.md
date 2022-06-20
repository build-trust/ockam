
From the root directory of the ockam codebase:

## Builder

Build the builder:

```
docker build \
  --tag ockam-builder:latest \
  --tag ghcr.io/build-trust/ockam-builder:latest \
  tools/docker/builder
```

Run the builder:

```
docker run --rm -it -e HOST_USER_ID=$(id -u) --volume $(pwd):/work ockam-builder:latest bash
```

## Cloud node

```
docker build \
  --tag ockam-cloud-node:latest \
  --tag ghcr.io/build-trust/ockam-cloud-node:latest \
  --file tools/docker/cloud-node/Dockerfile .
```

Run the cloud node:

```
docker run --rm -it ockam-cloud-node:latest
```

## Ockam

```
docker build \
  --tag ockam:latest \
  --tag ghcr.io/build-trust/ockam:latest \
  --file tools/docker/ockam/Dockerfile .
```

Run Ockam binary:

```
docker run --rm -it ockam:latest --help
```

## Verifying Ockam Images
All Ockam images are signed by [cosign](https://github.com/sigstore/cosign), you can verify our images using the commands below with our [public key](https://github.com/build-trust/ockam/blob/main/tools/docker/cosign.pub)

```bash
$ cat cosign.pub

-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEt/XQMe16Vr/iIDr/ckKws8P3/x5W
lu6nc6jxKa/Ue5C6RI6xAbNlvzmpY/KjUU3Jie+3P9UG7TkkrsVRC7Zi0g==
-----END PUBLIC KEY-----

$ cosign verify --key cosign.pub $IMAGE_NAME
```
